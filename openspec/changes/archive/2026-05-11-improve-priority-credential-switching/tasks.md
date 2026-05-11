## 1. 数据结构扩展

- [x] 1.1 在 `src/kiro/model/credentials.rs` 的 `KiroCredentials` 新增可选字段 `quota_reset_at: Option<String>` 和 `refresh_cooldown_until: Option<String>`(RFC3339 字符串,`#[serde(default, skip_serializing_if = "Option::is_none")]` 保持兼容)
- [x] 1.2 在 `src/kiro/token_manager.rs::CredentialEntry` 新增字段:`inflight: Arc<AtomicU32>`、`last_failure_at: Option<Instant>`、`last_refresh_failure_at: Option<Instant>`、`cooldown_until: Option<Instant>`、`quota_reset_at: Option<DateTime<Utc>>`、`refresh_cooldown_until: Option<DateTime<Utc>>`
- [x] 1.3 在 `MultiTokenManager::new` 构造 entry 时初始化新字段(从 `KiroCredentials` 读取持久化的 `quota_reset_at` / `refresh_cooldown_until`)
- [x] 1.4 在 `persist_credentials` 同步 entry 的 `quota_reset_at`/`refresh_cooldown_until` 回写到 `KiroCredentials`

## 2. RAII Guard 与 inflight 计数

- [x] 2.1 新增 `pub struct InflightGuard { inflight: Arc<AtomicU32>, credential_id: u64 }`,`impl Drop` 中 `fetch_sub(1, Ordering::Release)`,`#[must_use]` 标注
- [x] 2.2 修改 `MultiTokenManager::acquire_context` 签名返回 `(CallContext, InflightGuard)`,在 entry 定位后 `inflight.fetch_add(1, Ordering::Acquire)` 并构造 Guard
- [x] 2.3 修改 `src/kiro/provider.rs::call_api_with_retry` 和 `call_mcp_with_retry`:调用处改为 `let (ctx, _guard) = ...`,`_guard` 持有到循环体末尾/请求完成(流式响应需 move 到响应处理)
- [x] 2.4 检查 `src/anthropic/stream.rs` 流式响应路径,确认 Guard 正确跨越异步边界(若需要,附加到 stream state)

## 3. 选择算法重写

- [x] 3.1 在 `select_next_credential` 的 priority 分支,将排序 key 改为 `(e.cooldown_until.map(|t| t > now).unwrap_or(false), e.inflight.load(Ordering::Relaxed), e.credentials.priority, e.last_used_at.clone())`
- [x] 3.2 balanced 分支保持现状不变;补注释说明"未来统一淘汰"
- [x] 3.3 `acquire_context` 去除 "balanced 才每次重选,priority 才吃 current_id" 的分叉 — priority 模式也改为每次调用 `select_next_credential` 选择(current_id 仍然维护用于 Admin UI 当前指针展示,但不影响选择)
- [x] 3.4 确保 `cooldown_until` 过期(`t <= now`)时自动清除字段,避免脏数据

## 4. 429 cooldown 切换

- [x] 4.1 在 `provider.rs::call_api_with_retry` 的 429 分支:解析 `Retry-After`(秒),`cooldown_secs = retry_after_secs.unwrap_or(30).min(120)`
- [x] 4.2 调用新增的 `MultiTokenManager::mark_cooldown(id, Duration::from_secs(cooldown_secs))` 设置 `entry.cooldown_until = Some(Instant::now() + duration)`
- [x] 4.3 继续 `continue` 到下一轮 attempt,**不 sleep**;下一轮 select 会自动跳过仍在 cooldown 的凭据
- [x] 4.4 引入"全员 cooldown"兜底:若 `select_next_credential` 返回的凭据自身仍 `cooldown_until > now`,则对其剩余 cooldown 执行 sleep(替代原 `retry_delay_rate_limited`)
- [x] 4.5 `call_mcp_with_retry` 的 429 分支做同样改造

## 5. 402 配额自愈

- [x] 5.1 修改 `report_quota_exhausted` 签名接受 `quota_reset_at: Option<DateTime<Utc>>` 参数,写入 entry
- [x] 5.2 在 `provider.rs` 的 402 处理分支:调用 `self.token_manager.fetch_and_record_quota_reset(ctx.id).await`(新增 helper),内部调用 `get_usage_limits_for(id)` 获取 `next_date_reset`,夹紧到 `[now, now + 45d]`,再调用 `report_quota_exhausted(id, Some(reset_at))`
- [x] 5.3 getUsageLimits 失败时 fallback 到 `report_quota_exhausted(id, None)`,保持现有"永久禁用"语义
- [x] 5.4 `canonicalize` 和 `persist` 路径确保 `quotaResetAt` 序列化到 `credentials.json`

## 6. 失败计数时间衰减

