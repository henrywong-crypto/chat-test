/// Static Cognito pool configuration shared across the auth sub-modules.

#[derive(Debug, Clone)]
pub struct CognitoConfig {
    pub user_pool_id: String,
    /// AWS region hosting the user pool (e.g. `"us-east-1"`).
    pub region: String,
}

impl CognitoConfig {
    pub fn new(user_pool_id: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            user_pool_id: user_pool_id.into(),
            region: region.into(),
        }
    }

    /// `https://cognito-idp.{region}.amazonaws.com/{pool_id}`
    pub fn issuer_url(&self) -> String {
        format!(
            "https://cognito-idp.{}.amazonaws.com/{}",
            self.region, self.user_pool_id
        )
    }

    /// `{issuer_url}/.well-known/jwks.json`
    pub fn jwks_url(&self) -> String {
        format!("{}/.well-known/jwks.json", self.issuer_url())
    }
}
