## ADDED Requirements

### Requirement: 请求入口日志包含 kiro_model 字段

`handlers.rs` 的 "Received POST /v1/messages request" 日志 SHALL 包含 `kiro_model` 字段，值为 Anthropic 模型名映射后的实际 Kiro 模型标识符。

#### Scenario: 入口日志包含 kiro_model

- **WHEN** 客户端发送 POST /v1/messages，请求模型为 "opus 4.6"
- **THEN** 日志行 SHALL 包含 `kiro_model=claude-opus-4.6` 字段

#### Scenario: 无法映射时 kiro_model 回退到原始模型名

- **WHEN** 客户端发送未知模型名（如 "unknown-model"），`map_model` 返回 None
- **THEN** 日志行 SHALL 包含 `kiro_model=unknown-model`（使用原始模型名）

### Requirement: 凭据调度日志

`provider.rs` SHALL 在每次凭据选定后（`acquire_context` 返回后）写入一条 INFO 日志，包含凭据编号（`credential_id`）和实际 Kiro 模型名（`kiro_model`）。

#### Scenario: API 路径凭据调度日志

- **WHEN** API 路径选定凭据 #8 处理 claude-opus-4.6 请求
- **THEN** 日志 SHALL 包含 `credential_id=8`、`kiro_model=claude-opus-4.6`，消息格式为 "凭据 #N 开始请求"

#### Scenario: MCP 路径凭据调度日志

- **WHEN** MCP 路径选定凭据 #9 处理请求
- **THEN** 日志 SHALL 包含 `credential_id=9` 和 `kiro_model` 字段

### Requirement: 429 错误日志包含凭据编号和 kiro_model

`provider.rs` 的 429 WARN 日志 SHALL 包含 `credential_id` 和 `kiro_model` 字段，使日志读者无需推断即可知道是哪个凭据在报错以及请求的是哪个模型。

#### Scenario: API 路径 429 日志包含凭据信息

- **WHEN** 凭据 #9 的 API 请求返回 429
- **THEN** WARN 日志 SHALL 包含 `credential_id=9` 和 `kiro_model=<实际模型名>` 字段

#### Scenario: MCP 路径 429 日志包含凭据信息

- **WHEN** MCP 路径凭据 #2 的请求返回 429
- **THEN** WARN 日志 SHALL 包含 `credential_id=2` 和 `kiro_model=<实际模型名>` 字段
