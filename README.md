# Notifications MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-notifications.svg)](https://crates.io/crates/mcp-notifications)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://www.zavora.ai)

Multi-channel notifications for AI agents — email, SMS, push, in-app, webhooks, Slack. Templates with variable substitution, broadcasting, user preferences, quiet hours, and delivery tracking. 13 tools.

## Architecture

<p align="center">
  <img src="https://raw.githubusercontent.com/zavora-ai/mcp-notifications/main/docs/assets/architecture.svg" alt="MCP Notifications Architecture" width="850"/>
</p>

## Tools (13)

### Sending (4)

| Tool | Purpose | Risk |
|------|---------|------|
| `send_notification` | Send via any channel | internal_write |
| `send_from_template` | Send using template + variables | internal_write |
| `broadcast` | Send to multiple recipients | internal_write |
| `schedule_notification` | Schedule for future delivery | internal_write |

### Templates (3)

| Tool | Purpose | Risk |
|------|---------|------|
| `list_templates` | List notification templates | read_only |
| `get_template` | Get template details + variables | read_only |
| `create_template` | Create a new template | internal_write |

### Tracking (3)

| Tool | Purpose | Risk |
|------|---------|------|
| `list_notifications` | List sent notifications | read_only |
| `get_status` | Get delivery status | read_only |
| `retry_notification` | Retry a failed notification | internal_write |

### Preferences (3)

| Tool | Purpose | Risk |
|------|---------|------|
| `get_preferences` | Get user channel preferences | read_only |
| `update_preferences` | Update preferences + quiet hours | internal_write |
| `get_delivery_stats` | Sent/delivered/failed counts | read_only |

## Channels

| Channel | Use Case |
|---------|----------|
| 📧 Email | Formal communications, reports |
| 📱 SMS | OTP codes, urgent alerts |
| 🔔 Push | Real-time mobile/desktop alerts |
| 💬 In-App | Activity feeds, updates |
| 🔗 Webhook | System integrations |
| 💼 Slack | Team notifications |

## Installation

```bash
cargo install mcp-notifications
```

## Configuration

No configuration needed — starts with demo templates:
- **Welcome Email** — `{{name}}`, `{{company}}`, `{{url}}`
- **OTP SMS** — `{{code}}`
- **System Alert** — `{{title}}`, `{{message}}`

Future backends (Twilio, SendGrid, Firebase, SNS) will use environment variables.

## Client Configuration

### Claude Desktop / Kiro / Cursor

```json
{
  "mcpServers": {
    "notifications": {
      "command": "mcp-notifications",
      "args": []
    }
  }
}
```

## Usage Examples

### Send a notification
```
"Send an email to alice@company.com about the deployment"
→ send_notification(channel="email", recipient="alice@company.com", subject="Deploy Complete", body="v2.1 is live")
```

### Use a template
```
"Send a welcome email to Frank at Zavora"
→ send_from_template(template_id="tpl-welcome", recipient="frank@zavora.ai", variables={"name":"Frank","company":"Zavora","url":"https://app.zavora.ai"})
```

### Broadcast
```
"Notify all users about maintenance"
→ broadcast(channel="push", recipients=["user-1","user-2","user-3"], body="Maintenance in 1 hour")
```

### Check delivery
```
"Did the notification to Alice get delivered?"
→ get_status(id="notif-xxx")
```

## Delivery States

```
Queued → Sent → Delivered
                ↘ Failed → (retry)
                ↘ Bounced
```

## MCP Server Manifest

```toml
server_id = "mcp_notifications"
display_name = "Notifications"
version = "1.0.0"
domain = "platform-core"
risk_level = "medium"
writes_allowed = "gated"
```

## License

Apache-2.0

---

Part of the [ADK-Rust Enterprise](https://enterprise.adk-rust.com) MCP server ecosystem.

Built with ❤️ by [Zavora AI](https://zavora.ai)

## rmcp and MCP compatibility

This server is built with [`rmcp` 3.1.2](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v3.1.2) and requires Rust 1.94.1 or newer. The rmcp 3 rollout retains legacy MCP initialization compatibility and targets MCP protocol revisions `2025-11-25` and `2026-07-28`.

## MCP 2026-07-28 rollout (P4 workflow/business)

This server uses `rmcp` 3.1.2 and `adk-mcp-sdk` 0.2 with a minimum supported
Rust version of **1.94.1**. It accepts stateless MCP 2026 requests with
per-request protocol, client identity, and capability metadata while retaining
the legacy MCP 2025-11-25 initialize flow for ordinary tools.

- **Tasks:** None; this server's operations are short-lived and execute directly.
- **MRTR approvals:** None; this server exposes no manifest-classified protected operations.
- **Discovery and routing:** rmcp serves on-demand discovery and validates the
  per-request protocol envelope; HTTP deployments can route with `Mcp-Method`
  and `Mcp-Name`. The packaged binary currently uses stdio.
- **Caching:** `tools/list` returns a public `ttlMs` of 60,000 for MCP 2026;
  rmcp omits the cache fields for legacy clients.
- **Deprecated extensions:** this server does not add new Roots, Sampling, or
  dynamic client-registration dependencies.

Protected tools require `MCP_REQUEST_STATE_KEY` with at least 32 high-entropy
bytes. All replicas must share that key so sealed approval state can resume on
another instance. Approval state is bound to the client identity, tool, and
arguments and expires after two minutes. Missing identity, invalid state,
rejection, or legacy protocol use fails closed. Task records are process-local
for the current stdio runtime; use a durable task store before deploying the
server behind scale-to-zero HTTP infrastructure.
