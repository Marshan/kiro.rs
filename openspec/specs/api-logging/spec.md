# api-logging Specification

## Purpose

Defines the behavior of the local API call logging subsystem for the Anthropic-compatible proxy, including how `request_id` correlates log lines across the Anthropic compatibility layer and the Kiro upstream, and how log identifiers relate to client-visible SSE identifiers.

## Requirements

### Requirement: 同一请求的三行日志共用 request_id

同一次 Anthropic 兼容层入站请求，在本地 API 日志中产生的 REQUEST、KIRO REQUEST、RESPONSE 三行日志的 `request_id` 字段 MUST 完全一致。

#### Scenario: 流式请求日志三行对齐
- **WHEN** 一个流式 Anthropic 请求成功调用 Kiro 并正常结束
- **THEN** 该请求产生的 REQUEST 行、KIRO REQUEST 行、RESPONSE 行的 `request_id` 字段值必须相同

#### Scenario: 非流式请求日志三行对齐
- **WHEN** 一个非流式 Anthropic 请求成功调用 Kiro 并返回
- **THEN** 该请求产生的 REQUEST 行、KIRO REQUEST 行、RESPONSE 行的 `request_id` 字段值必须相同

#### Scenario: 并发请求日志互不串扰
- **WHEN** 两个入站请求几乎同时到达
- **THEN** 每个请求各自的三行日志之间使用自己的 `request_id`，两个请求的 `request_id` 值不得相等

### Requirement: KIRO REQUEST 日志不得使用占位符 request_id

KIRO REQUEST 日志的 `request_id` 字段 MUST 在写入时已取得本次请求的真实追踪 ID，不得写入 `"pending"` 等占位值。

#### Scenario: KIRO REQUEST 输出真实 request_id
- **WHEN** 代理写入 KIRO REQUEST 日志
- **THEN** 该行的 `request_id` 字段与同一请求 REQUEST 行的 `request_id` 相同，且不为字面量 `pending`

### Requirement: 客户端可见的 SSE message.id 与日志 request_id 解耦

日志 `request_id` 的变更 MUST NOT 影响返回给客户端的 Anthropic SSE `message_start.message.id` 及相关事件字段。`StreamContext.message_id` 继续作为对外 API 契约的一部分独立生成。

#### Scenario: SSE message.id 仍由 StreamContext 生成
- **WHEN** 流式请求产生 SSE `message_start` 事件
- **THEN** 事件中的 `message.id` 值由 `StreamContext` 内部生成，与日志 `request_id` 允许不同

### Requirement: 日志字段格式保持向后兼容

`ApiLogger` 输出的文本模板（字段名、顺序、列宽）MUST 保持与本次变更前一致，仅 `request_id` 字段的取值来源改变。

#### Scenario: 日志模板未变化
- **WHEN** 读取新的 REQUEST / KIRO REQUEST / RESPONSE 日志行
- **THEN** 三行的表头、字段名、字段顺序与变更前完全一致

### Requirement: 请求入口日志包含 kiro_model 字段

`handlers.rs` 的 "Received POST /v1/messages request" 日志 SHALL 包含 `kiro_model` 字段，值为 Anthropic 模型名映射后的实际 Kiro 模型标识符。

#### Scenario: 入口日志包含 kiro_model

- **WHEN** 客户端发送 POST /v1/messages，请求模型为 "opus 4.6"
- **THEN** 日志行 SHALL 包含 `kiro_model=claude-opus-4.6` 字段

#### Scenario: 无法映射时 kiro_model 回退到原始模型名

- **WHEN** 客户端发送未知模型名（如 "unknown-model"），`map_model` 返回 None
- **THEN** 日志行 SHALL 包含 `kiro_model=unknown-model`（使用原始模型名）

### Requirement: 凭据调度日志

`provider.rs` SHALL 在每次凭据选定后（`acquire_context` 返回后）写入一条 INFO 日志，包含凭据编号（`credential_id`）和实际 Kiro 模型名（`kiro_model`）。

#### Scenario: API 路径凭据调度日志

- **WHEN** API 路径选定凭据 #8 处理 claude-opus-4.6 请求
- **THEN** 日志 SHALL 包含 `credential_id=8`、`kiro_model=claude-opus-4.6`，消息格式为 "凭据 #N 开始请求"

#### Scenario: MCP 路径凭据调度日志

- **WHEN** MCP 路径选定凭据 #9 处理请求
- **THEN** 日志 SHALL 包含 `credential_id=9` 和 `kiro_model` 字段

### Requirement: 429 错误日志包含凭据编号和 kiro_model

`provider.rs` 的 429 WARN 日志 SHALL 包含 `credential_id` 和 `kiro_model` 字段，使日志读者无需推断即可知道是哪个凭据在报错以及请求的是哪个模型。

#### Scenario: API 路径 429 日志包含凭据信息

- **WHEN** 凭据 #9 的 API 请求返回 429
- **THEN** WARN 日志 SHALL 包含 `credential_id=9` 和 `kiro_model=<实际模型名>` 字段

#### Scenario: MCP 路径 429 日志包含凭据信息

- **WHEN** MCP 路径凭据 #2 的请求返回 429
- **THEN** WARN 日志 SHALL 包含 `credential_id=2` 和 `kiro_model=<实际模型名>` 字段
