use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub channel: Channel,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: DeliveryStatus,
    pub template_id: Option<String>,
    pub sent_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Channel {
    Email,
    Sms,
    Push,
    InApp,
    Webhook,
    Slack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Queued,
    Sent,
    Delivered,
    Failed,
    Bounced,
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
