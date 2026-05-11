## 1. 入口生成 request_id

- [x] 1.1 在 `src/anthropic/handlers.rs` 的 `create_message`（`/v1/messages` 入口）里、现有 REQUEST 日志写入点之前，新增一行生成 `let request_id = format!("msg_{}", Uuid::new_v4().to_string().replace('-', ""));`，替换原来日志语句内部的 `format!` 临时生成
- [x] 1.2 在 `post_messages_cc`（`/cc/v1/messages` 入口）里同样在 REQUEST 日志写入点前生成 `request_id`，替换原来日志语句内部的临时 `format!`
- [x] 1.3 将该 `request_id` 通过参数透传给 `handle_stream_request`、`handle_stream_request_buffered` 和 `handle_non_stream_request`（三者签名各新增一个 `request_id: &str` 参数）

## 2. 流式路径（/v1/messages）日志对齐

- [x] 2.1 在 `handle_stream_request` 中，把 KIRO REQUEST 日志 `format_kiro_request` 的第一个参数从 `"pending"` 改为传入的 `request_id`
- [x] 2.2 将 `request_id` 继续透传给 `create_sse_stream`（该工厂函数同样新增 `request_id` 参数，并随状态闭包一起持有）
- [x] 2.3 在 SSE 流终止分支（`body_stream` 返回 `None` 时）调用 `format_response` 处，将第一个参数从 `&ctx.message_id` 改为 `&request_id`
- [x] 2.4 保留 `ctx.message_id` 的所有既有用途（`message_start` 等 SSE 事件），不做修改

## 3. 流式路径（/cc/v1/messages 缓冲模式）日志对齐

- [x] 3.1 在 `handle_stream_request_buffered` 中，把 KIRO REQUEST 日志 `format_kiro_request` 的第一个参数从 `"pending"` 改为传入的 `request_id`
- [x] 3.2 将 `request_id` 继续透传给 `create_buffered_sse_stream`（该工厂函数同样新增 `request_id` 参数，并随状态闭包一起持有）
- [x] 3.3 在该工厂函数的 RESPONSE 日志写入点，将第一个参数从现有 `ctx.inner.message_id` 改为 `request_id`

## 4. 非流式路径日志对齐

- [x] 4.1 在 `handle_non_stream_request` 中，将 KIRO REQUEST 日志调用里的 `"pending"` 占位改为 `request_id`
- [x] 4.2 将同一函数内 RESPONSE 日志调用（`format_response`）的第一个参数从使用 `msg_id`（函数内部生成的对外响应 `id`）改为 `request_id`

## 5. 代码健壮性检查

- [x] 5.1 全局搜索 `"pending"`、`ApiLogger::format_kiro_request`、`ApiLogger::format_response`，确认没有遗漏调用点仍然使用旧逻辑
- [x] 5.2 确认 `src/api_logger.rs` 的模板字符串、字段名、列宽未改动（`git diff src/api_logger.rs` 为空）

## 6. 验证

- [x] 6.1 `cargo build --release` 通过
- [x] 6.2 `cargo clippy` 无新增 warning（本次修改的 `handlers.rs` / `api_logger.rs` 未产生新 warning；其他文件中存在的 `field_reassign_with_default` 等是预先就有的旧告警）
- [x] 6.3 `cargo test` 全部通过（208 passed, 0 failed）
