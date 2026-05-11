## 1. Config

- [ ] 1.1 在 `src/model/config.rs` 的 `Config` struct 新增 `api_log_enabled: bool`（默认 false）和 `api_log_path: Option<String>` 字段

## 2. ApiLogger 模块

- [ ] 2.1 新建 `src/api_logger.rs`，定义 `ApiLogger` struct，持有 `tokio::sync::mpsc::Sender<String>`
- [ ] 2.2 实现 `ApiLogger::new(path: &str) -> Self`，启动后台写文件 task（有界 channel，容量 1024）
- [ ] 2.3 实现 `ApiLogger::log(&self, entry: String)`，用 `try_send` 异步投递，满时静默丢弃
- [ ] 2.4 实现三个格式化方法：`format_request`、`format_kiro_request`、`format_response`，输出人类可读文本块
- [ ] 2.5 在 `src/main.rs` 的 `mod` 声明中注册 `api_logger` 模块

## 3. AppState 集成

- [ ] 3.1 在 `src/anthropic/middleware.rs` 的 `AppState` 新增 `api_logger: Option<Arc<ApiLogger>>` 字段
- [ ] 3.2 更新 `AppState::new()` 签名，新增 `api_logger` 参数（或用 builder 方法）
- [ ] 3.3 在 `src/main.rs` 中根据 config 初始化 `ApiLogger`，注入 `AppState`

## 4. StreamContext 输出文本累积

- [ ] 4.1 在 `src/anthropic/stream.rs` 的 `StreamContext` 新增 `pub output_text: String` 字段，初始化为空字符串
- [ ] 4.2 在 `StreamContext::create_text_delta_events()` 中追加文本到 `output_text`

## 5. 日志调用点（handlers.rs）

- [ ] 5.1 在 `post_messages()` 入口（convert_request 成功后）调用 `format_request` 并写入日志，包含 `model_in`、`model_kiro`、完整 messages
- [ ] 5.2 在 `handle_stream_request()` 和 `handle_non_stream_request()` 中，provider.call_api 调用前写入 KIRO REQUEST 日志（含 url、request_body）
- [ ] 5.3 在流式响应结束时（`create_sse_stream` 的 `None` 分支），从 `ctx.output_text` 和 `ctx.output_tokens` 读取数据，写入 RESPONSE 日志
- [ ] 5.4 在非流式响应完成后写入 RESPONSE 日志（含完整 content、stop_reason、tokens、duration_ms）

## 6. 验证

- [ ] 6.1 运行 `cargo test` 确认无编译错误和测试失败
- [ ] 6.2 运行 `cargo clippy` 确认无新增警告
