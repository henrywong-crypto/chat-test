#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { BedrockRsStack } from '../lib/bedrock-rs-stack';

const app = new cdk.App();

new BedrockRsStack(app, 'BedrockRsStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region:  process.env.CDK_DEFAULT_REGION ?? 'us-east-1',
  },
  // Pass stack-level configuration via CDK context or environment variables.
  // Example: cdk deploy -c imageTag=abc123
  imageTag: app.node.tryGetContext('imageTag') ?? 'latest',
});
