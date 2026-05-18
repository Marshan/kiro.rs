## Why

当所有凭据都遭遇 Kiro 的"suspicious activity"封禁时，kiro-rs 仅应用 30 秒 cooldown 并无限循环重试，导致被封禁的凭据永远无法成功，而系统却持续消耗重试配额、管理界面也无法反映真实状态。同时，现有日志缺少实际调用的 Kiro 模型名称和凭据编号，排查问题时需要人工推断。

## What Changes

- **429 响应体解析**：区分 `INSUFFICIENT_MODEL_CAPACITY`（容量不足）和 "suspicious activity"（账号封禁）两种 429 类型
- **差异化 cooldown**：suspicious activity 类型应用 600 秒 cooldown（原 30 秒），容量不足保持 30 秒不变
- **失败计数联动**：suspicious activity 429 同时调用 `report_failure()`，3 次后自动禁用凭据，停止无效重试
- **入口日志增强**：`handlers.rs` 的请求日志新增 `kiro_model` 字段
- **凭据调度日志**：`provider.rs` 在凭据选定后新增 INFO 日志，包含凭据编号和 kiro_model
- **错误日志增强**：`provider.rs` 的 429 WARN 日志新增凭据编号和 kiro_model 字段
- 以上改动覆盖 API 路径和 MCP 路径

## Capabilities

### New Capabilities

- `suspicious-activity-429-handling`: 识别并差异化处理 Kiro 的 suspicious activity 429 响应，应用长 cooldown 并联动失败计数

### Modified Capabilities

- `api-logging`: 在请求入口日志、凭据调度日志、429 错误日志中新增 kiro_model 和凭据编号字段
- `credential-switching`: suspicious activity 429 触发 report_failure()，影响凭据自动禁用逻辑

## Impact

- `src/kiro/provider.rs`：API 路径和 MCP 路径的 429 处理逻辑、新增凭据调度日志
- `src/anthropic/handlers.rs`：请求入口日志新增 kiro_model 字段
- 管理界面：suspicious activity 封禁的凭据失败次数将正确递增，可从 UI 感知
