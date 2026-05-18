## 1. Suspicious Activity 429 差异化处理（provider.rs）

- [x] 1.1 在 `provider.rs` 中添加辅助函数 `is_suspicious_activity_429(body: &str) -> bool`，检查响应体是否包含 "suspicious activity" 字符串
- [x] 1.2 修改 API 路径（`call_api_with_retry`）的 429 处理逻辑：suspicious activity 应用 600s cooldown（上限 600s）并调用 `report_failure(ctx.id)`；容量不足保持 30s cooldown（上限 120s）不调用 `report_failure`
- [x] 1.3 修改 MCP 路径（`call_mcp_with_retry`）的 429 处理逻辑：与 API 路径保持一致的差异化处理
- [x] 1.4 更新 API 路径 429 的 cooldown 日志消息，区分 "可疑活动限流" 和 "容量不足限流"
- [x] 1.5 更新 MCP 路径 429 的 cooldown 日志消息，同上

## 2. 日志增强（handlers.rs）

- [x] 2.1 将 `handlers.rs` 中 "Received POST /v1/messages request" 的 `tracing::info!` 调用移到 `model_kiro` 计算之后（约 line 266-267），并在日志中加入 `kiro_model = %model_kiro` 字段

## 3. 日志增强（provider.rs）

- [x] 3.1 在 API 路径 `acquire_context` 返回后新增 `tracing::info!` 日志，包含 `credential_id = ctx.id` 和 `kiro_model = %model.as_deref().unwrap_or("unknown")`，消息格式为 "凭据 #{} 开始请求"
- [x] 3.2 在 MCP 路径 `acquire_context` 返回后新增同格式的 `tracing::info!` 日志
- [x] 3.3 在 API 路径的 429 WARN 日志中加入 `credential_id = ctx.id` 和 `kiro_model = %model.as_deref().unwrap_or("unknown")` 字段
- [x] 3.4 在 MCP 路径的 429 WARN 日志中加入相同字段

## 4. 验证

- [x] 4.1 运行 `cargo build --release` 确认编译通过，无警告
- [x] 4.2 运行 `cargo test` 确认所有测试通过
- [x] 4.3 运行 `cargo clippy` 确认无 lint 问题
