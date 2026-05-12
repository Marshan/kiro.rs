## ADDED Requirements

### Requirement: 400 响应结构化诊断日志

当 Kiro 上游返回 HTTP 400 时，系统 SHALL 在 `anyhow::bail!` 退出之前写入一条结构化 `tracing::error!` 日志，包含当前请求使用的 `credential_id`、`endpoint` 名称、`profile_arn`、`status`、`body` 字段，用于事后定性是上游瞬态故障（含地域风控）还是本项目构造 bug。本要求 MUST NOT 改变 400 的重试/退避/凭据切换行为。

#### Scenario: API 路径 400 日志带完整字段

- **WHEN** `call_api_with_retry` 中上游返回 400（例如 `{"reason":"INVALID_MODEL_ID"}`）
- **THEN** 系统 SHALL 在 bail 之前打印一条 `ERROR` 级日志，字段包含 `credential_id`（当前凭据 id）、`endpoint`（`KiroEndpoint::name()` 的返回值，如 `"ide"`）、`profile_arn`（凭据的 `profile_arn` 或缺省字符串 `<none>`）、`status`（HTTP 状态码字符串）、`body`（上游响应体原文）

#### Scenario: MCP 路径 400 日志带完整字段

- **WHEN** `call_mcp_with_retry` 中上游返回 400
- **THEN** 系统 SHALL 在 bail 之前打印一条 `ERROR` 级日志，字段集合与 API 路径一致（`credential_id`、`endpoint`、`profile_arn`、`status`、`body`）

#### Scenario: 400 日志不触发凭据切换与退避

- **WHEN** 系统在 400 分支写入诊断日志
- **THEN** 系统 SHALL NOT 调用 `report_failure` / `mark_cooldown`，且 SHALL NOT 更新 `last_used_at`，随后 SHALL 立即以原有错误文本 `bail!` 退出（对外响应保持 502 行为）

#### Scenario: 不同凭据承接的连续 400 可按 credential_id 聚合

- **WHEN** 多次 400 发生在不同凭据上（轮转路径）
- **THEN** 日志读者 SHALL 能通过 `grep credential_id=` 聚合判断 400 分布（同一 id 连续 = 倾向上游瞬态；多 id 分布 = 倾向项目构造 bug）
