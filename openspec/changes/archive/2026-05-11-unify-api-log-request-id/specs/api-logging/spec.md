## ADDED Requirements

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
