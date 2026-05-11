## Context

`kiro-rs` is an Anthropic API-compatible proxy that translates Anthropic model requests into Kiro API requests. Model name mapping is centralized in `src/anthropic/converter.rs` (`map_model()` and `get_context_window_size()`), and the advertised model list lives in `src/anthropic/handlers.rs` (`get_models()`).

Currently, the opus branch in `map_model()` only distinguishes opus 4.5 from everything else (which falls back to opus 4.6). A client sending `claude-opus-4-7-20250514` would silently receive opus 4.6 responses. The thinking-type detection in `override_thinking_from_model_name()` also only recognizes opus 4.6 as `adaptive`.

## Goals / Non-Goals

**Goals:**
- Correctly route `claude-opus-4-7*` model names to `claude-opus-4.7` on the Kiro backend
- Advertise `claude-opus-4-7` and `claude-opus-4-7-thinking` in the `/v1/models` response
- Assign 1M context window to opus 4.7 (consistent with opus 4.6)
- Apply `adaptive` thinking type to opus 4.7 thinking requests (consistent with opus 4.6)

**Non-Goals:**
- Changes to the Kiro backend or its model availability
- Changes to sonnet, haiku, or other model families
- Any API contract changes

## Decisions

**Decision: Explicit version check, not a "latest" fallback**

The current pattern checks for specific versions (4.5) and falls back to 4.6. We extend this by adding an explicit 4.7 check before the fallback, keeping the fallback at 4.6. This means unknown future opus versions still map to 4.6 (conservative), and 4.7 is explicitly handled.

Alternative considered: make 4.7 the new fallback. Rejected — it would silently upgrade unversioned `claude-opus-4` requests to 4.7, which could break clients that haven't tested against it.

**Decision: Opus 4.7 uses `adaptive` thinking (same as 4.6)**

The `adaptive` thinking type was introduced with opus 4.6. Opus 4.7 is a newer model in the same family and should support adaptive thinking. We extend the `is_opus_4_6` check to cover 4.7 as well (rename to `is_opus_adaptive_thinking` for clarity).

**Decision: 1M context window for opus 4.7**

Opus 4.6 has 1M context. Opus 4.7 as a newer model is expected to have at least the same. We assign 1M context to `claude-opus-4.7`.

## Risks / Trade-offs

- **[Risk] Kiro backend may not yet support `claude-opus-4.7`** → The proxy will correctly route the request, but Kiro may return an error. This is the expected failure mode and surfaces clearly to the client. No silent degradation.
- **[Risk] Context window size assumption** → If opus 4.7 has a different context window than 1M, the `get_context_window_size()` return value will be wrong. This only affects the `context_window` field in the `/v1/models` response, not actual request behavior.
