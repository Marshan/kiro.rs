## Context

kiro-rs 是一个 Anthropic API 兼容代理，将请求转发给 Kiro 后端。当前 `provider.rs` 对所有 HTTP 429 响应一律应用 30 秒 cooldown，不区分原因。Kiro 实际上会返回两种截然不同的 429：

1. **容量不足**（`reason: "INSUFFICIENT_MODEL_CAPACITY"`）：服务器繁忙，30 秒后通常可恢复
2. **可疑活动封禁**（`reason: null`，message 含 "suspicious activity"）：账号被 Kiro 临时调查限制，封禁时长为分钟到小时级

由于两种 429 被同等对待，被封禁的凭据每 30 秒出 cooldown 后立刻再次被拒，陷入无限循环。同时，现有日志缺少实际调用的 Kiro 模型名称和凭据编号，排查问题时需要人工推断。

## Goals / Non-Goals

**Goals:**
- 解析 429 响应体，区分 suspicious activity 和容量不足两种类型
- 对 suspicious activity 429 应用 600 秒 cooldown（原 30 秒）
- 对 suspicious activity 429 调用 `report_failure()`，3 次后自动禁用凭据
- 在 `handlers.rs` 入口日志中加入 `kiro_model` 字段
- 在 `provider.rs` 凭据选定后新增 INFO 日志（凭据编号 + kiro_model）
- 在 `provider.rs` 的 429 WARN 日志中加入凭据编号和 kiro_model
- 以上改动覆盖 API 路径和 MCP 路径

**Non-Goals:**
- 不修改凭据选择算法（priority/balanced 模式）
- 不修改 `report_failure` 的禁用阈值（保持 3 次）
- 不对其他 HTTP 错误码（401/403/402/5xx）的处理逻辑做任何改动
- 不引入新的外部依赖

## Decisions

### 决策 1：通过响应体字符串匹配识别 suspicious activity

**选择**：检查响应体是否包含 `"suspicious activity"` 子字符串。

**理由**：Kiro 的 suspicious activity 429 响应体格式为 `{"message":"Due to suspicious activity...","reason":null}`，`reason` 字段为 null，无法通过 reason 字段区分。字符串匹配简单可靠，无需引入 JSON 解析依赖（响应体已作为字符串存在）。

**备选方案**：解析 JSON 检查 `reason == null` + message 前缀。风险：Kiro 可能在未来修改消息文本，但 "suspicious activity" 是语义核心词，稳定性更高。

### 决策 2：suspicious activity 429 同时调用 report_failure()

**选择**：在标记 600s cooldown 的同时调用 `report_failure(ctx.id)`。

**理由**：suspicious activity 封禁是 Kiro 对账号的主动限制，与 401/403 的"凭据无效"性质相近。让失败计数递增，3 次后自动禁用，可以：
1. 停止对被封禁凭据的无效重试
2. 在管理界面暴露问题（失败次数可见）
3. 管理员解除封禁后可通过"重置失败"重新启用

**备选方案**：只加长 cooldown，不调用 report_failure。缺点：系统仍会周期性重试被封禁凭据，浪费重试配额。

### 决策 3：两阶段日志（入口 + 凭据调度）

**选择**：保留 `handlers.rs` 的入口日志（加 kiro_model），在 `provider.rs` 凭据选定后新增调度日志。

**理由**：`handlers.rs` 日志在凭据选择之前触发，无法包含凭据编号。两阶段方案保留了"请求进入系统"的时间锚点，同时在凭据选定后提供完整的调度信息。

**备选方案**：将请求日志整体移到 provider.rs。缺点：失去入口时间锚点，排查超时等问题时缺少参照。

### 决策 4：cooldown 上限提升至 600 秒（仅 suspicious activity）

**选择**：suspicious activity 的默认 cooldown 为 600 秒，`Retry-After` 头优先，上限 600 秒。容量不足保持原有 30 秒默认、120 秒上限。

**理由**：Kiro 的 suspicious activity 封禁通常持续数分钟，30 秒远不够。600 秒（10 分钟）是保守估计，配合 report_failure 的 3 次禁用机制，最多等待 30 分钟后凭据被禁用，停止重试。

## Risks / Trade-offs

- **[风险] suspicious activity 封禁解除后凭据仍被禁用** → 管理员需手动点击"重置失败"重新启用。这是可接受的权衡：主动封禁的凭据不应自动恢复，需要人工确认。
- **[风险] Kiro 修改 suspicious activity 消息文本** → 字符串匹配失效，退化为原有 30s cooldown 行为（不会更差）。可在未来通过更新匹配逻辑修复。
- **[权衡] 两阶段日志增加日志行数** → 每次请求多一条 INFO 日志。对于高并发场景，日志量会增加。可接受，因为这些信息对排查问题至关重要。
