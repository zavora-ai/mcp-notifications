# Notifications MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-notifications.svg)](https://crates.io/crates/mcp-notifications)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Multi-channel notifications for AI agents — email, SMS, push, in-app, webhooks, Slack. Templates, broadcasting, preferences, delivery tracking. 13 tools.

## Tools (13)

| Tool | Purpose | Risk |
|------|---------|------|
| `send_notification` | Send via any channel | internal_write |
| `send_from_template` | Send using template + variables | internal_write |
| `broadcast` | Send to multiple recipients | internal_write |
| `schedule_notification` | Schedule for future delivery | internal_write |
| `list_notifications` | List sent notifications | read_only |
| `get_status` | Get delivery status | read_only |
| `retry_notification` | Retry a failed notification | internal_write |
| `list_templates` | List notification templates | read_only |
| `get_template` | Get template details | read_only |
| `create_template` | Create a template | internal_write |
| `get_preferences` | Get user notification preferences | read_only |
| `update_preferences` | Update preferences (channels, quiet hours) | internal_write |
| `get_delivery_stats` | Delivery stats (sent/delivered/failed) | read_only |

## Installation

```bash
cargo install mcp-notifications
```

## Configuration

No configuration needed — starts with demo templates (Welcome Email, OTP SMS, System Alert).

```json
{ "mcpServers": { "notifications": { "command": "mcp-notifications" } } }
```

## License

Apache-2.0 — Part of [ADK-Rust Enterprise](https://enterprise.adk-rust.com)
