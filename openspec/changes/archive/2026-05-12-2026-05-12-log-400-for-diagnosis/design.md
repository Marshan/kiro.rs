## Context

日志样本（2026-05-11 23:13–23:15，客户端为 Claude Code）：

```
23:13:11 POST /v1/messages ... message_count=215
23:13:13 ERROR Kiro API 调用失败: 流式 API 请求失败: 400 Bad Request
         {"message":"Invalid model. Please select a different model to continue.","reason":"INVALID_MODEL_ID"}
23:13:13 POST /v1/messages ... (+2s)    → 400
23:13:15 POST /v1/messages ... (+2s)    → 400
23:13:18 POST /v1/messages ... (+3s)    → 400
23:13:23 POST /v1/messages ... (+5s)    → 400
23:13:33 POST /v1/messages ... (+10s)   → 400
23:13:51 POST /v1/messages ... (+18s)   → 400
23:14:28 POST /v1/messages ... (+37s)   → 400
```

根据用户确认："凭据本身之前能用之后也能用"，排除"该凭据永久不支持 opus-4-7"。残留解释空间收敛为两条：

- **(a)** 上游以非标准状态码表达瞬态状态：计费系统同步延迟、模型路由后端过载、软限流、**地域风控**（出口 IP/ASN 被临时拦截）等；
- **(b)** 本项目 bug：converter 在长上下文（`message_count=215`）或特定 profileArn/endpoint 路径下构造出 Kiro 不认的请求。

`src/kiro/provider.rs:442-444`（优化后变为 `:449-461`）对 400 的处理是 `bail!()` 直接退出，不触发 `report_failure` / `mark_cooldown` / `last_used_at` 更新。`cd8fb61` 开启的同优先级四元组选择器（`src/kiro/token_manager.rs:841-860`）依赖这些状态字段轮转；400 不打标记意味着"选到谁就一路打到退避耗尽"。

当前日志仅能看到错误文本，缺失 `credential_id` / `endpoint` / `profile_arn`，无法在日志层把 (a) 和 (b) 分离。后续任何行为层修复（如把 `INVALID_MODEL_ID` 按瞬态退避处理、或定位请求构造 bug）都依赖这一次现场证据。

## Goals / Non-Goals

**Goals:**
- 下次再复现 400 `INVALID_MODEL_ID` 时，仅凭日志即可回答：8 次连续 400 是同一张凭据还是多张？endpoint 是否错配？profile_arn 是否为预期值？
- 结构化字段固定命名，便于 `grep credential_id=` / `grep endpoint=` 直接聚合。

**Non-Goals:**
- 不改 400 的重试/退避/切换语义（把 400 变成可重试属于下一次变更的讨论范围）。
- 不改 `handlers.rs` 的 `map_provider_error`（仍返回 502 给客户端）。
- 不新增 tracing span 或引入更大范围观测性重构。
- 不暴露 `profile_arn` 全值之外的变体（例如 hash）；当前默认端点 `ide` 的 profile_arn 仅在本地 credentials.json 可见，直接打印有利于排查且未引入新暴露面。

## Decisions

### 1. 日志落点：紧贴 `bail!` 之前，保留两处（API + MCP）

`src/kiro/provider.rs` 有两条独立 400 分支：`call_api_with_retry`（正式 API 流/非流）与 `call_mcp_with_retry`（WebSearch 等 MCP 调用）。两者都保留，字段命名完全一致，仅"消息"前缀区分 `API` / `MCP`。

**为什么不把日志统一到 `handlers.rs::map_provider_error`？**
- `map_provider_error` 只拿到 `anyhow::Error` 的字符串，无法访问 `ctx.id`、`endpoint.name()`、`ctx.credentials.profile_arn`。硬塞进 `anyhow::anyhow!` 的消息会降低可解析度（要从字符串反解字段）。
- 400 失败频度低、日志量可接受，就地打点最直接。

### 2. 字段选择

| 字段 | 来源 | 作用 |
|---|---|---|
| `credential_id` | `ctx.id` (u64) | 区分"同一张凭据连续 400"（倾向上游瞬态/风控）vs "多张凭据都 400"（倾向项目 bug） |
| `endpoint` | `endpoint.name()` 返回 `&'static str` | 定位 `ide` / 未来其它 endpoint 与凭据的错配 |
| `profile_arn` | `ctx.credentials.profile_arn.as_deref().unwrap_or("<none>")` | 核查历史 `53df562` 修过的"切换后携带过期 ARN"是否再次出现 |
| `status` | `%status` | `reqwest::StatusCode` 的 Display 格式，包含 "400 Bad Request" |
| `body` | `%body` | 包含 `{"message":"...","reason":"INVALID_MODEL_ID"}`，便于直接 grep reason |

**为什么不对 `profile_arn` 做 hash/脱敏？**
- `profile_arn` 不是凭据秘密（是 AWS ARN 形式的路由标识），历史 commit 已经在非脱敏文本里打印过。
- 直接打印有利于对比正常请求日志里的 arn；脱敏反而阻碍排查。

### 3. 保留 `bail!` 文本不变

`map_provider_error`（`src/anthropic/handlers.rs:32-70`）靠错误字符串匹配分流（如 `CONTENT_LENGTH_EXCEEDS_THRESHOLD`、`Input is too long`）。本次不动 `bail!` 文本，确保对外响应完全不变。

### 4. 不触碰凭据状态机

400 分支保持"不调用 `report_failure` / `mark_cooldown` / 不更新 `last_used_at`"。理由：
- 本次目标是观测，不是修复。若在观测阶段顺手改行为，会把"现象是否改变"与"观测是否有效"耦合，难以分离定性。
- 正确的行为修复（例如对 `INVALID_MODEL_ID` 做短 cooldown + 换凭据）应在看到现场证据后另起一次 change。

## Risks / Trade-offs

- **[日志放大]** 400 在正常流量下少见；但若遇到一次集中故障（本次场景下 8 连 400），日志会多 8 行 `error!`。可接受。
- **[profile_arn 泄漏担忧]** 见 Decisions 2 的说明，arn 不是秘密。
- **[观测盲区]** 如果下次复现时 400 的原因与本次不同（例如上游把 401 误映射成 400），本次字段集合仍然覆盖；但如果问题发生在进入 `call_api_with_retry` 之前（例如 `convert_request` 阶段就失败），本次日志不会触发——那类问题由 `src/anthropic/handlers.rs:244-262` 的 `ConversionError` 分支处理，不在本次覆盖面。

## Migration Plan

- 无数据迁移，无对外 API 变更。
- 部署即生效；下一次 400 故障的日志即带新字段。
- 回滚策略：`git revert` 本次提交即可恢复原行为。
