## ADDED Requirements

### Requirement: Priority 模式同优先级凭据自动轮转

在 priority 负载均衡模式下,系统 SHALL 在每次获取凭据时按 `(cooldown_active, inflight_count, priority, last_used_at)` 四元组排序选择,使相同 `priority` 的多张凭据自动轮转使用,不再依赖 `current_id` 粘性。

#### Scenario: 同优先级多张凭据按飞行中请求数分散

- **WHEN** 系统配置了 5 张 priority 均为 0 的凭据,10 个并发请求同时到达
- **THEN** 10 个请求应按当前飞行中请求数 (inflight) 最少的原则分散到这 5 张凭据,而不是全部打到第一张

#### Scenario: 同优先级凭据按最近使用时间轮转

- **WHEN** 3 张 priority 均为 0 的凭据依次接收串行请求,且 inflight 始终相同
- **THEN** 下一次请求 SHALL 选择 `last_used_at` 最早的那张(含 `None`),使三张凭据轮流承接流量

#### Scenario: 不同优先级仍按 priority 选择

- **WHEN** 配置中一张凭据 priority=0,另一张 priority=1
- **THEN** 系统 SHALL 始终优先选择 priority=0 的凭据,仅当其不可用时才使用 priority=1

### Requirement: 飞行中请求计数与自动释放

系统 SHALL 维护每张凭据的"飞行中请求数"(`inflight`) 为原子计数器,在 `acquire_context` 返回调用上下文时自增 1,在调用方持有的 Guard 被 Drop(无论成功、失败或 panic)时自减 1,使选择器能看到实时负载。

#### Scenario: 请求结束后计数归零

- **WHEN** 一张凭据连续接收并完成 N 个请求
- **THEN** 所有请求完成后,该凭据的 inflight 应回到 0

#### Scenario: 请求 panic 不导致计数泄漏

- **WHEN** 调用方在持有 Guard 期间 panic
- **THEN** 凭据的 inflight 仍 SHALL 正确减 1(由 RAII Drop 保证)

### Requirement: 429 限速立即切换

当 Kiro API 返回 HTTP 429 时,系统 SHALL 将当前凭据标记为短期 cooldown 状态(cooldown_until),立即重试并尝试选择其他凭据,仅当所有凭据均处于 cooldown 时才对返回的凭据执行等待退避。

#### Scenario: 有未限速凭据时立即切换

- **WHEN** 当前凭据 A 返回 429,凭据 B/C/D/E 未处于 cooldown
- **THEN** 系统 SHALL 在下一次尝试中选择 B/C/D/E 中的一张,不对 A 执行 `sleep`

#### Scenario: 所有凭据均限速时退回等待

- **WHEN** 所有凭据均处于 cooldown 状态
- **THEN** 系统 SHALL 对选择器返回的凭据(恢复时间最早的那张)按 `Retry-After` 或默认退避时间 sleep

#### Scenario: cooldown 时长优先使用 Retry-After 响应头

- **WHEN** 429 响应头包含 `Retry-After: 15`
- **THEN** 该凭据的 cooldown_until SHALL 设置为 `now + 15s`,上限 120s

#### Scenario: 无 Retry-After 时使用默认 30 秒

- **WHEN** 429 响应不包含 `Retry-After` 头
- **THEN** 该凭据的 cooldown_until SHALL 设置为 `now + 30s`

### Requirement: 配额用尽自动恢复

当 Kiro API 返回 HTTP 402 且消息为 `MONTHLY_REQUEST_COUNT` 时,系统 SHALL 调用 `getUsageLimits` 获取 `nextDateReset`,记录到凭据的 `quota_reset_at` 字段,并由后台恢复任务在到点后自动重启该凭据,无需人工干预。

#### Scenario: 记录 nextDateReset 并在到点自动恢复

- **WHEN** 凭据 A 返回 402 MONTHLY_REQUEST_COUNT,`getUsageLimits` 响应 `nextDateReset = 2026-06-01T00:00:00Z`
- **THEN** 凭据 A 被标记 `disabled=true, disabled_reason=QuotaExceeded, quota_reset_at=2026-06-01T00:00:00Z`
- **AND WHEN** 到达该时间后下一次后台扫描触发
- **THEN** 凭据 A SHALL 被重新启用,`disabled=false, disabled_reason=None`,失败计数清零

#### Scenario: getUsageLimits 调用失败不阻塞降级

- **WHEN** 402 后调用 `getUsageLimits` 失败(网络/上游错误)
- **THEN** 凭据仍 SHALL 被禁用,`quota_reset_at` 保持 `None`,后台任务不对其尝试恢复(行为退化到现有"永久禁用"语义)

