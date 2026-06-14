use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::offline::offline_uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Offline,
    Msa,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Account {
    Offline {
        username: String,
        uuid: Uuid,

        session_token: String,
    },
    Msa {
        username: String,
        uuid: Uuid,
        access_token: String,
    },
}

impl Account {
    pub fn offline(username: impl Into<String>) -> Self {
        let username = username.into();
        let uuid = offline_uuid(&username);
        let session_token = Uuid::new_v4().as_simple().to_string();
        Self::Offline {
            username,
            uuid,
            session_token,
        }
    }

    pub fn kind(&self) -> AccountKind {
        match self {
            Self::Offline { .. } => AccountKind::Offline,
            Self::Msa { .. } => AccountKind::Msa,
        }
    }

    pub fn username(&self) -> &str {
        match self {
            Self::Offline { username, .. } | Self::Msa { username, .. } => username,
        }
    }

    pub fn uuid(&self) -> Uuid {
        match self {
            Self::Offline { uuid, .. } | Self::Msa { uuid, .. } => *uuid,
        }
    }

    pub fn access_token(&self) -> &str {
        match self {
            Self::Offline { session_token, .. } => session_token,
            Self::Msa { access_token, .. } => access_token,
        }
    }

    pub fn launch_user_type(&self) -> &'static str {
        match self {
            Self::Offline { .. } => "legacy",
            Self::Msa { .. } => "msa",
        }
    }
}
