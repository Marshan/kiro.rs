## ADDED Requirements

### Requirement: Opus 4.8 model name mapping
The proxy SHALL map any incoming Anthropic model name containing "opus" together with "4-8" or "4.8" to the Kiro model ID `claude-opus-4.8`.

#### Scenario: Exact versioned model name
- **WHEN** a client sends a request with model `claude-opus-4-8-20260529`
- **THEN** the proxy maps it to Kiro model ID `claude-opus-4.8`

#### Scenario: Short model name with dot notation
- **WHEN** a client sends a request with model `claude-opus-4.8`
- **THEN** the proxy maps it to Kiro model ID `claude-opus-4.8`

#### Scenario: Thinking suffix does not affect mapping
- **WHEN** a client sends a request with model `claude-opus-4-8-thinking`
- **THEN** the proxy maps it to Kiro model ID `claude-opus-4.8`

#### Scenario: Opus 4.7 mapping unchanged
- **WHEN** a client sends a request with model `claude-opus-4-7-20250514`
- **THEN** the proxy still maps it to Kiro model ID `claude-opus-4.7`

#### Scenario: Opus 4.6 fallback unchanged
- **WHEN** a client sends a request with model `claude-opus-4-20250514` (no specific version)
- **THEN** the proxy maps it to Kiro model ID `claude-opus-4.6` (existing fallback behavior unchanged)

### Requirement: Opus 4.8 advertised in models list
The proxy's `/v1/models` endpoint SHALL include `claude-opus-4-8` and `claude-opus-4-8-thinking` in its response.

#### Scenario: Models list includes opus 4.8
- **WHEN** a client calls `GET /v1/models`
- **THEN** the response includes a model entry with id `claude-opus-4-8`

#### Scenario: Models list includes opus 4.8 thinking variant
- **WHEN** a client calls `GET /v1/models`
- **THEN** the response includes a model entry with id `claude-opus-4-8-thinking`

### Requirement: Opus 4.8 context window
The proxy SHALL report a 1,000,000 token context window for `claude-opus-4.8`.

#### Scenario: Context window size for opus 4.8
- **WHEN** `get_context_window_size()` is called with a model name containing "opus" and "4.8"
- **THEN** it returns `1_000_000`

### Requirement: Opus 4.8 thinking type
The proxy SHALL use `adaptive` thinking type for opus 4.8 models with a thinking suffix, and SHALL set `output_config.effort` to `high`.

#### Scenario: Adaptive thinking for opus 4.8
- **WHEN** a client sends a request with model `claude-opus-4-8-thinking`
- **THEN** the proxy sets thinking type to `adaptive` (not `enabled`)
- **AND** the proxy sets `output_config.effort` to `high`
