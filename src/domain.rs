use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub channel: Channel,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: DeliveryStatus,
    pub delivery_path: String,
    pub status_detail: String,
    pub template_id: Option<String>,
    pub attempted_at: Option<String>,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Email,
    Sms,
    Push,
    InApp,
    Webhook,
    Slack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Queued,
    Delivered,
    Suppressed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub channel: Channel,
    pub subject: Option<String>,
    pub body_template: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    pub user_id: String,
    pub email_enabled: bool,
    pub sms_enabled: bool,
    pub push_enabled: bool,
    pub in_app_enabled: bool,
    pub quiet_hours: Option<String>,
}
