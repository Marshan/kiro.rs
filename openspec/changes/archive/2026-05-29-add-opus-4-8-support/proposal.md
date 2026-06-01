## Why

Anthropic 发布了 Claude Opus 4.8，这是 Opus 系列最新一代模型。当前 `map_model()` 的 opus 分支只显式识别 4.7 与 4.5，其它一律落到 4.6 fallback，意味着客户端发来 `claude-opus-4-8` 会被静默路由到 4.6，拿到错误的模型。`/v1/models` 也没有公布 4.8，使用 `/v1/models` 做模型发现的客户端看不到新模型。

## What Changes

- `map_model()` 在 opus 分支新增 4-8/4.8 显式分支（位于 4.7 之前），返回 `claude-opus-4.8`；其它分支不变，未指定版本的 opus 仍 fallback 到 4.6
- `get_context_window_size()` 将 `claude-opus-4.8` 加入 1M 上下文窗口列表
- `get_models()` 在列表头部新增 `claude-opus-4-8` 与 `claude-opus-4-8-thinking` 两条
- `override_thinking_from_model_name()` 的 `is_opus_adaptive` 判定追加 4-8/4.8，让 `claude-opus-4-8-thinking` 走 `adaptive` 思考类型（与 4.6/4.7 一致）
- `README.md` 模型映射表追加一行 `*opus*`（含 4.8/4-8） → `claude-opus-4.8`
- 新增对应单元测试

## Capabilities

### New Capabilities

- `opus-4-8-model-support`: 通过代理路由并公告 Claude Opus 4.8（标准与 thinking 变体）

### Modified Capabilities

<!-- 不修改任何现有 spec 的需求 -->

## Impact

- `src/anthropic/converter.rs`：`map_model()` 与 `get_context_window_size()`
- `src/anthropic/handlers.rs`：`get_models()` 模型列表与 `override_thinking_from_model_name()` thinking 检测
- `README.md`：模型映射表
- 无 API 契约变更，无破坏性变更，完全向后兼容
- 风险：Kiro 后端可能尚未支持 `claude-opus-4.8`，此时代理仍会正确路由请求，由 Kiro 直接回错给客户端，无静默降级
