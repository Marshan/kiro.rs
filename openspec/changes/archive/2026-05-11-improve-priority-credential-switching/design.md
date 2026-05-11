## Context

`src/kiro/token_manager.rs` 中的 `MultiTokenManager` 负责多凭据选择、故障转移和 Token 刷新。当前 priority 模式实现有三个已观察到的问题:

1. **单凭据承接全部流量**:`current_id` 粘性 + `min_by_key(priority)` 稳定返回首个,导致同优先级 5 张凭据中 4 张在看戏。
2. **429 死等**:`src/kiro/provider.rs:456` 的 429 分支对当前凭据做 5~60s 指数退避,而非切换到未限速的凭据。即使有 4 张空闲凭据也视而不见。
3. **402 永久禁用**:`report_quota_exhausted` 将凭据标记 `QuotaExceeded` 后无自愈路径,尽管 `getUsageLimits` 响应里 `nextDateReset` 已经提供了账号级月度重置时间。

次要问题:连续失败计数无时间窗口、Token 刷新失败一禁到底、`credentials.json` 频繁写盘。

相关文件:
- `src/kiro/token_manager.rs`(2599 行)
- `src/kiro/provider.rs`(557 行)
- `src/kiro/model/usage_limits.rs`(已解析 `nextDateReset`)

## Goals / Non-Goals

**Goals:**
- 同优先级凭据在 priority 模式下自动轮转,无需手动配置不同 priority
- 429 响应立即尝试别的凭据,只有全部限速时才退回等待
- 402 配额用尽的凭据到 `nextDateReset` 自动恢复,无需人工介入
- 连续失败计数带时间衰减,避免跨时段抖动累计误禁用
- 保持 balanced 模式完全不变(将在独立 change 废弃)
- 对单凭据部署零感知,仅多凭据时行为改善

**Non-Goals:**
- 不重构为 Selector trait 架构(YAGNI,凭据量级 ≤5)
- 不引入 EWMA / 真·负载均衡 / P2C 等复杂算法
- 不修改 `config.json` 字段结构
- 不废弃 balanced 模式(本次仅保留,后续 change 处理)
- 不改变 Admin API 既有路径(仅在 snapshot 中新增 3 个展示字段)
- 不修改 Token 刷新协议本身(Social/IdC)

## Decisions

### 1. 选择算法:三元组排序,不引入新的策略抽象

**选择**:`select_next_credential` 在 priority 模式下改为 `min_by_key(|e| (e.cooldown_until > now, e.inflight, e.credentials.priority, e.last_used_at))`。

排序键四个维度的含义:
- `cooldown_until > now`(bool,false 排前):限速中的凭据降权,但不完全排除
- `inflight`:飞行中请求最少的凭据优先(解决并发挤兑)
- `priority`:原有优先级(数字小者优先)
- `last_used_at`:同优先级下,最久未用的优先(实现轮转)

**为什么不抽 Selector trait**:凭据量级 ≤5,策略只有两种(priority/balanced),trait 会增加间接层但没有扩展收益。改一个函数 10 行代码搞定。

**为什么 cooldown 用降权而不是排除**:如果全部凭据都在 cooldown,"排除"会返回 None → 上层等待;"降权"会返回最早恢复的那张,让上层仍能发一次请求让响应头自然指导下一轮 backoff。

### 2. 飞行中计数:AtomicU32 + RAII Guard

**选择**:`CredentialEntry.inflight: AtomicU32`。`acquire_context` 返回 `(CallContext, InflightGuard)`,Guard 持有 entry 的 `Arc<AtomicU32>`,Drop 时 `fetch_sub(1)`。

**为什么不用 `Mutex<HashMap<u64, u32>>`**:选择器每次请求都会读 inflight,高频操作,原子变量无锁开销。

**Guard 的生命周期**:provider.rs 中 `let _guard = ...` 持有到函数末尾(包括响应体读取)。流式响应需要特别注意:Guard 必须 move 到 stream 处理闭包中,直到 stream 结束才释放。

**并发正确性**:`fetch_add` 发生在 `acquire_context` 返回前,`fetch_sub` 发生在 Guard Drop,成对出现。即使请求 panic,Drop 仍会执行。

### 3. 429 切换语义

**选择**:429 触发后,设置 `entry.cooldown_until = Instant::now() + cooldown_duration`,其中:
- 优先使用响应头 `Retry-After`(如果是秒数)
- 否则默认 30s
- 上限 120s(防止恶意服务端返回超大值)

然后 `continue` 进入下一轮 attempt,让 `select_next_credential` 选别的凭据。

仅当 `select_next_credential` 返回的凭据自身 `cooldown_until > now`(即全部凭据都在 cooldown)时,才退回到 `retry_delay_rate_limited` 的原退避逻辑,对**返回的那张凭据**按其 cooldown 剩余时间等待。

**为什么不删除原退避逻辑**:单凭据部署下(全员 cooldown)仍需退避。保留作为 fallback。

**不修改的行为**:408/5xx/网络错误仍然走 `retry_delay`,因为它们通常是链路/上游本身的问题,换凭据未必能解决。只有 429 是凭据本身被限速的明确信号。

### 4. 配额自愈:`nextDateReset` + 后台扫描

**选择**:
- `report_quota_exhausted` 不再只接受 `id`,而是异步地在 402 处理后调用 `get_usage_limits_for(id)`,把 `usage_limits.next_date_reset` 写入 `entry.quota_reset_at`
- 后台任务 `tokio::spawn` 循环 `sleep(60s)` + 扫描所有 `disabled_reason == QuotaExceeded` 的 entry,`Utc::now() >= quota_reset_at` 则清除 disabled 状态
- 启动位置:`main.rs` 构造完 `Arc<MultiTokenManager>` 后启动,`Arc::downgrade` 持有弱引用避免进程永不退出

