use crate::{AppError, AppState, extractors::Protobuf, pb::AnalyticsEvent};
use axum::{
    extract::State,
    http::{StatusCode, request::Parts},
    response::IntoResponse,
};
use chat_core::User;
use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Row, Serialize, Deserialize)]
pub struct AnalyticsEventRow {
    // EventContext fields
    pub client_id: String,
    pub app_version: String,
    pub system_os: String,
    pub system_arch: String,
    pub system_locale: String,
    pub system_timezone: String,
    pub user_id: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub geo_country: Option<String>,
    pub geo_region: Option<String>,
    pub geo_city: Option<String>,
    pub client_ts: i64,
    pub server_ts: i64,

    // Event type
    pub event_type: EventTypeRow,

    // AppExitEvent specific fields
    pub exit_code: Option<ExitCodeRow>,

    // User auth events fields
    pub email: Option<String>,
    pub workspace_id: Option<String>,

    // Chat events fields
    pub chat_id: Option<String>,

    // MessageSentEvent specific fields
    pub message_type: Option<String>,
    pub message_size: Option<i32>,
    pub total_files: Option<i32>,

    // NavigationEvent specific fields
    pub navigation_from: Option<String>,
    pub navigation_to: Option<String>,

    // Timestamp for when the record was inserted
    pub inserted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventTypeRow {
    AppStart = 8,
    AppExit = 9,
    UserLogin = 10,
    UserLogout = 11,
    UserRegister = 12,
    ChatCreated = 13,
    MessageSent = 14,
    ChatJoined = 15,
    ChatLeft = 16,
    Navigation = 17,
    #[default]
    Unspecified,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitCodeRow {
    #[default]
    Unspecified = 0,
    Success = 1,
    Failure = 2,
}

pub(crate) async fn create_event_handler(
    parts: Parts,
    State(state): State<AppState>,
    Protobuf(event): Protobuf<AnalyticsEvent>,
) -> Result<impl IntoResponse, AppError> {
    let mut row = AnalyticsEventRow::try_from(event)?;
    // get user info from extension
    if let Some(user) = parts.extensions.get::<User>() {
        row.user_id = Some(user.id.to_string());
    } else {
        row.user_id = None;
    }
    let mut insert = state.client.insert("analytics_events")?;
    insert.write(&row).await?;
    insert.end().await?;
    Ok(StatusCode::CREATED)
}
