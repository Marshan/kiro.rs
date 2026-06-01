## Context

`kiro-rs` 是 Anthropic API 兼容代理，将 Anthropic 模型请求翻译成 Kiro API 请求。模型名映射集中在 `src/anthropic/converter.rs`（`map_model()` / `get_context_window_size()`），公布的模型列表在 `src/anthropic/handlers.rs`（`get_models()`），thinking 类型决策也在 handlers 的 `override_thinking_from_model_name()`。

当前 opus 分支显式识别 4-7/4.7 与 4-5/4.5 两个版本，其它一律 fallback 到 4.6。客户端发来 `claude-opus-4-8-20260529` 会被静默映射到 4.6，跑出来不是预期模型。Thinking 检测只把 4.6/4.7 视作 `adaptive`，4.8 不识别会被错误地降级到 `enabled`。

此前的 `2026-05-11-add-opus-4-7-support` change（已 archive）建立了同类变更的模板，本次按相同思路扩展。

## Goals / Non-Goals

**Goals:**
- 将 `claude-opus-4-8*` 名称正确映射到 Kiro 后端的 `claude-opus-4.8`
- 在 `/v1/models` 公布 `claude-opus-4-8` 与 `claude-opus-4-8-thinking`
- 给 4.8 分配 1M 上下文窗口（与 4.7/4.6 看齐）
- 4.8 thinking 请求走 `adaptive` 类型（与 4.7/4.6 一致）

**Non-Goals:**
- Kiro 后端模型可用性（取决于 Kiro 是否已上线 4.8，本次不涉及）
- sonnet / haiku 或其它模型族的变更
- 任何 API 契约变更

## Decisions

**Decision: 显式版本检查，保留 4.6 fallback**

按现有 4.5/4.7 的写法，新增一条显式 4-8/4.8 检查，置于 4-7 之前（最新优先）；fallback 仍保留 4.6，未指定版本的 `claude-opus-4` 行为不变。

替代方案：将 4.8 设为新 fallback。**否决**——会让没指定版本的客户端被静默升级到 4.8，破坏既有约定。这与 4.7 那次的决策一致。

**Decision: Opus 4.8 使用 `adaptive` thinking（与 4.6/4.7 同）**

`adaptive` thinking 类型在 4.6 引入；4.7 沿用；4.8 是同族更新版本，沿用 `adaptive` 是最自然的延续。`is_opus_adaptive` 判定追加 4-8/4.8。budget_tokens 仍 20000，`output_config.effort` 仍 `high`。

**Decision: 1M 上下文窗口**

4.6/4.7 都是 1M。4.8 作为更新版本至少不低于此，按 1M 处理。`get_context_window_size()` 仅影响 `/v1/models` 的元数据上报，不影响实际请求行为，错了影响也很小。

**Decision: Kiro 后端模型 ID 假定为 `claude-opus-4.8`**

沿用 4.6/4.7 的 dot-notation 命名（`claude-opus-4.6`、`claude-opus-4.7`）。如果 Kiro 实际用别的 ID，本次只需要改 `map_model()` 单一返回值，影响面小。

## Risks / Trade-offs

- **[Risk] Kiro 后端可能尚未支持 `claude-opus-4.8`** → 代理会正确路由，Kiro 直接返回错误（如 `ValidationException`），客户端能清晰看到失败原因。无静默降级。
- **[Risk] 上下文窗口大小假设** → 若 4.8 实际不是 1M，`get_context_window_size()` 返回值会有误。仅影响 `/v1/models` 的 `context_window` 元数据字段，不影响请求处理。
- **[Risk] Kiro 模型 ID 实际命名可能不是 `claude-opus-4.8`** → 若 Kiro 用别的 ID，本次只需改 `map_model()` 的返回字符串与对应测试，调整代价低。
