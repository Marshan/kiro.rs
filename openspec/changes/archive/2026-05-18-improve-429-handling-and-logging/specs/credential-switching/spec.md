## MODIFIED Requirements

### Requirement: 429 限速立即切换

当 Kiro API 返回 HTTP 429 时，系统 SHALL 根据响应体内容区分两种类型并差异化处理：

- **容量不足**（`reason: "INSUFFICIENT_MODEL_CAPACITY"`）：将当前凭据标记为 30 秒 cooldown（`Retry-After` 头优先，上限 120 秒），不递增失败计数，立即重试并尝试选择其他凭据
- **可疑活动封禁**（响应体包含 "suspicious activity"）：将当前凭据标记为 600 秒 cooldown（`Retry-After` 头优先，上限 600 秒），同时调用 `report_failure()` 递增失败计数，立即重试并尝试选择其他凭据

仅当所有凭据均处于 cooldown 时才对返回的凭据执行等待退避。

#### Scenario: 有未限速凭据时立即切换

- **WHEN** 当前凭据 A 返回 429，凭据 B/C/D/E 未处于 cooldown
- **THEN** 系统 SHALL 在下一次尝试中选择 B/C/D/E 中的一张，不对 A 执行 `sleep`

#### Scenario: 所有凭据均限速时退回等待

- **WHEN** 所有凭据均处于 cooldown 状态
- **THEN** 系统 SHALL 对选择器返回的凭据（恢复时间最早的那张）按 `Retry-After` 或默认退避时间 sleep

#### Scenario: 容量不足 cooldown 时长优先使用 Retry-After 响应头

- **WHEN** 容量不足 429 响应头包含 `Retry-After: 15`
- **THEN** 该凭据的 cooldown_until SHALL 设置为 `now + 15s`，上限 120s

#### Scenario: 容量不足无 Retry-After 时使用默认 30 秒

- **WHEN** 容量不足 429 响应不包含 `Retry-After` 头
- **THEN** 该凭据的 cooldown_until SHALL 设置为 `now + 30s`

#### Scenario: 可疑活动封禁无 Retry-After 时使用默认 600 秒

- **WHEN** 可疑活动 429 响应不包含 `Retry-After` 头
- **THEN** 该凭据的 cooldown_until SHALL 设置为 `now + 600s`，failure_count 递增 1

#### Scenario: 可疑活动封禁 3 次后凭据自动禁用

- **WHEN** 凭据 A 连续 3 次收到可疑活动 429（考虑时间衰减规则）
- **THEN** 凭据 A SHALL 被自动禁用，停止被选择器选中
