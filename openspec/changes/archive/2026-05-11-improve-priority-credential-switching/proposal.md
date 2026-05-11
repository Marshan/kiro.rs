## Why

当前 priority 模式下,`current_id` 一旦选定就会"粘住"到该凭据耗尽 3 次失败才切换。在凭据数量少(≤5)且 priority 普遍相同的典型部署中,这导致**单张凭据承接 100% 流量,其余全部待命**;同时 429 限速被处理为"同凭据死等 5~60s"而不是"立刻切换到未限速的凭据",402 配额用尽后凭据被永久禁用(尽管 Kiro 的 `getUsageLimits` 已经给出 `nextDateReset` 重置时间),连续失败计数无时间窗口导致深夜偶发抖动跨小时累计。这些问题合在一起,让"多凭据配置"在可用性和吞吐上都没发挥出应有价值。

## What Changes

- priority 模式下,**同优先级凭据按 `(inflight, last_used_at)` 自动轮转**,不再依赖 `current_id` 粘性
- 新增**飞行中请求计数 (inflight)**,通过 RAII Guard 在请求进入/退出时自动增减,避免并发挤兑同一张
- **429 响应立即切换凭据**:将当前凭据标记为短期 cooldown(优先取 `Retry-After` 头,缺省 30s),下一轮选择时跳过;仅当全部凭据都在 cooldown 时退回原退避等待
- **402 配额用尽调用 `getUsageLimits` 捕获 `nextDateReset`**,后台任务每 60s 扫描,到点自动恢复(无需人工 reset)
- **连续失败计数带时间衰减**:`last_failure_at` 距今 > 10 分钟时在累加前减半,避免跨时段累积误禁用
- **Token 刷新失败 30 分钟冷却自愈**,与 API 失败自愈并轨,避免 DNS/网络抖动永久打穿
- **`persist_credentials` 增加 5s debounce**,与 `kiro_stats.json` 写盘对齐,减少多凭据同时刷新 token 时的磁盘争用
- balanced 模式**保留现有行为不变**(后续独立 change 废弃)

## Capabilities

### New Capabilities

- `credential-switching`: 多凭据选择、负载分摊、故障转移、配额自愈的完整行为契约

### Modified Capabilities

(无 — 当前无既存的 credential-switching 规格)

## Impact

- **受影响代码**
  - `src/kiro/token_manager.rs`:`CredentialEntry` 新增字段、`select_next_credential` 排序 key、`acquire_context` 返回 Guard、`report_failure` 衰减、`report_quota_exhausted` 记录 `reset_at`、`report_refresh_failure` 30 分钟冷却、新增后台恢复任务、`persist_credentials` debounce
  - `src/kiro/provider.rs`:`call_api_with_retry` / `call_mcp_with_retry` 的 429 分支改为"标记 cooldown + 立即切换",非 429 退避逻辑不变;持有 Guard 直到响应处理完成
  - `src/main.rs` 或启动路径:在 `MultiTokenManager::new` 后启动后台恢复任务
- **API / 配置**
  - `CredentialEntrySnapshot` 新增 `inflight: u32`、`quotaResetAt: Option<String>`、`cooldownUntil: Option<String>`,Admin API 向后兼容(新增字段)
  - 不引入新的 `config.json` 字段
- **依赖**:无新增 crate
- **向后兼容**:balanced 模式语义不变;priority 模式对"单凭据不变"部署零感知(仅多凭据时行为改善)
