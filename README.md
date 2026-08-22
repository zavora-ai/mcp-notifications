# Notifications MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-notifications.svg)](https://crates.io/crates/mcp-notifications)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://www.zavora.ai)

Truthful multi-channel delivery for AI agents. Webhooks and Slack incoming webhooks can be called directly; email, SMS, push, and Slack can use configured HTTPS gateway adapters. A record becomes `delivered` only after its endpoint returns a successful HTTP status. Missing or failed delivery paths are `failed`, never simulated as sent.

## Architecture

<p align="center">
  <img src="https://raw.githubusercontent.com/zavora-ai/mcp-notifications/main/docs/assets/architecture.svg" alt="MCP Notifications Architecture" width="850"/>
</p>

## Tools (13)

### Sending (4)

| Tool | Purpose | Risk |
|------|---------|------|
| `send_notification` | Attempt delivery and return endpoint-confirmed status | external_write · MRTR |
| `send_from_template` | Render a template, then attempt delivery | external_write · MRTR |
| `broadcast` | Deliver to multiple recipients; supports MCP Tasks | external_write · MRTR |
| `schedule_notification` | Queue a record only; no scheduler is claimed | internal_write |

### Templates (3)

| Tool | Purpose | Risk |
|------|---------|------|
| `list_templates` | List notification templates | read_only |
| `get_template` | Get template details + variables | read_only |
| `create_template` | Create a new template | internal_write |

### Tracking (3)

| Tool | Purpose | Risk |
|------|---------|------|
| `list_notifications` | List queued, delivered, suppressed, and failed attempts | read_only |
| `get_status` | Get delivery status | read_only |
| `retry_notification` | Retry a failed notification through its real path | external_write · MRTR |

### Preferences (3)

| Tool | Purpose | Risk |
|------|---------|------|
| `get_preferences` | Get user channel preferences | read_only |
| `update_preferences` | Update preferences + quiet hours | internal_write |
| `get_delivery_stats` | Queued/delivered/suppressed/failed counts | read_only |

## Channels

| Channel | Delivery path |
|---------|---------------|
| Email | `MCP_NOTIFICATIONS_EMAIL_ENDPOINT` HTTPS adapter |
| SMS | `MCP_NOTIFICATIONS_SMS_ENDPOINT` HTTPS adapter |
| Push | `MCP_NOTIFICATIONS_PUSH_ENDPOINT` HTTPS adapter |
| In-app | Explicit local queue; no external delivery claim |
| Webhook | HTTPS URL supplied as `recipient` |
| Slack | Incoming HTTPS webhook as `recipient`, or `MCP_NOTIFICATIONS_SLACK_ENDPOINT` |

## Installation

```bash
cargo install --git https://github.com/zavora-ai/mcp-notifications --tag v1.2.0
```

The tagged GitHub install provides this release immediately. The unqualified
`cargo install mcp-notifications` command may resolve to an older crates.io release.

## Configuration

The server starts without credentials, but only webhook recipients and the local in-app queue work without a gateway. Configure one endpoint per channel as needed:

```bash
export MCP_NOTIFICATIONS_EMAIL_ENDPOINT=https://gateway.example/v1/email
export MCP_NOTIFICATIONS_SMS_ENDPOINT=https://gateway.example/v1/sms
export MCP_NOTIFICATIONS_PUSH_ENDPOINT=https://gateway.example/v1/push
export MCP_NOTIFICATIONS_SLACK_ENDPOINT=https://hooks.slack.com/services/...
export MCP_NOTIFICATIONS_BEARER_TOKEN=secret   # optional shared gateway auth
export MCP_REQUEST_STATE_KEY='at-least-32-high-entropy-bytes'
```

Gateway requests contain `channel`, `recipient`, `subject`, and `body`. Endpoint URLs are redacted from tool responses. Demo templates are included:
- **Welcome Email** — `{{name}}`, `{{company}}`, `{{url}}`
- **OTP SMS** — `{{code}}`
- **System Alert** — `{{title}}`, `{{message}}`

An adapter can forward this stable JSON contract to Twilio, SendGrid, Firebase, SNS, or an internal delivery service without this MCP server pretending that an unconfigured provider exists.

When `recipient` matches a stored preference `user_id`, disabled channels and active `HH:MM-HH:MM` quiet hours are enforced before delivery. Quiet hours use the server's local timezone and produce `suppressed`, with no network request.

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
queued        local in-app or scheduler record; no delivery attempted
delivered     a configured HTTP endpoint returned 2xx
suppressed    channel preference or quiet hours prevented an attempt
failed        no delivery path, transport failure, or non-2xx response
```

## MCP Server Manifest

```toml
server_id = "mcp_notifications"
display_name = "Notifications"
version = "1.2.0"
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

This server uses exact `rmcp` 3.1.2 and `adk-mcp-sdk` 0.3 with a minimum supported
Rust version of **1.94.1**. It accepts stateless MCP 2026 requests with
per-request protocol, client identity, and capability metadata while retaining
the legacy MCP 2025-11-25 initialize flow for ordinary tools.

- **Structured results:** every tool advertises an `outputSchema`; JSON responses are also returned as MCP 2026 `structuredContent`, and `{ "ok": false }` becomes `isError: true`.
- **Tool annotations:** read-only, mutating, destructive, idempotent, and open-world hints are generated from the declared policy.
- **Tasks:** `broadcast`, with a 10-minute TTL and live status text.
- **MRTR approvals:** `send_notification`, `send_from_template`, `broadcast`, and `retry_notification` because they write to external systems.
- **Discovery and routing:** rmcp serves on-demand discovery and validates the
  per-request protocol envelope; HTTP deployments can route with `Mcp-Method`
  and `Mcp-Name`. The packaged binary currently uses stdio.
- **Caching:** `tools/list` returns a public `ttlMs` of 86,400,000 for MCP 2026;
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
