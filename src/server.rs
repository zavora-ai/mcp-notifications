use crate::domain::*;
use chrono::{Local, Timelike, Utc};
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use url::Url;

fn error_result(error: impl std::fmt::Display) -> String {
    serde_json::json!({"ok": false, "error": error.to_string()}).to_string()
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyInput {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IdInput {
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SendInput {
    pub channel: String, // "email", "sms", "push", "in_app", "webhook", "slack"
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
pub struct ChannelFilterInput {
    pub channel: Option<String>,
}

fn parse_channel(s: &str) -> Result<Channel, String> {
    match s {
        "email" => Ok(Channel::Email),
        "sms" => Ok(Channel::Sms),
        "push" => Ok(Channel::Push),
        "in_app" => Ok(Channel::InApp),
        "webhook" => Ok(Channel::Webhook),
        "slack" => Ok(Channel::Slack),
        _ => Err(format!("unsupported channel: {s}")),
    }
}

fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::Email => "email",
        Channel::Sms => "sms",
        Channel::Push => "push",
        Channel::InApp => "in_app",
        Channel::Webhook => "webhook",
        Channel::Slack => "slack",
    }
}

fn safe_recipient(recipient: &str) -> String {
    Url::parse(recipient).map_or_else(
        |_| recipient.to_string(),
        |url| {
            let host = url.host_str().unwrap_or("redacted");
            let authority = url
                .port()
                .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
            format!("{}://{authority}/…", url.scheme())
        },
    )
}

fn notification_view(notification: &Notification) -> serde_json::Value {
    serde_json::json!({
        "id": notification.id,
        "channel": notification.channel,
        "recipient": safe_recipient(&notification.recipient),
        "subject": notification.subject,
        "body": notification.body,
        "status": notification.status,
        "delivery_path": notification.delivery_path,
        "status_detail": notification.status_detail,
        "template_id": notification.template_id,
        "attempted_at": notification.attempted_at,
        "delivered_at": notification.delivered_at,
    })
}

fn configured_endpoint(channel: Channel, recipient: &str) -> Result<Option<(Url, String)>, String> {
    if channel == Channel::InApp {
        return Ok(None);
    }
    let (candidate, path) = match channel {
        Channel::Webhook => (recipient.to_string(), "recipient_webhook".to_string()),
        Channel::Slack if recipient.starts_with("https://") => {
            (recipient.to_string(), "recipient_slack_webhook".to_string())
        }
        _ => {
            let key = format!(
                "MCP_NOTIFICATIONS_{}_ENDPOINT",
                channel_name(channel).to_uppercase()
            );
            let endpoint = std::env::var(&key).map_err(|_| {
                format!("no delivery path configured; set {key} or use channel=webhook with an HTTPS recipient")
            })?;
            (
                endpoint,
                format!("configured_{}_gateway", channel_name(channel)),
            )
        }
    };
    let url =
        Url::parse(&candidate).map_err(|error| format!("invalid delivery endpoint: {error}"))?;
    if url.scheme() != "https" && !(cfg!(debug_assertions) && url.scheme() == "http") {
        return Err(
            "delivery endpoints must use HTTPS (HTTP is accepted only in debug builds)".into(),
        );
    }
    Ok(Some((url, path)))
}

struct DeliveryOutcome {
    status: DeliveryStatus,
    path: String,
    detail: String,
    attempted_at: Option<String>,
    delivered_at: Option<String>,
}

#[derive(Clone)]
pub struct NotificationServer {
    pub notifications: Arc<RwLock<Vec<Notification>>>,
    pub templates: Arc<RwLock<Vec<Template>>>,
    pub preferences: Arc<RwLock<Vec<Preference>>>,
    client: reqwest::Client,
}

impl NotificationServer {
    pub fn seeded() -> Self {
        let templates = vec![
            Template {
                id: "tpl-welcome".into(),
                name: "Welcome Email".into(),
                channel: Channel::Email,
                subject: Some("Welcome to {{company}}!".into()),
                body_template: "Hi {{name}}, welcome to {{company}}. Get started at {{url}}."
                    .into(),
                variables: vec!["name".into(), "company".into(), "url".into()],
            },
            Template {
                id: "tpl-otp".into(),
                name: "OTP SMS".into(),
                channel: Channel::Sms,
                subject: None,
                body_template: "Your verification code is {{code}}. Expires in 5 minutes.".into(),
                variables: vec!["code".into()],
            },
            Template {
                id: "tpl-alert".into(),
                name: "System Alert".into(),
                channel: Channel::Push,
                subject: Some("Alert: {{title}}".into()),
                body_template: "{{message}}".into(),
                variables: vec!["title".into(), "message".into()],
            },
        ];
        let preferences = vec![
            Preference {
                user_id: "user-1".into(),
                email_enabled: true,
                sms_enabled: true,
                push_enabled: true,
                in_app_enabled: true,
                quiet_hours: Some("22:00-07:00".into()),
            },
            Preference {
                user_id: "user-2".into(),
                email_enabled: true,
                sms_enabled: false,
                push_enabled: true,
                in_app_enabled: true,
                quiet_hours: None,
            },
        ];
        Self {
            notifications: Arc::new(RwLock::new(Vec::new())),
            templates: Arc::new(RwLock::new(templates)),
            preferences: Arc::new(RwLock::new(preferences)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("the HTTPS delivery client configuration is valid"),
        }
    }

    async fn deliver(
        &self,
        channel: Channel,
        recipient: &str,
        subject: Option<&str>,
        body: &str,
    ) -> DeliveryOutcome {
        if let Some(detail) = self.suppression_reason(channel, recipient).await {
            return DeliveryOutcome {
                status: DeliveryStatus::Suppressed,
                path: "preference_policy".into(),
                detail,
                attempted_at: None,
                delivered_at: None,
            };
        }
        if channel == Channel::InApp {
            return DeliveryOutcome {
                status: DeliveryStatus::Queued,
                path: "local_in_app_queue".into(),
                detail: "queued locally; no external delivery was attempted".into(),
                attempted_at: None,
                delivered_at: None,
            };
        }

        let attempted_at = Utc::now().to_rfc3339();
        let (endpoint, path) = match configured_endpoint(channel, recipient) {
            Ok(Some(endpoint)) => endpoint,
            Ok(None) => unreachable!("in-app delivery returned above"),
            Err(detail) => {
                return DeliveryOutcome {
                    status: DeliveryStatus::Failed,
                    path: "unconfigured".into(),
                    detail,
                    attempted_at: Some(attempted_at),
                    delivered_at: None,
                };
            }
        };

        let payload = if channel == Channel::Slack {
            serde_json::json!({"text": subject.map_or_else(|| body.to_string(), |title| format!("*{title}*\n{body}"))})
        } else {
            serde_json::json!({
                "channel": channel_name(channel),
                "recipient": recipient,
                "subject": subject,
                "body": body,
            })
        };
        let mut request = self.client.post(endpoint).json(&payload);
        if let Ok(token) = std::env::var("MCP_NOTIFICATIONS_BEARER_TOKEN") {
            request = request.bearer_auth(token);
        }
        match request.send().await {
            Ok(response) if response.status().is_success() => DeliveryOutcome {
                status: DeliveryStatus::Delivered,
                path,
                detail: format!(
                    "delivery endpoint accepted the request with HTTP {}",
                    response.status()
                ),
                attempted_at: Some(attempted_at),
                delivered_at: Some(Utc::now().to_rfc3339()),
            },
            Ok(response) => DeliveryOutcome {
                status: DeliveryStatus::Failed,
                path,
                detail: format!("delivery endpoint returned HTTP {}", response.status()),
                attempted_at: Some(attempted_at),
                delivered_at: None,
            },
            Err(error) => DeliveryOutcome {
                status: DeliveryStatus::Failed,
                path,
                detail: format!("delivery request failed: {error}"),
                attempted_at: Some(attempted_at),
                delivered_at: None,
            },
        }
    }

    async fn suppression_reason(&self, channel: Channel, recipient: &str) -> Option<String> {
        let preferences = self.preferences.read().await;
        let preference = preferences.iter().find(|item| item.user_id == recipient)?;
        let enabled = match channel {
            Channel::Email => preference.email_enabled,
            Channel::Sms => preference.sms_enabled,
            Channel::Push => preference.push_enabled,
            Channel::InApp => preference.in_app_enabled,
            Channel::Webhook | Channel::Slack => true,
        };
        if !enabled {
            return Some(format!(
                "{} delivery is disabled by user preference",
                channel_name(channel)
            ));
        }
        let quiet_hours = preference.quiet_hours.as_deref()?;
        if quiet_hours_active(quiet_hours) {
            return Some(format!("suppressed during local quiet hours {quiet_hours}"));
        }
        None
    }

    async fn attempt(
        &self,
        channel: Channel,
        recipient: String,
        subject: Option<String>,
        body: String,
        template_id: Option<String>,
    ) -> Notification {
        let outcome = self
            .deliver(channel, &recipient, subject.as_deref(), &body)
            .await;
        Notification {
            id: format!("notif-{}", &uuid::Uuid::new_v4().to_string()[..8]),
            channel,
            recipient,
            subject,
            body,
            status: outcome.status,
            delivery_path: outcome.path,
            status_detail: outcome.detail,
            template_id,
            attempted_at: outcome.attempted_at,
            delivered_at: outcome.delivered_at,
        }
    }
}

#[tool_router]
impl NotificationServer {
    #[tool(
        description = "Attempt real notification delivery. Webhook accepts an HTTPS recipient directly; email, SMS, push, and symbolic Slack recipients require a configured channel endpoint. Returns delivered only after a successful endpoint response; in_app is queued locally."
    )]
    async fn send_notification(&self, Parameters(input): Parameters<SendInput>) -> String {
        let channel = match parse_channel(&input.channel) {
            Ok(channel) => channel,
            Err(error) => return serde_json::json!({"ok": false, "error": error}).to_string(),
        };
        let notification = self
            .attempt(channel, input.recipient, input.subject, input.body, None)
            .await;
        let result = serde_json::json!({"ok": notification.status != DeliveryStatus::Failed, "notification": notification_view(&notification)});
        self.notifications.write().await.push(notification);
        result.to_string()
    }

    #[tool(description = "Send a notification using a template with variable substitution")]
    async fn send_from_template(&self, Parameters(input): Parameters<SendTemplateInput>) -> String {
        let templates = self.templates.read().await;
        let tpl = match templates.iter().find(|t| t.id == input.template_id) {
            Some(t) => t.clone(),
            None => return error_result(format!("template {} not found", input.template_id)),
        };
        let vars = input.variables.unwrap_or(serde_json::json!({}));
        let mut body = tpl.body_template.clone();
        let mut subject = tpl.subject.clone();
        if let Some(obj) = vars.as_object() {
            for (k, v) in obj {
                let val = v.as_str().unwrap_or("");
                body = body.replace(&format!("{{{{{}}}}}", k), val);
                if let Some(subject) = subject.as_mut() {
                    *subject = subject.replace(&format!("{{{{{}}}}}", k), val);
                }
            }
        }
        let notification = self
            .attempt(
                tpl.channel,
                input.recipient,
                subject,
                body,
                Some(input.template_id),
            )
            .await;
        let result = serde_json::json!({"ok": notification.status != DeliveryStatus::Failed, "notification": notification_view(&notification)});
        self.notifications.write().await.push(notification);
        result.to_string()
    }

    #[tool(description = "Broadcast a notification to multiple recipients")]
    async fn broadcast(&self, Parameters(input): Parameters<BroadcastInput>) -> String {
        adk_mcp_sdk::set_current_task_status("Delivering notifications to broadcast recipients");
        let channel = match parse_channel(&input.channel) {
            Ok(channel) => channel,
            Err(error) => return serde_json::json!({"ok": false, "error": error}).to_string(),
        };
        let mut created = Vec::with_capacity(input.recipients.len());
        for recipient in input.recipients {
            created.push(
                self.attempt(
                    channel,
                    recipient,
                    input.subject.clone(),
                    input.body.clone(),
                    None,
                )
                .await,
            );
        }
        let delivered = created
            .iter()
            .filter(|n| n.status == DeliveryStatus::Delivered)
            .count();
        let queued = created
            .iter()
            .filter(|n| n.status == DeliveryStatus::Queued)
            .count();
        let suppressed = created
            .iter()
            .filter(|n| n.status == DeliveryStatus::Suppressed)
            .count();
        let failed = created.len() - delivered - queued - suppressed;
        self.notifications
            .write()
            .await
            .extend(created.iter().cloned());
        let views: Vec<_> = created.iter().map(notification_view).collect();
        serde_json::json!({"ok": failed == 0, "count": created.len(), "delivered": delivered, "queued": queued, "suppressed": suppressed, "failed": failed, "notifications": views}).to_string()
    }

    #[tool(description = "List sent notifications, optionally filtered by channel")]
    async fn list_notifications(
        &self,
        Parameters(input): Parameters<ChannelFilterInput>,
    ) -> String {
        let notifs = self.notifications.read().await;
        let filtered: Vec<serde_json::Value> = notifs
            .iter()
            .filter(|n| {
                input
                    .channel
                    .as_ref()
                    .is_none_or(|c| format!("{:?}", n.channel).to_lowercase() == *c)
            })
            .map(notification_view)
            .collect();
        serde_json::to_string_pretty(&filtered).unwrap()
    }

    #[tool(description = "Get notification delivery status")]
    async fn get_status(&self, Parameters(input): Parameters<IdInput>) -> String {
        let notifs = self.notifications.read().await;
        match notifs.iter().find(|n| n.id == input.id) {
            Some(n) => serde_json::to_string_pretty(&notification_view(n)).unwrap(),
            None => error_result(format!("notification {} not found", input.id)),
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
            None => error_result(format!("template {} not found", input.id)),
        }
    }

    #[tool(description = "Create a notification template")]
    async fn create_template(&self, Parameters(input): Parameters<CreateTemplateInput>) -> String {
        let id = format!("tpl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let channel = match parse_channel(&input.channel) {
            Ok(channel) => channel,
            Err(error) => return serde_json::json!({"ok": false, "error": error}).to_string(),
        };
        let tpl = Template {
            id: id.clone(),
            name: input.name.clone(),
            channel,
            subject: input.subject,
            body_template: input.body_template,
            variables: input.variables,
        };
        self.templates.write().await.push(tpl);
        format!("Created template '{}' (id: {})", input.name, id)
    }

    #[tool(description = "Get user notification preferences")]
    async fn get_preferences(&self, Parameters(input): Parameters<IdInput>) -> String {
        let prefs = self.preferences.read().await;
        match prefs.iter().find(|p| p.user_id == input.id) {
            Some(p) => serde_json::to_string_pretty(p).unwrap(),
            None => error_result(format!("no preferences for user {}", input.id)),
        }
    }

    #[tool(description = "Update user notification preferences")]
    async fn update_preferences(&self, Parameters(input): Parameters<PreferenceInput>) -> String {
        if input
            .quiet_hours
            .as_deref()
            .is_some_and(|value| parse_quiet_hours(value).is_none())
        {
            return error_result(
                "quiet_hours must be HH:MM-HH:MM with valid, different start and end times",
            );
        }
        let mut prefs = self.preferences.write().await;
        match prefs.iter_mut().find(|p| p.user_id == input.user_id) {
            Some(p) => {
                if let Some(v) = input.email_enabled {
                    p.email_enabled = v;
                }
                if let Some(v) = input.sms_enabled {
                    p.sms_enabled = v;
                }
                if let Some(v) = input.push_enabled {
                    p.push_enabled = v;
                }
                if let Some(v) = input.in_app_enabled {
                    p.in_app_enabled = v;
                }
                if let Some(v) = input.quiet_hours {
                    p.quiet_hours = Some(v);
                }
                format!("Updated preferences for {}", input.user_id)
            }
            None => {
                prefs.push(Preference {
                    user_id: input.user_id.clone(),
                    email_enabled: input.email_enabled.unwrap_or(true),
                    sms_enabled: input.sms_enabled.unwrap_or(true),
                    push_enabled: input.push_enabled.unwrap_or(true),
                    in_app_enabled: input.in_app_enabled.unwrap_or(true),
                    quiet_hours: input.quiet_hours,
                });
                format!("Created preferences for {}", input.user_id)
            }
        }
    }

    #[tool(description = "Get truthful queued, delivered, suppressed, and failed counts")]
    async fn get_delivery_stats(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let notifs = self.notifications.read().await;
        let delivered = notifs
            .iter()
            .filter(|n| matches!(n.status, DeliveryStatus::Delivered))
            .count();
        let failed = notifs
            .iter()
            .filter(|n| matches!(n.status, DeliveryStatus::Failed))
            .count();
        let queued = notifs
            .iter()
            .filter(|n| matches!(n.status, DeliveryStatus::Queued))
            .count();
        let suppressed = notifs
            .iter()
            .filter(|n| matches!(n.status, DeliveryStatus::Suppressed))
            .count();
        serde_json::to_string_pretty(&serde_json::json!({"total": notifs.len(), "delivered": delivered, "failed": failed, "queued": queued, "suppressed": suppressed})).unwrap()
    }

    #[tool(
        description = "Queue a notification record for a future scheduler. This tool does not claim delivery and does not run a scheduler."
    )]
    async fn schedule_notification(&self, Parameters(input): Parameters<SendInput>) -> String {
        let id = format!("notif-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let channel = match parse_channel(&input.channel) {
            Ok(channel) => channel,
            Err(error) => return serde_json::json!({"ok": false, "error": error}).to_string(),
        };
        let notif = Notification {
            id: id.clone(),
            channel,
            recipient: input.recipient,
            subject: input.subject,
            body: input.body,
            status: DeliveryStatus::Queued,
            delivery_path: "scheduler_not_implemented".into(),
            status_detail: "queued as a record only; no scheduler is running".into(),
            template_id: None,
            attempted_at: None,
            delivered_at: None,
        };
        self.notifications.write().await.push(notif);
        serde_json::json!({"ok": true, "notification_id": id, "status": "queued", "delivery_attempted": false}).to_string()
    }

    #[tool(description = "Retry a failed notification")]
    async fn retry_notification(&self, Parameters(input): Parameters<IdInput>) -> String {
        let original = self
            .notifications
            .read()
            .await
            .iter()
            .find(|n| n.id == input.id)
            .cloned();
        let Some(original) = original else {
            return serde_json::json!({"ok": false, "error": format!("notification {} not found", input.id)}).to_string();
        };
        if original.status != DeliveryStatus::Failed {
            return serde_json::json!({"ok": false, "error": format!("notification {} is not failed", input.id), "status": original.status}).to_string();
        }
        let outcome = self
            .deliver(
                original.channel,
                &original.recipient,
                original.subject.as_deref(),
                &original.body,
            )
            .await;
        let mut notifs = self.notifications.write().await;
        let notification = notifs
            .iter_mut()
            .find(|n| n.id == input.id)
            .expect("notification cannot disappear");
        notification.status = outcome.status;
        notification.delivery_path = outcome.path;
        notification.status_detail = outcome.detail;
        notification.attempted_at = outcome.attempted_at;
        notification.delivered_at = outcome.delivered_at;
        serde_json::json!({"ok": notification.status != DeliveryStatus::Failed, "notification": notification_view(notification)}).to_string()
    }
}

