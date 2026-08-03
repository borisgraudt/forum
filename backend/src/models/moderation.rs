use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Report {
    pub id: i64,
    pub reporter_id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub reason: String,
    pub details: Option<String>,
    pub status: String,
    pub resolved_by: Option<i64>,
    pub resolved_at: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ReportView {
    pub id: i64,
    pub reporter_id: i64,
    pub reporter_username: String,
    pub target_type: String,
    pub target_id: i64,
    pub reason: String,
    pub details: Option<String>,
    pub status: String,
    pub resolved_by: Option<i64>,
    pub resolved_at: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserSanction {
    pub id: i64,
    pub user_id: i64,
    pub kind: String,
    pub reason: Option<String>,
    pub created_by: i64,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserSanctionView {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub kind: String,
    pub reason: Option<String>,
    pub created_by: i64,
    pub created_by_username: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub actor_id: Option<i64>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub meta: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanctionKind {
    Ban,
    Mute,
    Timeout,
}

impl SanctionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ban => "ban",
            Self::Mute => "mute",
            Self::Timeout => "timeout",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ban" => Some(Self::Ban),
            "mute" => Some(Self::Mute),
            "timeout" => Some(Self::Timeout),
            _ => None,
        }
    }
}
