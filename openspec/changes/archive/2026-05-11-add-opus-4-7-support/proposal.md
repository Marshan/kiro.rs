## Why

Claude Opus 4.7 is the latest and most capable model in the Anthropic lineup. The proxy currently maps any unrecognized opus model to `claude-opus-4.6`, meaning clients requesting `claude-opus-4-7` silently get the wrong model. This needs to be fixed now that Opus 4.7 is available.

## What Changes

- Add `claude-opus-4.7` as a recognized Kiro model ID in the model mapping logic
- Add `claude-opus-4-7` and `claude-opus-4-7-thinking` to the `/v1/models` endpoint response
- Extend the 1M context window assignment to include `claude-opus-4.7`
- Extend the `adaptive` thinking type detection to include opus 4.7 models

## Capabilities

### New Capabilities

- `opus-4-7-model-support`: Support for routing and advertising Claude Opus 4.7 (standard and thinking variants) through the proxy

### Modified Capabilities

<!-- No existing spec-level behavior changes -->

## Impact

- `src/anthropic/converter.rs`: `map_model()` and `get_context_window_size()` functions
- `src/anthropic/handlers.rs`: `get_models()` model list and `override_thinking_from_model_name()` thinking detection
- No API contract changes, no breaking changes, fully backward compatible
