import * as cdk          from 'aws-cdk-lib';
import * as dynamodb      from 'aws-cdk-lib/aws-dynamodb';
import * as ecr           from 'aws-cdk-lib/aws-ecr';
import * as ecs           from 'aws-cdk-lib/aws-ecs';
import * as ec2           from 'aws-cdk-lib/aws-ec2';
import * as elbv2         from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as iam           from 'aws-cdk-lib/aws-iam';
import * as cognito       from 'aws-cdk-lib/aws-cognito';
import * as s3            from 'aws-cdk-lib/aws-s3';
import * as logs          from 'aws-cdk-lib/aws-logs';
import { Construct }      from 'constructs';

// ─────────────────────────────────────────────────────────────────────────────

export interface BedrockRsStackProps extends cdk.StackProps {
  /** Docker image tag to deploy (e.g. a Git SHA). Default: "latest". */
  imageTag: string;
}

export class BedrockRsStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: BedrockRsStackProps) {
    super(scope, id, props);

    // ── Cognito ────────────────────────────────────────────────────────────────
    const userPool = new cognito.UserPool(this, 'UserPool', {
      userPoolName:         'bedrock-rs-users',
      selfSignUpEnabled:    false,           // admin-invite only
      signInAliases:        { email: true },
      autoVerify:           { email: true },
      passwordPolicy: {
        minLength:        12,
        requireLowercase: true,
        requireUppercase: true,
        requireDigits:    true,
        requireSymbols:   false,
      },
      accountRecovery: cognito.AccountRecovery.EMAIL_ONLY,
      removalPolicy:   cdk.RemovalPolicy.RETAIN,
    });

    // Groups used by the authorisation layer.
    new cognito.CfnUserPoolGroup(this, 'AdminGroup', {
      userPoolId:  userPool.userPoolId,
      groupName:   'Administrators',
      description: 'Full admin access',
    });
    new cognito.CfnUserPoolGroup(this, 'BotCreatorsGroup', {
      userPoolId:  userPool.userPoolId,
      groupName:   'CreatingBotAllowed',
    });
    new cognito.CfnUserPoolGroup(this, 'PublishersGroup', {
      userPoolId:  userPool.userPoolId,
      groupName:   'PublishAllowed',
    });

    const userPoolClient = new cognito.UserPoolClient(this, 'UserPoolClient', {
      userPool,
      userPoolClientName:   'bedrock-rs-web',
      authFlows: {
        userPassword:        true,
        userSrp:             true,
      },
      generateSecret:        false,
      accessTokenValidity:   cdk.Duration.hours(8),
      idTokenValidity:       cdk.Duration.hours(8),
      refreshTokenValidity:  cdk.Duration.days(30),
    });

    // ── DynamoDB ──────────────────────────────────────────────────────────────

    // Conversations table:  userId (PK) + conversationId (SK)
    const conversationsTable = new dynamodb.Table(this, 'ConversationsTable', {
      tableName:     'bedrock-rs-conversations',
      partitionKey:  { name: 'userId',         type: dynamodb.AttributeType.STRING },
      sortKey:       { name: 'conversationId', type: dynamodb.AttributeType.STRING },
      billingMode:   dynamodb.BillingMode.PAY_PER_REQUEST,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecovery: true,
    });

    // Bots table:  userId (PK) + botId (SK)
    //   GSI: visibility-index  →  visibility (PK) + createTime (SK)
    const botsTable = new dynamodb.Table(this, 'BotsTable', {
      tableName:     'bedrock-rs-bots',
      partitionKey:  { name: 'userId', type: dynamodb.AttributeType.STRING },
      sortKey:       { name: 'botId',  type: dynamodb.AttributeType.STRING },
      billingMode:   dynamodb.BillingMode.PAY_PER_REQUEST,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecovery: true,
    });
    botsTable.addGlobalSecondaryIndex({
      indexName:     'visibility-index',
      partitionKey:  { name: 'visibility', type: dynamodb.AttributeType.STRING },
      sortKey:       { name: 'createTime', type: dynamodb.AttributeType.NUMBER },
      projectionType: dynamodb.ProjectionType.ALL,
    });

    // Inference profiles table:  userId (PK) + modelId (SK)
    const inferenceProfilesTable = new dynamodb.Table(this, 'InferenceProfilesTable', {
      tableName:     'bedrock-rs-inference-profiles',
      partitionKey:  { name: 'userId',  type: dynamodb.AttributeType.STRING },
      sortKey:       { name: 'modelId', type: dynamodb.AttributeType.STRING },
      billingMode:   dynamodb.BillingMode.PAY_PER_REQUEST,
      removalPolicy: cdk.RemovalPolicy.RETAIN,
      pointInTimeRecovery: true,
    });

    // ── S3 bucket (large message offload) ─────────────────────────────────────
    const largeMessageBucket = new s3.Bucket(this, 'LargeMessageBucket', {
      bucketName:      `bedrock-rs-messages-${this.account}-${this.region}`,
      versioned:       false,
      encryption:      s3.BucketEncryption.S3_MANAGED,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      lifecycleRules: [{
        // Automatically expire large message blobs after 365 days.
        expiration: cdk.Duration.days(365),
      }],
      removalPolicy:   cdk.RemovalPolicy.RETAIN,
    });

