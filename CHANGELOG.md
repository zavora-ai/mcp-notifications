# Changelog

## [1.2.0] - 2026-08-22

### Fixed

- Removed simulated `sent` results. Delivery is now `delivered` only after an HTTPS endpoint accepts the request, `failed` when no path exists or delivery fails, and `queued` only when no attempt is claimed.
- Redacted webhook paths from MCP results and tracked actual attempt/delivery timestamps.
- Enforced stored channel preferences and local-time quiet hours with an explicit `suppressed` state.

### Added

- Direct webhook and Slack webhook delivery plus configurable HTTPS adapters for email, SMS, push, and Slack.
- MCP Tasks for broadcasts, sealed MRTR approval for external delivery, structured results, output schemas, tool annotations, and 24-hour tool discovery caching through adk-mcp-sdk 0.3.0.
- MCP 2025-11-25 and 2026-07-28 protocol smoke tests and real local HTTP delivery tests.
