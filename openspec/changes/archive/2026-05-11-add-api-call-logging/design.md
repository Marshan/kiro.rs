## Context

kiro-rs 是一个 Anthropic API 兼容代理，将客户端请求转换后发给 Kiro 后端。现有日志基于 `tracing` crate，输出到 stdout，通过 `RUST_LOG` 环境变量控制级别。

需要新增一套独立的 API 调用日志，记录每次请求的完整入参出参，写入独立文件，通过 `config.json` 开关控制，不干扰现有 tracing 日志。

关键数据流：
```
post_messages()          ← 记录点1：客户端入参（model_in, messages）
  → convert_request()   ← 得到 model_kiro（映射后模型名）
  → provider.call_api() ← 记录点2：Kiro 请求体 + URL
  → stream/response     ← 记录点3：完整响应（content, tokens, stop_reason, 耗时）
```

## Goals / Non-Goals

**Goals:**
- 每次请求写入三段日志：客户端入参、Kiro 请求、响应出参
- `model_in`（原始）和 `model_kiro`（映射后）作为独立字段记录
- 完整记录 messages 内容和响应文本
- 通过 `apiLogEnabled` / `apiLogPath` 配置控制，默认关闭
- 不阻塞请求路径（异步写入）
- 不影响现有 tracing 日志

**Non-Goals:**
- 日志轮转（log rotation）
- 结构化 JSON 格式（人类可读文本即可）
- WebSearch 请求的详细日志（可后续扩展）

## Decisions

**Decision: tokio channel 异步写入，不用 Mutex**

日志写入用 `tokio::sync::mpsc` channel，后台 task 负责实际写文件。好处：写日志不阻塞请求路径，即使磁盘慢也不影响响应延迟。`ApiLogger` 持有 `Sender<String>`，调用方只需 `send`，不等待。

**Decision: `Arc<Option<ApiLogger>>` 注入 AppState**

`ApiLogger` 包在 `Option` 里，`apiLogEnabled=false` 时为 `None`，调用点只需 `if let Some(logger) = &state.api_logger`，零开销。`Arc` 保证 Clone 时共享同一实例。

**Decision: `output_text` 字段加入 `StreamContext`**

流式响应的完整输出文本需要在流处理过程中累积。在 `StreamContext.create_text_delta_events()` 里同步追加到 `output_text: String`。字段始终存在（不用 feature flag），但只在 api_logger 存在时才被读取，内存开销可接受。

**Decision: request_id 复用 `message_id`**

`StreamContext` 已有 `message_id`（`msg_xxx` 格式 UUID），直接复用作为三段日志的关联 ID，不需要额外生成。

**Decision: 日志格式**

每段日志用 `──` 分隔线 + 标题，字段用固定宽度对齐，messages 内容截断到前 500 字符（避免超长消息撑爆日志文件）。

```
[2026-05-11 14:23:01.234] ── REQUEST ──────────────────────────────────
  request_id : msg_abc123
  model_in   : claude-opus-4-7-20250514
  model_kiro : claude-opus-4.7
  stream     : true
  max_tokens : 8192
  messages   : [2 messages]
    [0] user    : "帮我写一个 Rust 函数..."
    [1] assistant: "好的，这里是..."
```

## Risks / Trade-offs

- **[Risk] output_text 内存占用** → 长对话的完整输出可能占用较多内存。缓解：只在流结束时读取一次，之后 StreamContext 被 drop。
- **[Risk] channel 满导致日志丢失** → mpsc channel 用有界 buffer（1024），满时 `try_send` 失败静默丢弃，不阻塞请求。日志丢失比阻塞请求更可接受。
- **[Trade-off] messages 截断** → 截断到 500 字符可能丢失上下文，但避免日志文件过大。截断长度可后续做成可配置。
