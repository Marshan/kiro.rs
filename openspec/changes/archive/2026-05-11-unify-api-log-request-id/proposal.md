## Why

`src/api_logger.rs` 打印的 REQUEST / KIRO REQUEST / RESPONSE 三行日志，`request_id` 字段各自独立：REQUEST 使用在 `src/anthropic/handlers.rs:295` 临时生成的 UUID，KIRO REQUEST 使用字面量 `"pending"` 占位，RESPONSE 使用 `StreamContext.message_id`。同一次入站调用在日志里拿不到同一个 ID，排障时无法把三行关联起来。

## What Changes

- 在 Anthropic handler 入口一次性生成单个 `request_id`，作为日志追踪标识。
- 将该 `request_id` 透传到 `handle_stream_request` 与 `handle_non_stream_request`，并写入 REQUEST、KIRO REQUEST、RESPONSE 三处日志。
- 去掉 KIRO REQUEST 使用的 `"pending"` 占位。
- 保留 `StreamContext.message_id` 作为返回给客户端的 Anthropic SSE `message.id`，与日志 `request_id` 职责分离。
- 非破坏性：Anthropic 对外响应格式不变，仅本地日志行为变化。

## Capabilities

### New Capabilities
- `api-logging`: 覆盖 Anthropic 兼容层请求日志的结构与字段语义，规定同一次入站请求在 REQUEST / KIRO REQUEST / RESPONSE 三行日志中共用同一个 `request_id`。

### Modified Capabilities
<!-- 无 -->

## Impact

- 代码：`src/anthropic/handlers.rs`（生成并传递 `request_id`，修改两个 handler 函数签名与 SSE 流工厂调用）、`src/api_logger.rs` 无签名变化（字段名保持 `request_id`）。
- API：对外 Anthropic API 行为不变，客户端看到的 `message.id` 仍由 `StreamContext` 生成。
- 依赖：无新增依赖。
- 观测：日志读者可通过单一 `request_id` 贯穿定位一次请求的三行记录。