- [x] 6.1 `report_failure` 在 `failure_count += 1` 前:若 `last_failure_at.is_some() && now - last_failure_at > Duration::from_secs(600)`,则 `failure_count /= 2`
- [x] 6.2 更新 `last_failure_at = Some(Instant::now())`
- [x] 6.3 对 `report_refresh_failure` 做相同处理(使用 `last_refresh_failure_at`)

## 7. Refresh 失败 30 分钟冷却

- [x] 7.1 在 `report_refresh_failure` 触发禁用时,额外写入 `entry.refresh_cooldown_until = Some(Utc::now() + chrono::Duration::minutes(30))`
- [x] 7.2 `InvalidRefreshToken` 禁用路径不设置 `refresh_cooldown_until`(永久失效)

## 8. 后台恢复任务

- [x] 8.1 实现 `MultiTokenManager::spawn_recovery_task(self: &Arc<Self>)`:创建 `Arc::downgrade` 弱引用,`tokio::spawn` 循环 `sleep(Duration::from_secs(60))`
- [x] 8.2 任务体内调用 `try_recover_expired_cooldowns()`:扫描所有 entry,对 `disabled_reason == QuotaExceeded && quota_reset_at <= now` 或 `disabled_reason == TooManyRefreshFailures && refresh_cooldown_until <= now` 的 entry,清除 `disabled`、`disabled_reason`、相关计数器,记录 INFO 日志
- [x] 8.3 在 `main.rs`(或 `src/server.rs` 启动路径)构造完 `Arc<MultiTokenManager>` 后调用 `spawn_recovery_task()`
- [x] 8.4 任务体内使用 `std::panic::AssertUnwindSafe` + `catch_unwind` 避免单次异常杀死整个任务(或每次 iteration 独立容错)

## 9. persist_credentials debounce

- [x] 9.1 新增 `last_persist_at: Mutex<Option<Instant>>` + `credentials_dirty: AtomicBool` 到 `MultiTokenManager`
- [x] 9.2 封装 `persist_credentials_debounced`:设置 dirty,若距上次写入 > 5s 则立即写并清 dirty;否则返回(后台任务兜底)
- [x] 9.3 后台恢复任务(或新增单独的 flusher task)在每次 iteration 检查 dirty,若 > 5s 未落盘则 flush
- [x] 9.4 在关闭 hook(如 `Drop for MultiTokenManager` 或 main 的 signal handler)强制 flush
- [x] 9.5 把所有 `persist_credentials()` 调用点换成 `persist_credentials_debounced()`(除明确需要同步的场景,如添加新凭据)

## 10. Admin API 快照字段

- [x] 10.1 `CredentialEntrySnapshot` 新增 `inflight: u32`、`quota_reset_at: Option<String>`、`cooldown_until: Option<String>`(camelCase 序列化,`skip_serializing_if = "Option::is_none"`)
- [x] 10.2 `snapshot()` 方法填充新字段:`cooldown_until` 用 `Instant` 换算回 `DateTime<Utc>`(粗略:`Utc::now() + (instant - Instant::now())`)

## 11. 单元测试

- [x] 11.1 `test_select_same_priority_rotates_by_last_used_at`:3 张同 priority 串行请求,验证轮转
- [x] 11.2 `test_select_balances_by_inflight`:模拟 5 张同 priority,人工 fetch_add inflight,验证选最小的那张
- [x] 11.3 `test_inflight_guard_decrements_on_drop`:acquire + drop,验证 inflight 回 0
- [x] 11.4 `test_429_cooldown_skips_rate_limited_credential`:A 标 cooldown,下次选择应跳过 A 选 B
- [x] 11.5 `test_429_all_cooldown_waits_on_earliest`:全部 cooldown,验证选到"恢复最早"的那张
- [x] 11.6 `test_quota_reset_at_recovers_credential`:禁用 entry.quota_reset_at=past,调 `try_recover_expired_cooldowns`,验证恢复
- [x] 11.7 `test_quota_reset_at_clamped_to_45d`:模拟 `get_usage_limits` 返回 100 天后,验证被夹紧
- [x] 11.8 `test_failure_count_decays_after_10min`:failure_count=2 + last_failure 11 分钟前 + 再失败 → 最终为 2
- [x] 11.9 `test_refresh_cooldown_30min_auto_recover`:手工 refresh_cooldown_until=past,验证恢复
- [x] 11.10 `test_invalid_refresh_token_never_recovers`:InvalidRefreshToken 禁用,扫描不恢复
- [x] 11.11 `test_persist_debounce_coalesces_writes`:短时间内 3 次 persist,验证只写 1 次

## 12. 验证与清理

- [x] 12.1 `cargo clippy --all-targets -- -D warnings`
- [x] 12.2 `cargo test`(全量)
- [x] 12.3 `cargo build --release`
- [x] 12.4 手工验证:启动 binary,确认日志里能看到 "启动后台恢复任务"、正常请求不报错
- [x] 12.5 `openspec validate improve-priority-credential-switching --strict`

