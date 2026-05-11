## ADDED Requirements

### Requirement: API logging controlled by config
The system SHALL support an `apiLogEnabled` boolean field in `config.json` (default `false`) and an `apiLogPath` string field (default `"api.log"`). When `apiLogEnabled` is `false`, no API log file is created or written.

#### Scenario: Logging disabled by default
- **WHEN** `config.json` does not contain `apiLogEnabled`
- **THEN** no API log file is created and no API log entries are written

#### Scenario: Logging enabled with custom path
- **WHEN** `config.json` contains `"apiLogEnabled": true` and `"apiLogPath": "/tmp/kiro-api.log"`
- **THEN** the proxy writes API log entries to `/tmp/kiro-api.log`

#### Scenario: Logging enabled with default path
- **WHEN** `config.json` contains `"apiLogEnabled": true` but no `apiLogPath`
- **THEN** the proxy writes API log entries to `api.log` in the working directory

### Requirement: Request entry logged on every LLM call
When API logging is enabled, the system SHALL write a REQUEST log entry at the start of every `POST /v1/messages` call containing: `request_id`, `model_in` (client-supplied model name), `model_kiro` (mapped Kiro model ID), `stream`, `max_tokens`, and the full messages list with role and content (truncated to 500 chars per message).

#### Scenario: Request entry contains both model names
- **WHEN** a client sends `POST /v1/messages` with model `claude-opus-4-7-20250514`
- **THEN** the REQUEST log entry contains `model_in: claude-opus-4-7-20250514` and `model_kiro: claude-opus-4.7` as separate fields

#### Scenario: Messages content is recorded
- **WHEN** a client sends a request with 3 messages
- **THEN** the REQUEST log entry lists all 3 messages with their role and content (truncated to 500 chars each)

### Requirement: Kiro request entry logged before upstream call
When API logging is enabled, the system SHALL write a KIRO REQUEST log entry immediately before sending the request to the Kiro API, containing: `request_id`, `url`, and the full request body JSON.

#### Scenario: Kiro request entry contains URL and body
- **WHEN** the proxy is about to call the Kiro API
- **THEN** the KIRO REQUEST log entry contains the full `https://q.*.amazonaws.com/generateAssistantResponse` URL and the serialized request body

### Requirement: Response entry logged after completion
When API logging is enabled, the system SHALL write a RESPONSE log entry after the response is fully delivered to the client, containing: `request_id`, `model_in`, `model_kiro`, `stop_reason`, `input_tokens`, `output_tokens`, `duration_ms`, and the full response text content.

#### Scenario: Stream response entry contains full output text
- **WHEN** a streaming request completes
- **THEN** the RESPONSE log entry contains the complete concatenated text output from all text_delta events

#### Scenario: Non-stream response entry contains full output text
- **WHEN** a non-streaming request completes
- **THEN** the RESPONSE log entry contains the complete text content from the response body

#### Scenario: Duration is recorded
- **WHEN** any request completes
- **THEN** the RESPONSE log entry contains `duration_ms` reflecting the wall-clock time from request receipt to response completion

### Requirement: Log entries are human-readable and correlated
Log entries SHALL use a plain-text format with a timestamp prefix, a labeled section header, and fixed-width field alignment. All three entries for a single request SHALL share the same `request_id` value.

#### Scenario: Three entries share request_id
- **WHEN** a single LLM call is made
- **THEN** the REQUEST, KIRO REQUEST, and RESPONSE entries all contain the same `request_id`

#### Scenario: Log does not block request processing
- **WHEN** the log file write is slow
- **THEN** the client response is not delayed (writes are asynchronous via channel)