    // ── ECR repository ────────────────────────────────────────────────────────
    const repository = new ecr.Repository(this, 'AppRepository', {
      repositoryName: 'bedrock-rs',
      removalPolicy:  cdk.RemovalPolicy.RETAIN,
      lifecycleRules: [{
        // Keep only the last 20 images; purge older ones automatically.
        maxImageCount: 20,
      }],
    });

    // ── VPC ───────────────────────────────────────────────────────────────────
    const vpc = new ec2.Vpc(this, 'Vpc', {
      maxAzs:     2,
      natGateways: 1,   // cost optimisation; use 2 for HA
      subnetConfiguration: [
        { cidrMask: 24, name: 'public',  subnetType: ec2.SubnetType.PUBLIC },
        { cidrMask: 24, name: 'private', subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
      ],
    });

    // ── IAM task role ─────────────────────────────────────────────────────────
    const taskRole = new iam.Role(this, 'TaskRole', {
      roleName:   'bedrock-rs-task-role',
      assumedBy:  new iam.ServicePrincipal('ecs-tasks.amazonaws.com'),
      description: 'Runtime role for bedrock-rs ECS tasks',
    });

    // Bedrock permissions
    taskRole.addToPolicy(new iam.PolicyStatement({
      sid:     'BedrockInference',
      effect:  iam.Effect.ALLOW,
      actions: [
        'bedrock:InvokeModel',
        'bedrock:InvokeModelWithResponseStream',
      ],
      resources: ['arn:aws:bedrock:*::foundation-model/*'],
    }));

    taskRole.addToPolicy(new iam.PolicyStatement({
      sid:     'BedrockInferenceProfiles',
      effect:  iam.Effect.ALLOW,
      actions: [
        'bedrock:CreateInferenceProfile',
        'bedrock:GetInferenceProfile',
        'bedrock:ListInferenceProfiles',
        'bedrock:DeleteInferenceProfile',
        'bedrock:TagResource',
        'bedrock:UntagResource',
        'bedrock:ListTagsForResource',
      ],
      resources: [
        `arn:aws:bedrock:*:${this.account}:inference-profile/*`,
        'arn:aws:bedrock:*::foundation-model/*',
      ],
    }));

    taskRole.addToPolicy(new iam.PolicyStatement({
      sid:     'BedrockListModels',
      effect:  iam.Effect.ALLOW,
      actions: [
        'bedrock:ListFoundationModels',
        'bedrock:GetFoundationModel',
        'bedrock:ListCrossRegionInferenceProfiles',
      ],
      resources: ['*'],
    }));

    // DynamoDB permissions
    conversationsTable.grantReadWriteData(taskRole);
    botsTable.grantReadWriteData(taskRole);
    inferenceProfilesTable.grantReadWriteData(taskRole);

    // S3 permissions (large message bucket)
    largeMessageBucket.grantReadWrite(taskRole);

    // Cognito permissions (admin user management)
    taskRole.addToPolicy(new iam.PolicyStatement({
      sid:     'CognitoUserManagement',
      effect:  iam.Effect.ALLOW,
      actions: [
        'cognito-idp:ListUsers',
        'cognito-idp:AdminGetUser',
        'cognito-idp:AdminListGroupsForUser',
        'cognito-idp:AdminAddUserToGroup',
        'cognito-idp:AdminRemoveUserFromGroup',
        'cognito-idp:AdminEnableUser',
        'cognito-idp:AdminDisableUser',
      ],
      resources: [userPool.userPoolArn],
    }));

    // ── CloudWatch log group ───────────────────────────────────────────────────
    const logGroup = new logs.LogGroup(this, 'AppLogGroup', {
      logGroupName:  '/ecs/bedrock-rs',
      retention:     logs.RetentionDays.ONE_MONTH,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // ── ECS cluster ───────────────────────────────────────────────────────────
    const cluster = new ecs.Cluster(this, 'Cluster', {
      clusterName:         'bedrock-rs',
      vpc,
      containerInsights:   true,
    });

    // ── Task definition ───────────────────────────────────────────────────────
    const taskDefinition = new ecs.FargateTaskDefinition(this, 'TaskDef', {
      family:        'bedrock-rs',
      cpu:           512,    // 0.5 vCPU; bump to 1024 for heavier load
      memoryLimitMiB: 1024,
      taskRole,
    });

    const container = taskDefinition.addContainer('app', {
      image:     ecs.ContainerImage.fromEcrRepository(repository, props.imageTag),
      logging:   ecs.LogDrivers.awsLogs({
        streamPrefix: 'bedrock-rs',
        logGroup,
      }),
      environment: {
        SITE_ADDR:                        '0.0.0.0:3000',
        AWS_REGION:                       this.region,
        COGNITO_USER_POOL_ID:             userPool.userPoolId,
        COGNITO_CLIENT_ID:                userPoolClient.userPoolClientId,
        CONVERSATIONS_TABLE_NAME:         conversationsTable.tableName,
        BOTS_TABLE_NAME:                  botsTable.tableName,
        INFERENCE_PROFILES_TABLE_NAME:    inferenceProfilesTable.tableName,
        LARGE_MESSAGE_BUCKET:             largeMessageBucket.bucketName,
        RUST_LOG:                         'info,app=info',
      },
      healthCheck: {
        command:     ['CMD-SHELL', 'curl -sf http://localhost:3000/api/models || exit 1'],
        interval:    cdk.Duration.seconds(30),
        timeout:     cdk.Duration.seconds(5),
        retries:     3,
        startPeriod: cdk.Duration.seconds(30),
      },
    });

    container.addPortMappings({ containerPort: 3000 });

    // ── Security groups ───────────────────────────────────────────────────────
    const albSg = new ec2.SecurityGroup(this, 'AlbSg', {
      vpc,
      description: 'ALB security group',
    });
    albSg.addIngressRule(ec2.Peer.anyIpv4(), ec2.Port.tcp(80),  'HTTP from internet');
    albSg.addIngressRule(ec2.Peer.anyIpv4(), ec2.Port.tcp(443), 'HTTPS from internet');

    const appSg = new ec2.SecurityGroup(this, 'AppSg', {
      vpc,
      description: 'App task security group',
    });
    appSg.addIngressRule(albSg, ec2.Port.tcp(3000), 'Traffic from ALB');

    // ── Application Load Balancer ──────────────────────────────────────────────
    const alb = new elbv2.ApplicationLoadBalancer(this, 'Alb', {
      loadBalancerName: 'bedrock-rs',
      vpc,
      internetFacing:   true,
      securityGroup:    albSg,
      vpcSubnets:       { subnetType: ec2.SubnetType.PUBLIC },
    });

    // HTTP listener (redirects to HTTPS in production; serves directly in dev)
    const httpListener = alb.addListener('HttpListener', {
      port: 80,
      // To enable HTTPS, replace the open action with an HTTPS redirect:
      // defaultAction: elbv2.ListenerAction.redirect({ port: '443', protocol: 'HTTPS' })
      open: true,
    });

    // ── ECS Fargate service ────────────────────────────────────────────────────
    const service = new ecs.FargateService(this, 'Service', {
      serviceName:          'bedrock-rs',
      cluster,
      taskDefinition,
      desiredCount:         2,    // two tasks for HA (SSE requires sticky sessions if count > 1)
      securityGroups:       [appSg],
      vpcSubnets:           { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
      assignPublicIp:       false,
      enableExecuteCommand: true,   // allows `ecs execute-command` for debugging
    });

    // Register the service with the ALB target group.
    // Enable sticky sessions so SSE streaming connections stay on the same task.
    const targetGroup = httpListener.addTargets('AppTargets', {
      targetGroupName: 'bedrock-rs-tg',
      port:            3000,
      protocol:        elbv2.ApplicationProtocol.HTTP,
      targets:         [service],
      healthCheck: {
        path:                    '/api/models',
        healthyHttpCodes:        '200',
        interval:                cdk.Duration.seconds(30),
        healthyThresholdCount:   2,
        unhealthyThresholdCount: 3,
      },
      stickinessCookieDuration: cdk.Duration.hours(1),
    });

    // Auto-scaling policy (optional, uncomment to enable)
    // const scaling = service.autoScaleTaskCount({ minCapacity: 2, maxCapacity: 8 });
    // scaling.scaleOnCpuUtilization('CpuScaling', {
    //   targetUtilizationPercent: 60,
    //   scaleInCooldown:  cdk.Duration.seconds(60),
    //   scaleOutCooldown: cdk.Duration.seconds(30),
    // });

    // ── Outputs ───────────────────────────────────────────────────────────────
    new cdk.CfnOutput(this, 'AlbDnsName', {
      value:       alb.loadBalancerDnsName,
      description: 'Application Load Balancer DNS name',
      exportName:  'BedrockRsAlbDns',
    });

    new cdk.CfnOutput(this, 'EcrRepositoryUri', {
      value:       repository.repositoryUri,
      description: 'ECR repository URI — push your Docker image here',
      exportName:  'BedrockRsEcrUri',
    });

    new cdk.CfnOutput(this, 'UserPoolId', {
      value:       userPool.userPoolId,
      description: 'Cognito User Pool ID (set as COGNITO_USER_POOL_ID env var)',
      exportName:  'BedrockRsUserPoolId',
    });

    new cdk.CfnOutput(this, 'UserPoolClientId', {
      value:       userPoolClient.userPoolClientId,
      description: 'Cognito User Pool Client ID (set as COGNITO_CLIENT_ID env var)',
      exportName:  'BedrockRsUserPoolClientId',
    });

    new cdk.CfnOutput(this, 'LargeMessageBucketName', {
      value:       largeMessageBucket.bucketName,
      description: 'S3 bucket for large message offload',
      exportName:  'BedrockRsMessageBucket',
    });
  }
}
