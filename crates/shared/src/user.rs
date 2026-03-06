use serde::{Deserialize, Serialize};

// ── UserGroup ─────────────────────────────────────────────────────────────────

/// Mirrors Cognito group names.  Admin implies all other permissions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserGroup {
    Admin,
    CreatingBotAllowed,
    PublishAllowed,
    /// Catch-all for any unknown groups returned by Cognito.
    #[serde(other)]
    Standard,
}

impl UserGroup {
    /// Parse a Cognito group name string into a `UserGroup`.
    pub fn from_cognito_name(name: &str) -> Self {
        match name {
            "Admin"               => Self::Admin,
            "CreatingBotAllowed"  => Self::CreatingBotAllowed,
            "PublishAllowed"      => Self::PublishAllowed,
            _                     => Self::Standard,
        }
    }

    /// Return the Cognito group name string.
    pub fn as_cognito_name(&self) -> &'static str {
        match self {
            Self::Admin              => "Admin",
            Self::CreatingBotAllowed => "CreatingBotAllowed",
            Self::PublishAllowed     => "PublishAllowed",
            Self::Standard           => "Standard",
        }
    }
}

// ── User ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Cognito `sub` claim — stable unique identifier.
    pub id: String,
    pub email: String,
    pub groups: Vec<UserGroup>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.groups.contains(&UserGroup::Admin)
    }

    /// Admins can always create bots regardless of the explicit group.
    pub fn can_create_bot(&self) -> bool {
        self.is_admin() || self.groups.contains(&UserGroup::CreatingBotAllowed)
    }

    /// Admins can always publish.
    pub fn can_publish(&self) -> bool {
        self.is_admin() || self.groups.contains(&UserGroup::PublishAllowed)
    }
}
