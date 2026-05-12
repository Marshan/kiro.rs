## Why

在 `cd8fb61` 打开 priority 模式同优先级凭据轮转后，出现了一类新的用户可观察症状：上游 Kiro 在短时间内对 `opus-4-7` 请求返回 `400 Bad Request {"reason":"INVALID_MODEL_ID"}`，但同一张凭据在故障前后均可正常使用。

当前 `src/kiro/provider.rs` 的 400 处理（API 路径 `:449-461`、MCP 路径 `:234-246`）直接 `anyhow::bail!(...)` 退出，`src/anthropic/handlers.rs:32-70` 的 `map_provider_error` 仅透传错误文本。导致现有日志**缺乏**定位所必需的结构化字段：

- `credential_id`：无法判断 8 次连续 400 是同一张凭据还是不同凭据（决定是"上游瞬态/地域风控"还是"项目 bug"）
- `endpoint` 名称：无法验证 endpoint 与 credential 是否错配
- `profile_arn`：无法复核历史 `53df562` 修过的"切换后携带过期 ARN"是否再次出现

在不补齐这些字段前，无法对症下药（修 400 语义 vs. 修具体请求构造 bug）。本次变更只做**观测增强**，不改变重试/退避/切换行为，为下一次复现收集证据。

## What Changes

- 在 `src/kiro/provider.rs` 的 API 路径 400 分支 `bail!` 前，新增一行 `tracing::error!`，结构化字段包含 `credential_id`、`endpoint`（来自 `KiroEndpoint::name()`）、`profile_arn`（取自 `ctx.credentials.profile_arn`，缺省记为 `<none>`）、`status`、`body`。
- 在 MCP 路径 400 分支做相同处理（字段一致）。
- **不改**客户端可见行为：`bail!` 的错误文本不变，`map_provider_error` 返回的 502 响应不变。
- **不改**凭据状态机：400 仍然不调用 `report_failure` / `mark_cooldown` / 更新 `last_used_at`，不触发凭据切换或服务端退避。

## Capabilities

### Modified Capabilities
- `credential-switching`: 新增一条观测性要求，规定 400 响应在 bail 前必须以结构化日志记录凭据、endpoint、profile_arn、body，便于把"上游瞬态/风控"与"项目构造 bug"在日志层分离定性。

### New Capabilities
<!-- 无 -->

## Impact

- 代码：仅 `src/kiro/provider.rs` 两处 400 分支新增日志行，无签名变化、无新依赖、无行为改变。
- API：对外 Anthropic API / Admin API 行为完全不变。
- 观测：400 失败事件在日志中具备 `credential_id` / `endpoint` / `profile_arn` 字段，可直接 `grep` 定性。
- 性能：可忽略（每次 400 多一条 tracing 日志）。
- 回滚：`git revert` 即可，无持久化状态变化。