fn quiet_hours_active(specification: &str) -> bool {
    let Some((start, end)) = parse_quiet_hours(specification) else {
        return false;
    };
    let now = Local::now().hour() * 60 + Local::now().minute();
    if start <= end {
        (start..end).contains(&now)
    } else {
        now >= start || now < end
    }
}

fn parse_quiet_hours(specification: &str) -> Option<(u32, u32)> {
    fn minutes(value: &str) -> Option<u32> {
        let (hour, minute) = value.split_once(':')?;
        if hour.len() != 2 || minute.len() != 2 {
            return None;
        }
        let hour: u32 = hour.parse().ok()?;
        let minute: u32 = minute.parse().ok()?;
        (hour < 24 && minute < 60).then_some(hour * 60 + minute)
    }
    let (start, end) = specification.split_once('-')?;
    let (start, end) = (minutes(start)?, minutes(end)?);
    (start != end).then_some((start, end))
}

adk_mcp_sdk::mcp_2026_server! {
    server: NotificationServer,
    task_tools: ["broadcast"],
    task_ttl_overrides: [("broadcast", 600_000)],
    approval_tools: ["send_notification", "send_from_template", "broadcast", "retry_notification"],
    mutating_tools: ["send_notification", "send_from_template", "broadcast", "schedule_notification", "retry_notification", "create_template", "update_preferences"],
    destructive_tools: [],
    idempotent_tools: [],
    cache_ttl_ms: 86_400_000,
    instructions: "Deliver notifications only through configured HTTPS endpoints. A notification is marked delivered only after an endpoint accepts it. Missing or failed delivery paths return structured failures; in-app and scheduled records remain explicitly queued. External delivery requires MCP 2026 MRTR approval.",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_email_is_failed_not_sent() {
        let server = NotificationServer::seeded();
        let outcome = server
            .deliver(Channel::Email, "person@example.test", Some("Hello"), "Body")
            .await;
        assert_eq!(outcome.status, DeliveryStatus::Failed);
        assert!(outcome.detail.contains("no delivery path configured"));
    }

    #[tokio::test]
    async fn accepted_webhook_is_delivered_and_secret_path_is_redacted() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let responder = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let _ = stream.read(&mut request).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
        });
        let endpoint = format!("http://{address}/secret/token");
        let server = NotificationServer::seeded();
        let outcome = server
            .deliver(Channel::Webhook, &endpoint, Some("Hello"), "Body")
            .await;
        responder.await.unwrap();
        assert_eq!(outcome.status, DeliveryStatus::Delivered);
        assert_eq!(safe_recipient(&endpoint), format!("http://{address}/…"));
    }

    #[tokio::test]
    async fn disabled_channel_is_suppressed_without_a_delivery_attempt() {
        let server = NotificationServer::seeded();
        let outcome = server.deliver(Channel::Sms, "user-2", None, "Body").await;
        assert_eq!(outcome.status, DeliveryStatus::Suppressed);
        assert_eq!(outcome.attempted_at, None);
        assert_eq!(outcome.path, "preference_policy");
    }

    #[test]
    fn quiet_hours_handle_windows_that_cross_midnight() {
        assert!(!quiet_hours_active("invalid"));
        assert_eq!(parse_quiet_hours("22:00-22:00"), None);
        let now = Local::now();
        let start = (now.hour() + 23) % 24;
        let end = (now.hour() + 1) % 24;
        assert!(quiet_hours_active(&format!("{start:02}:00-{end:02}:59")));
    }
}
