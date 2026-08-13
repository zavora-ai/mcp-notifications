use crate::domain::*;
use chrono::Utc;
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyInput {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IdInput { pub id: String }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SendInput {
    pub channel: String,  // "email", "sms", "push", "in_app", "webhook", "slack"
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SendTemplateInput {
    pub template_id: String,
    pub recipient: String,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BroadcastInput {
    pub channel: String,
    pub recipients: Vec<String>,
    pub subject: Option<String>,
    pub body: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTemplateInput {
    pub name: String,
    pub channel: String,
    pub subject: Option<String>,
    pub body_template: String,
    pub variables: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PreferenceInput {
    pub user_id: String,
    pub email_enabled: Option<bool>,
    pub sms_enabled: Option<bool>,
    pub push_enabled: Option<bool>,
    pub in_app_enabled: Option<bool>,
    pub quiet_hours: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChannelFilterInput { pub channel: Option<String> }

fn parse_channel(s: &str) -> Channel {
    match s { "sms" => Channel::Sms, "push" => Channel::Push, "in_app" => Channel::InApp, "webhook" => Channel::Webhook, "slack" => Channel::Slack, _ => Channel::Email }
}

#[derive(Clone)]
pub struct NotificationServer {
    pub notifications: Arc<RwLock<Vec<Notification>>>,
    pub templates: Arc<RwLock<Vec<Template>>>,
    pub preferences: Arc<RwLock<Vec<Preference>>>,
}

impl NotificationServer {
    pub fn seeded() -> Self {
        let templates = vec![
            Template { id: "tpl-welcome".into(), name: "Welcome Email".into(), channel: Channel::Email, subject: Some("Welcome to {{company}}!".into()), body_template: "Hi {{name}}, welcome to {{company}}. Get started at {{url}}.".into(), variables: vec!["name".into(), "company".into(), "url".into()] },
            Template { id: "tpl-otp".into(), name: "OTP SMS".into(), channel: Channel::Sms, subject: None, body_template: "Your verification code is {{code}}. Expires in 5 minutes.".into(), variables: vec!["code".into()] },
            Template { id: "tpl-alert".into(), name: "System Alert".into(), channel: Channel::Push, subject: Some("Alert: {{title}}".into()), body_template: "{{message}}".into(), variables: vec!["title".into(), "message".into()] },
        ];
        let preferences = vec![
            Preference { user_id: "user-1".into(), email_enabled: true, sms_enabled: true, push_enabled: true, in_app_enabled: true, quiet_hours: Some("22:00-07:00".into()) },
            Preference { user_id: "user-2".into(), email_enabled: true, sms_enabled: false, push_enabled: true, in_app_enabled: true, quiet_hours: None },
        ];
        Self {
            notifications: Arc::new(RwLock::new(Vec::new())),
            templates: Arc::new(RwLock::new(templates)),
            preferences: Arc::new(RwLock::new(preferences)),
        }
    }
}

#[tool_router]
impl NotificationServer {
    #[tool(description = "Send a notification via any channel (email, sms, push, in_app, webhook, slack)")]
    async fn send_notification(&self, Parameters(input): Parameters<SendInput>) -> String {
        let id = format!("notif-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let notif = Notification { id: id.clone(), channel: parse_channel(&input.channel), recipient: input.recipient.clone(), subject: input.subject, body: input.body, status: DeliveryStatus::Sent, template_id: None, sent_at: Utc::now().to_rfc3339() };
        self.notifications.write().await.push(notif);
        format!("Sent {} notification to {} (id: {})", input.channel, input.recipient, id)
    }

    #[tool(description = "Send a notification using a template with variable substitution")]
    async fn send_from_template(&self, Parameters(input): Parameters<SendTemplateInput>) -> String {
        let templates = self.templates.read().await;
        let tpl = match templates.iter().find(|t| t.id == input.template_id) {
            Some(t) => t.clone(),
            None => return format!("Template {} not found", input.template_id),
        };
        let vars = input.variables.unwrap_or(serde_json::json!({}));
        let mut body = tpl.body_template.clone();
        let mut subject = tpl.subject.clone().unwrap_or_default();
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj { let val = v.as_str().unwrap_or(""); body = body.replace(&format!("{{{{{}}}}}", k), val); subject = subject.replace(&format!("{{{{{}}}}}", k), val); }
        }
        let id = format!("notif-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let notif = Notification { id: id.clone(), channel: tpl.channel, recipient: input.recipient.clone(), subject: Some(subject), body, status: DeliveryStatus::Sent, template_id: Some(input.template_id), sent_at: Utc::now().to_rfc3339() };
        self.notifications.write().await.push(notif);
        format!("Sent template notification to {} (id: {})", input.recipient, id)
    }

    #[tool(description = "Broadcast a notification to multiple recipients")]
    async fn broadcast(&self, Parameters(input): Parameters<BroadcastInput>) -> String {
        let count = input.recipients.len();
        let mut notifs = self.notifications.write().await;
        for r in &input.recipients {
            let id = format!("notif-{}", &uuid::Uuid::new_v4().to_string()[..8]);
            notifs.push(Notification { id, channel: parse_channel(&input.channel), recipient: r.clone(), subject: input.subject.clone(), body: input.body.clone(), status: DeliveryStatus::Sent, template_id: None, sent_at: Utc::now().to_rfc3339() });
        }
        format!("Broadcast {} notification to {} recipients", input.channel, count)
    }

    #[tool(description = "List sent notifications, optionally filtered by channel")]
    async fn list_notifications(&self, Parameters(input): Parameters<ChannelFilterInput>) -> String {
        let notifs = self.notifications.read().await;
        let filtered: Vec<serde_json::Value> = notifs.iter()
            .filter(|n| input.channel.as_ref().map_or(true, |c| format!("{:?}", n.channel).to_lowercase() == *c))
            .map(|n| serde_json::json!({"id": n.id, "channel": format!("{:?}", n.channel), "recipient": n.recipient, "status": format!("{:?}", n.status), "sent_at": n.sent_at}))
            .collect();
        serde_json::to_string_pretty(&filtered).unwrap()
    }

    #[tool(description = "Get notification delivery status")]
    async fn get_status(&self, Parameters(input): Parameters<IdInput>) -> String {
        let notifs = self.notifications.read().await;
        match notifs.iter().find(|n| n.id == input.id) {
            Some(n) => serde_json::to_string_pretty(n).unwrap(),
            None => format!("Notification {} not found", input.id),
        }
    }

    #[tool(description = "List notification templates")]
    async fn list_templates(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let templates = self.templates.read().await;
        serde_json::to_string_pretty(&*templates).unwrap()
    }

    #[tool(description = "Get template details")]
    async fn get_template(&self, Parameters(input): Parameters<IdInput>) -> String {
        let templates = self.templates.read().await;
        match templates.iter().find(|t| t.id == input.id) {
            Some(t) => serde_json::to_string_pretty(t).unwrap(),
            None => format!("Template {} not found", input.id),
        }
    }

    #[tool(description = "Create a notification template")]
    async fn create_template(&self, Parameters(input): Parameters<CreateTemplateInput>) -> String {
        let id = format!("tpl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let tpl = Template { id: id.clone(), name: input.name.clone(), channel: parse_channel(&input.channel), subject: input.subject, body_template: input.body_template, variables: input.variables };
        self.templates.write().await.push(tpl);
        format!("Created template '{}' (id: {})", input.name, id)
    }

    #[tool(description = "Get user notification preferences")]
    async fn get_preferences(&self, Parameters(input): Parameters<IdInput>) -> String {
        let prefs = self.preferences.read().await;
        match prefs.iter().find(|p| p.user_id == input.id) {
            Some(p) => serde_json::to_string_pretty(p).unwrap(),
            None => format!("No preferences for user {}", input.id),
        }
    }

    #[tool(description = "Update user notification preferences")]
    async fn update_preferences(&self, Parameters(input): Parameters<PreferenceInput>) -> String {
        let mut prefs = self.preferences.write().await;
        match prefs.iter_mut().find(|p| p.user_id == input.user_id) {
            Some(p) => {
                if let Some(v) = input.email_enabled { p.email_enabled = v; }
                if let Some(v) = input.sms_enabled { p.sms_enabled = v; }
                if let Some(v) = input.push_enabled { p.push_enabled = v; }
                if let Some(v) = input.in_app_enabled { p.in_app_enabled = v; }
                if let Some(v) = input.quiet_hours { p.quiet_hours = Some(v); }
                format!("Updated preferences for {}", input.user_id)
            }
            None => {
                prefs.push(Preference { user_id: input.user_id.clone(), email_enabled: input.email_enabled.unwrap_or(true), sms_enabled: input.sms_enabled.unwrap_or(true), push_enabled: input.push_enabled.unwrap_or(true), in_app_enabled: input.in_app_enabled.unwrap_or(true), quiet_hours: input.quiet_hours });
                format!("Created preferences for {}", input.user_id)
            }
        }
    }

    #[tool(description = "Get delivery stats: sent, delivered, failed, bounced counts")]
    async fn get_delivery_stats(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let notifs = self.notifications.read().await;
        let sent = notifs.iter().filter(|n| matches!(n.status, DeliveryStatus::Sent)).count();
        let delivered = notifs.iter().filter(|n| matches!(n.status, DeliveryStatus::Delivered)).count();
        let failed = notifs.iter().filter(|n| matches!(n.status, DeliveryStatus::Failed)).count();
        let bounced = notifs.iter().filter(|n| matches!(n.status, DeliveryStatus::Bounced)).count();
        serde_json::to_string_pretty(&serde_json::json!({"total": notifs.len(), "sent": sent, "delivered": delivered, "failed": failed, "bounced": bounced})).unwrap()
    }

    #[tool(description = "Schedule a notification for future delivery")]
    async fn schedule_notification(&self, Parameters(input): Parameters<SendInput>) -> String {
        let id = format!("notif-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let notif = Notification { id: id.clone(), channel: parse_channel(&input.channel), recipient: input.recipient.clone(), subject: input.subject, body: input.body, status: DeliveryStatus::Queued, template_id: None, sent_at: Utc::now().to_rfc3339() };
        self.notifications.write().await.push(notif);
        format!("Scheduled {} notification to {} (id: {})", input.channel, input.recipient, id)
    }

    #[tool(description = "Retry a failed notification")]
    async fn retry_notification(&self, Parameters(input): Parameters<IdInput>) -> String {
        let mut notifs = self.notifications.write().await;
        match notifs.iter_mut().find(|n| n.id == input.id) {
            Some(n) => {
                if matches!(n.status, DeliveryStatus::Failed | DeliveryStatus::Bounced) {
                    n.status = DeliveryStatus::Sent;
                    format!("Retried notification {}", input.id)
                } else {
                    format!("Notification {} is not in failed state", input.id)
                }
            }
            None => format!("Notification {} not found", input.id),
        }
    }
}

adk_mcp_sdk::mcp_2026_server! {
    server: NotificationServer,
    task_tools: [],
    approval_tools: [],
    cache_ttl_ms: 60_000,
}
