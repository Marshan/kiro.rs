## Context

`src/anthropic/handlers.rs` 中的 REQUEST、KIRO REQUEST、RESPONSE 三处日志各自使用不同来源的 `request_id`：

- REQUEST（`handlers.rs:294-302`）：在写日志时临时 `Uuid::new_v4()` 生成 `msg_xxx`。
- KIRO REQUEST（`handlers.rs:357-362`）：硬编码 `"pending"`。
- RESPONSE（`handlers.rs:478-488` 以及非流式路径）：使用 `StreamContext::new_with_thinking` 内部生成的 `message_id`，该字段同时是返回给客户端 SSE 事件的 `message.id`。

结果是每次请求在日志里会出现三个不同的 ID，无法根据 ID 把一次请求的三行日志串起来。`StreamContext.message_id` 是对外 API 契约的一部分（Anthropic SSE `message_start`），不能直接替换为内部追踪 ID。

## Goals / Non-Goals

**Goals:**
- 同一次入站 HTTP 请求，在 REQUEST / KIRO REQUEST / RESPONSE 三行日志中共用同一个 `request_id`。
- 流式与非流式两条代码路径都满足上述不变式。
- `StreamContext.message_id` 的生成与返回给客户端的 Anthropic SSE 契约保持不变。

**Non-Goals:**
- 不改动 `src/api_logger.rs` 的日志模板（字段名、列宽、格式均保持原样）。
- 不改动 Kiro 下游请求体内容与 URL。
- 不引入结构化日志 / tracing span 等更大范围的观测性重构。
- 不修改凭据管理、token 刷新等与日志无关的模块。

## Decisions

### 1. 在 handler 入口生成唯一 `request_id`

在 `create_message`（Anthropic handler 的入口）内、实际写第一条 REQUEST 日志之前生成：

```rust
let request_id = format!("msg_{}", Uuid::new_v4().to_string().replace('-', ""));
```

该值作为本次入站调用的唯一日志追踪 ID。

**为什么在 handler 入口生成而不是在各日志点各自生成？**
因为需要在 REQUEST、KIRO REQUEST、RESPONSE 三处都引用同一个值，唯一的"源头"只能在最外层。

**为什么不直接用 `StreamContext.message_id` 作为 `request_id`？**
`StreamContext` 在调用 Kiro API 成功之后才创建（见 `handlers.rs:374`），但 REQUEST / KIRO REQUEST 两行日志需要在调用 Kiro 之前就写出来。此外 `message_id` 是对外 Anthropic SSE 契约的一部分，与内部日志追踪解耦更稳妥。

### 2. 通过函数参数把 `request_id` 传到两个 handler

`handle_stream_request` 和 `handle_non_stream_request` 的签名增加 `request_id: &str`（或 `String`）参数，并在内部：

- 替换 KIRO REQUEST 日志的 `"pending"` 占位。
- 传入到 SSE 流工厂 `create_sse_stream`，在流结束时用它写 RESPONSE 日志（替换现有对 `ctx.message_id` 的使用）。
- 非流式路径中，在调用 Kiro 返回后写 RESPONSE 日志时使用它。

**为什么用参数传递而不放到某个全局/Context 里？**
改动面最小、显式，符合项目现有函数签名风格（`model_in`、`model_kiro` 等也是这样透传）。不需要为单字段引入新 struct。

**备选方案（未采用）**：
- 引入 `RequestLogContext { request_id, model_in, model_kiro, api_logger, start }` struct 并全局传递。过度抽象，改动范围超出本次修复目标。
- 把 `request_id` 塞进 `StreamContext`。会把日志用 ID 与对外 SSE ID 混在一起，职责不清。

### 3. RESPONSE 日志改用 `request_id`，`ctx.message_id` 保持独立用途

流式路径下，`create_sse_stream` 接收 `request_id` 参数，在流终止分支（`handlers.rs:472-489`）把 `&ctx.message_id` 替换为 `&request_id` 作为 `format_response` 的第一个参数。`ctx.message_id` 继续用于生成 `message_start` / `message_delta` 等 SSE 事件，不受影响。

非流式路径下（`handle_non_stream_request` 里对应的 RESPONSE 日志点）同样替换为 `request_id`。

## Risks / Trade-offs

- [日志读者历史习惯可能把 `msg_xxx` 误认为 Anthropic 客户端看到的 `message.id`] → 两者格式都是 `msg_<hex>`，肉眼难区分。本次仅修复"三行内部对齐"，不改日志字段名。若未来希望区分，可以另起一次变更把日志字段重命名为 `trace_id`，或把 `ctx.message_id` 也加入 RESPONSE 日志作为独立列。
- [非流式路径与流式路径的日志写入点不同] → 两条路径都必须各自覆盖，tasks 里分别列出，避免只改了一条路径。
- [函数参数新增导致改动分散] → 仅两个函数签名变化（`handle_stream_request`、`handle_non_stream_request`）以及 `create_sse_stream` 工厂函数，影响面可控。

## Migration Plan

- 无数据迁移，无对外 API 变更。
- 部署即生效；日志读者可立即基于新的 `request_id` 定位三行日志。
- 回滚策略：`git revert` 本次提交即可恢复原行为，没有持久化状态变化。
