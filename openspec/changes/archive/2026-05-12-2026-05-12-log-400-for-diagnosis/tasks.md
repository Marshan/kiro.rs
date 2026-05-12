## 1. provider.rs 400 分支结构化日志

- [x] 1.1 在 `src/kiro/provider.rs` API 路径 400 分支（`call_api_with_retry` 内）`anyhow::bail!` 之前新增 `tracing::error!`，字段：`credential_id = ctx.id`、`endpoint = endpoint.name()`、`profile_arn = ctx.credentials.profile_arn.as_deref().unwrap_or("<none>")`、`status = %status`、`body = %body`，消息 `"API 400 失败（按设计不重试不切换）"`
- [x] 1.2 在同文件 MCP 路径 400 分支（`call_mcp_with_retry` 内）`anyhow::bail!` 之前新增相同字段集合的 `tracing::error!`，消息 `"MCP 400 失败（按设计不重试不切换）"`
- [x] 1.3 确认 `bail!` 的错误文本保持不变（API：`"{} API 请求失败: {} {}"`、MCP：`"MCP 请求失败: {} {}"`），避免影响 `map_provider_error` 下游行为

## 2. 代码健壮性检查

- [x] 2.1 `cargo build` 通过
- [x] 2.2 `cargo test` 全部通过（215 passed, 0 failed）
- [x] 2.3 确认本次未触碰凭据状态机：400 分支仍不调用 `report_failure` / `mark_cooldown`，也不更新 `last_used_at`；`src/kiro/token_manager.rs` 和 `src/anthropic/handlers.rs` 无改动
