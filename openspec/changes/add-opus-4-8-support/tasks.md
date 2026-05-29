## 1. Model Mapping (converter.rs)

- [x] 1.1 在 `map_model()` 的 opus 分支新增 4-8/4.8 显式判定，置于 4-7 检查之前，返回 `"claude-opus-4.8"`
- [x] 1.2 在 `get_context_window_size()` 的 1M 匹配分支加入 `claude-opus-4.8`
- [x] 1.3 更新 `map_model()` 的 doc 注释，补充 4.8 映射规则

## 2. Models List (handlers.rs)

- [x] 2.1 在 `get_models()` 列表头部新增 `claude-opus-4-8` 模型条目（display_name `Claude Opus 4.8`、max_tokens 64000）
- [x] 2.2 在 `get_models()` 中新增 `claude-opus-4-8-thinking` 模型条目（display_name `Claude Opus 4.8 (Thinking)`、max_tokens 64000）

## 3. Thinking Type Detection (handlers.rs)

- [x] 3.1 扩展 `override_thinking_from_model_name()` 中 `is_opus_adaptive` 判定，加入 4-8/4.8 匹配（与 4.6/4.7 同等待遇）
- [x] 3.2 同步更新 `override_thinking_from_model_name()` 的 doc 注释

## 4. Tests

- [x] 4.1 新增单元测试 `test_map_model_opus_4_8_versioned`：`claude-opus-4-8-20260529` → `claude-opus-4.8`
- [x] 4.2 新增单元测试 `test_map_model_opus_4_8_dot_notation`：`claude-opus-4.8` → `claude-opus-4.8`
- [x] 4.3 新增单元测试 `test_map_model_opus_4_8_thinking`：`claude-opus-4-8-thinking` → `claude-opus-4.8`
- [x] 4.4 确认现有 4.7 / 4.6 fallback 测试无回归

## 5. Documentation

- [x] 5.1 在 `README.md` 模型映射表新增一行 `*opus*`（含 4.8/4-8） → `claude-opus-4.8`
- [x] 5.2 检查 `README.md` 顶部 banner 是否需要追加 4.8 提及

## 6. Verification

- [x] 6.1 运行 `cargo test`，确认全部通过
- [x] 6.2 运行 `cargo clippy`，确认无 warning/error
