## 1. Model Mapping (converter.rs)

- [x] 1.1 Add opus 4.7 branch in `map_model()`: check for "4-7" or "4.7" → return `"claude-opus-4.7"`
- [x] 1.2 Extend `get_context_window_size()` to include `claude-opus-4.7` in the 1M context match arm

## 2. Models List (handlers.rs)

- [x] 2.1 Add `claude-opus-4-7` model entry to `get_models()` response
- [x] 2.2 Add `claude-opus-4-7-thinking` model entry to `get_models()` response

## 3. Thinking Type Detection (handlers.rs)

- [x] 3.1 Extend `override_thinking_from_model_name()` to treat opus 4.7 as `adaptive` thinking (alongside opus 4.6)

## 4. Tests

- [x] 4.1 Add unit test for `map_model("claude-opus-4-7-20250514")` → `"claude-opus-4.7"`
- [x] 4.2 Add unit test for `map_model("claude-opus-4.7")` → `"claude-opus-4.7"`
- [x] 4.3 Add unit test for `map_model("claude-opus-4-7-thinking")` → `"claude-opus-4.7"`
- [x] 4.4 Verify existing opus 4.6 fallback test still passes (no regression)
- [x] 4.5 Run `cargo test` and `cargo clippy` to confirm no errors
