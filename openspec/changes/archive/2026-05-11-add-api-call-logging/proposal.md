## Why

每次 LLM API 调用的入参和出参目前只有零散的 tracing 日志，无法完整追踪一次请求的全貌。调试问题时需要同时关注：客户端传入的原始模型名、映射后实际发给 Kiro 的模型名、完整的消息内容、Kiro 请求体、以及最终的响应内容和 token 统计。这些信息目前分散在不同日志级别，且响应内容完全缺失。

## What Changes

- 新增 `src/api_logger.rs` 模块，实现独立的 API 调用日志记录器
- `config.json` 新增 `apiLogEnabled`（bool）和 `apiLogPath`（string）两个配置字段
- `src/model/config.rs` 对应新增两个字段
- `src/anthropic/stream.rs` 的 `StreamContext` 新增 `output_text` 字段，在流式处理时累积完整输出文本
- `src/anthropic/handlers.rs` 在请求入口、Kiro 请求发出前、响应完成后三个时机写入 API 日志
- `src/main.rs` 初始化 `ApiLogger` 并注入 `AppState`

## Capabilities

### New Capabilities

- `api-call-logging`: 可配置的 API 调用详细日志，记录每次请求的双向入参出参，写入独立文件

### Modified Capabilities

## Impact

- `src/api_logger.rs`：新文件
- `src/model/config.rs`：新增两个字段
- `src/anthropic/stream.rs`：`StreamContext` 新增 `output_text` 字段
- `src/anthropic/handlers.rs`：三处新增日志调用
- `src/main.rs`：初始化并注入 ApiLogger
- `AppState`（`src/main.rs` 或相关文件）：新增 `api_logger` 字段
- 不影响现有 tracing 日志，完全向后兼容，默认关闭