**为什么不做概率探活**:`nextDateReset` 给出了确定性答案,探活是多余的 API 调用浪费配额。

**失败时的降级**:如果 `getUsageLimits` 本身失败(网络/上游问题),`quota_reset_at` 保持 `None`,行为退化到现在(永久禁用,等人工 reset)。这是"最坏也不比现在差"。

**重启持久化**:`quota_reset_at` 通过 `canonicalize_auth_method` 路径写回 `credentials.json`(复用现有持久化机制),重启后不丢失。

### 5. 失败计数时间衰减

**选择**:`report_failure` 在 `failure_count += 1` 之前检查 `last_failure_at`:
```
if last_failure_at.is_some() && now - last_failure_at > 10 min:
    failure_count /= 2  // 整数除法,2→1,3→1,1→0
last_failure_at = Some(now)
failure_count += 1
```

**为什么是 10 分钟和除 2**:凭借经验值。10 分钟足够跨过短暂抖动窗口,除 2 让"两次抖动"(相隔 10 分钟以上)不会累积到禁用阈值 3。

**同样应用于 refresh_failure_count**:Token 刷新也可能因网络抖动偶发失败。

### 6. Refresh 失败 30 分钟自愈

**选择**:`TooManyRefreshFailures` 禁用的 entry 写入 `refresh_cooldown_until = now + 30min`,后台恢复任务一并处理。`InvalidRefreshToken`(invalid_grant)**不**自愈,因为它是明确的永久失效信号。

### 7. `persist_credentials` debounce

**选择**:参照 `save_stats_debounced` 模式,`last_persist_at: Mutex<Option<Instant>>` + `credentials_dirty: AtomicBool`。5s 内的多次调用合并,由后台任务或下次调用时机落盘。

**同步点**:进程关闭时强制 flush,确保 `quota_reset_at` 不丢失。(通过 `Drop for MultiTokenManager` 或显式 shutdown hook)

### 8. Admin API 快照字段扩展

**选择**:`CredentialEntrySnapshot` 新增:
- `inflight: u32`(飞行中请求数,用于运维观察)
- `quota_reset_at: Option<String>`(配额恢复时间,RFC3339)
- `cooldown_until: Option<String>`(限速恢复时间,RFC3339)

这三个字段对 Admin UI 向后兼容(新增字段,老 UI 忽略)。Admin UI 侧暂不改,等字段稳定后单独 change 做展示。

## Risks / Trade-offs

- **Guard 忘记持有导致计数泄漏**:如果 provider.rs 里 `let _ = guard` 或没接住 Guard,`inflight` 不会 Drop 触发减一 → 计数单调增 → 该凭据永远看起来"最忙"被跳过。
  → 缓解:Guard 设计为 `#[must_use]`;并发测试断言 Drop 后 inflight 归零。

- **后台恢复任务 panic 导致配额永不自愈**:如果 `getUsageLimits` 调用 panic 或 task 被 abort。
  → 缓解:任务体内 `std::panic::catch_unwind` 或每个 iteration 独立 `tokio::spawn`;启动失败时 tracing 明确告警。

- **quota_reset_at 来自上游不可信**:如果 Kiro 返回一个遥远的未来时间或已经过去的时间。
  → 缓解:夹紧到 `[now, now + 45 days]`。

- **429 cooldown 放大单凭据部署延迟**:单凭据部署下,原逻辑在当前凭据上等待;新逻辑会先"切一轮"发现没别的凭据再退回等待。多一次 select 开销。
  → 缓解:select 是内存操作 O(n),n≤5,微秒级,实际无感。

- **失败计数衰减可能掩盖真实故障**:如果某凭据每 11 分钟失败一次,永远不会禁用。
  → 缓解:每小时失败 ~5 次的上游问题,运维从 success/failure 比例能观察到;不是禁用机制该解决的问题。

- **balanced 模式与新字段的交互**:balanced 模式不读 `inflight`,但 `inflight` 仍然会被写入(因为 acquire_context 是统一入口)。这是期望的,balanced 将来也能受益;如果未来废弃 balanced,只需删 `select_next_credential` 里的 balanced 分支。

- **持久化 debounce 导致崩溃丢失 quota_reset_at**:5s 窗口内进程被 kill,刚记录的 reset 时间丢失。
  → 缓解:可接受。丢失后行为回退到"盲目自愈"(下次请求触发 402 再重新记录);比永久禁用好得多。

## Migration Plan

- **部署**:单次部署完成,无数据迁移。
- **回滚**:若新逻辑有缺陷,可回退到上一个版本;`credentials.json` 多出的 `quotaResetAt`/`cooldownUntil` 字段老版本会在反序列化时忽略(`#[serde(default)]` 保护)。
- **观察指标**:部署后观察 `tracing` 日志的"切换到凭据 #X"频率、"凭据 #X 配额已到重置时间,自动恢复"事件、以及 Admin UI `inflight` 分布是否均匀。

## Open Questions

1. **后台恢复任务扫描周期 60s** 是否合理?更短会增加锁争用,更长会让恢复延迟 — 暂定 60s,部署观察。
2. **Admin UI 是否同步展示 `inflight` / `quotaResetAt` / `cooldownUntil`** — 本次 change 不改 UI,字段先落 API。
