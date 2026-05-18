## ADDED Requirements

### Requirement: Suspicious activity 429 应用长 cooldown

当 Kiro API 返回 HTTP 429 且响应体包含 "suspicious activity" 字符串时，系统 SHALL 对该凭据应用 600 秒 cooldown（而非默认 30 秒），`Retry-After` 响应头优先，上限 600 秒。

#### Scenario: Suspicious activity 429 触发 600 秒 cooldown

- **WHEN** Kiro 返回 429，响应体包含 "suspicious activity"，且无 `Retry-After` 头
- **THEN** 该凭据的 `cooldown_until` SHALL 设置为 `now + 600s`

#### Scenario: Retry-After 头优先于默认值

- **WHEN** Kiro 返回 suspicious activity 429，响应头包含 `Retry-After: 120`
- **THEN** 该凭据的 `cooldown_until` SHALL 设置为 `now + 120s`（使用头部值）

#### Scenario: Retry-After 超过上限时夹紧

- **WHEN** Kiro 返回 suspicious activity 429，响应头包含 `Retry-After: 9999`
- **THEN** 该凭据的 `cooldown_until` SHALL 设置为 `now + 600s`（夹紧到上限）

#### Scenario: 容量不足 429 保持原有 30 秒 cooldown

- **WHEN** Kiro 返回 429，响应体包含 `"reason":"INSUFFICIENT_MODEL_CAPACITY"`，且无 `Retry-After` 头
- **THEN** 该凭据的 `cooldown_until` SHALL 设置为 `now + 30s`（原有行为不变）

### Requirement: Suspicious activity 429 联动失败计数

当 Kiro API 返回 suspicious activity 429 时，系统 SHALL 在标记 cooldown 的同时调用 `report_failure()`，使失败计数递增；3 次后凭据被自动禁用，停止无效重试。

#### Scenario: Suspicious activity 429 递增失败计数

- **WHEN** 凭据 A 收到 suspicious activity 429
- **THEN** 凭据 A 的 `failure_count` SHALL 递增 1，同时标记 600s cooldown

#### Scenario: 3 次 suspicious activity 429 后凭据被禁用

- **WHEN** 凭据 A 连续 3 次收到 suspicious activity 429（考虑时间衰减规则）
- **THEN** 凭据 A SHALL 被自动禁用（`disabled=true`），停止被选择器选中

#### Scenario: 容量不足 429 不递增失败计数

- **WHEN** 凭据 A 收到 INSUFFICIENT_MODEL_CAPACITY 429
- **THEN** 凭据 A 的 `failure_count` SHALL 不变（原有行为不变）

### Requirement: API 路径和 MCP 路径行为一致

Suspicious activity 429 的差异化处理 SHALL 同时覆盖 `call_api_with_retry`（API 路径）和 `call_mcp_with_retry`（MCP 路径），两条路径行为一致。

#### Scenario: MCP 路径 suspicious activity 429 同样触发长 cooldown

- **WHEN** MCP 路径的 Kiro 请求返回 suspicious activity 429
- **THEN** 该凭据的 `cooldown_until` SHALL 设置为 `now + 600s`，`failure_count` 递增 1