#### Scenario: 上游返回异常的重置时间被夹紧

- **WHEN** `nextDateReset` 返回一个早于当前时间或远超 45 天的未来时间
- **THEN** 系统 SHALL 将 `quota_reset_at` 夹紧到 `[now, now + 45 days]` 区间内

#### Scenario: 后台恢复任务默认 60 秒扫描一次

- **WHEN** 系统启动并构造完 `MultiTokenManager`
- **THEN** SHALL 启动一个后台 tokio 任务,每 60 秒扫描所有禁用凭据,将 `quota_reset_at <= now` 或 `refresh_cooldown_until <= now` 的凭据重新启用

### Requirement: 连续失败计数时间衰减

系统 SHALL 在每次累加 `failure_count` 或 `refresh_failure_count` 之前,检查距离上次失败的时间。若距离上次失败超过 10 分钟,则先将失败计数整数除以 2 再累加 1,避免跨时段偶发抖动累计导致误禁用。

#### Scenario: 超过 10 分钟后失败计数衰减

- **WHEN** 凭据 A 在 00:00 失败 2 次(failure_count=2),接着在 00:15 再次失败
- **THEN** 凭据 A 在 00:15 的 failure_count SHALL 为 2(衰减: 2/2=1,再 +1=2),不触发禁用

#### Scenario: 10 分钟内连续失败仍累加

- **WHEN** 凭据 A 在 00:00 失败 2 次(failure_count=2),接着在 00:05 再次失败
- **THEN** 凭据 A 在 00:05 的 failure_count SHALL 为 3,触发禁用

### Requirement: Refresh 失败 30 分钟冷却自愈

当凭据因连续 Token 刷新失败(`TooManyRefreshFailures`)被禁用时,系统 SHALL 记录 `refresh_cooldown_until = now + 30min`,并由后台恢复任务到点后自动重新启用;`invalid_grant` 导致的永久失效(`InvalidRefreshToken`)不自愈。

#### Scenario: 刷新失败冷却到期自动恢复

- **WHEN** 凭据 A 连续 3 次刷新失败,被禁用(`TooManyRefreshFailures`),记录 `refresh_cooldown_until = now + 30min`
- **AND WHEN** 到达冷却时间,后台任务触发扫描
- **THEN** 凭据 A SHALL 被重新启用,`refresh_failure_count=0`

#### Scenario: invalid_grant 永久失效不自愈

- **WHEN** 凭据 A 刷新时上游返回 `invalid_grant`,标记 `InvalidRefreshToken`
- **THEN** 后台恢复任务 SHALL 不对其尝试恢复,该凭据保持禁用直至管理员手动处理

### Requirement: credentials.json 写盘防抖

系统 SHALL 对 `persist_credentials` 增加 5 秒防抖窗口,5 秒内的多次调用合并为一次磁盘写入,减少多凭据同时刷新 Token 时的 I/O 争用;进程正常关闭时 SHALL 强制刷盘,防止 `quota_reset_at` 等关键字段丢失。

#### Scenario: 短时间内多次持久化合并

- **WHEN** 5 秒内触发 3 次 `persist_credentials` 调用
- **THEN** 磁盘 SHALL 仅被写入一次(最后一次调用的内容)

#### Scenario: 进程关闭强制刷盘

- **WHEN** 进程收到关闭信号
- **THEN** 待落盘的凭据数据 SHALL 被同步写入磁盘

### Requirement: Admin API 快照暴露新状态字段

`CredentialEntrySnapshot` SHALL 新增 `inflight`、`quotaResetAt`、`cooldownUntil` 三个字段,供 Admin API 消费者观察凭据实时状态;旧字段保持兼容。

#### Scenario: 快照包含新字段

- **WHEN** 调用 Admin API 获取凭据快照
- **THEN** 响应 SHALL 包含 `inflight: u32`、`quotaResetAt: Option<String RFC3339>`、`cooldownUntil: Option<String RFC3339>` 字段

#### Scenario: 字段缺省值向后兼容

- **WHEN** 凭据从未被请求过
- **THEN** `inflight` SHALL 为 0,`quotaResetAt` 和 `cooldownUntil` SHALL 为 `null`

### Requirement: Balanced 模式行为保持不变

Balanced 模式的选择算法 SHALL 保持现有 `min_by_key((success_count, priority))` 行为不变,不受本次 priority 模式改动影响;`inflight` 字段虽然会被统一写入,但 balanced 分支不读取。

#### Scenario: Balanced 模式选择结果未改变

- **WHEN** `loadBalancingMode = "balanced"`,凭据 A success_count=5,凭据 B success_count=10
- **THEN** 系统 SHALL 选择凭据 A(与本次改动前的行为一致)
