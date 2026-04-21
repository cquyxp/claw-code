# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claw Code is a public Rust implementation of the `claw` CLI agent harness. It's a high-performance, native rewrite of the Claude Code agent system.

- **Repository:** ultraworkers/claw-code
- **Primary implementation:** `rust/` directory (Rust workspace)
- **Canonical binary:** `claw` (built from `rust/crates/rusty-claude-cli`)

## Common Commands

### Build and Run

```bash
# Build the entire workspace
cd rust
cargo build --workspace

# Run the CLI
./target/debug/claw

# Run a one-shot prompt
./target/debug/claw prompt "your prompt here"

# Shorthand prompt
./target/debug/claw "your prompt here"
```

### Verification and Testing

```bash
# Format code
cd rust
cargo fmt

# Run lints
cargo clippy --workspace --all-targets -- -D warnings

# Run tests
cargo test --workspace

# Run health check (first thing after building)
./target/debug/claw
# then run `/doctor` inside the REPL
```

### Mock Parity Harness

```bash
cd rust
# Run the full parity harness
./scripts/run_mock_parity_harness.sh

# Start mock service manually
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## High-Level Architecture

### Rust Workspace Crates

The project is organized as a Rust workspace with 9 crates in `rust/crates/`:

| Crate | Responsibility | Key Dependencies |
|-------|----------------|------------------|
| **rusty-claude-cli** | Main binary entrypoint, REPL, CLI parsing, output rendering | runtime, api, commands, tools, plugins |
| **runtime** | Core conversation loop, session persistence, config, permissions, MCP, hooks | api (types only) |
| **api** | Provider clients (Anthropic, OpenAI-compatible, xAI, DashScope), SSE streaming, request types | telemetry |
| **tools** | Built-in tool implementations (Bash, ReadFile, WriteFile, etc.), tool registry | runtime (file operations) |
| **commands** | Slash command definitions, parsing, help text | runtime, plugins |
| **plugins** | Plugin metadata, lifecycle management, plugin tool definitions | - |
| **telemetry** | Session trace events and usage telemetry types | - |
| **mock-anthropic-service** | Local mock service for parity testing | - |
| **compat-harness** | Extracts manifests from upstream TypeScript | - |

### Dependency Flow

```
rusty-claude-cli (main)
├── runtime (core logic)
│   ├── api (type definitions only)
│   ├── tools
│   ├── commands
│   └── plugins
├── api (client implementations)
│   └── telemetry
└── tools
    └── runtime (file operations)
```

## Core Architecture Deep Dive

### 1. Execution Flow: CLI Entrypoint to Tool Execution

#### Entrypoint (`rusty-claude-cli/src/main.rs`)
1. **`main()`** calls **`run()`**
2. **`parse_args()`** determines the `CliAction`
3. Primary modes:
   - `CliAction::Repl` - Interactive REPL mode
   - `CliAction::Prompt` - One-shot prompt
   - `CliAction::ResumeSession` - Resume saved session
   - Various utility commands (status, config, mcp, plugins, etc.)

#### REPL/Prompt Flow
```
LiveCli::new()
├── ConfigLoader::load() - Load config hierarchy
├── SessionStore::from_cwd() - Session management
├── GlobalToolRegistry::builtin() - Load tools
├── ProviderClient initialization
│   ├── detect_provider_kind(model) - Route to Anthropic/OpenAI/xAI
│   └── Resolve auth from env/settings
└── ConversationRuntime::new()
    ├── Session
    ├── ApiClient (trait)
    ├── ToolExecutor (trait)
    ├── PermissionPolicy
    └── System prompt
```

#### Conversation Turn (`runtime/src/conversation.rs`)
**`ConversationRuntime::run_turn()`** executes:
1. Push user input to session
2. **Loop** (max iterations):
   - Build `ApiRequest` from session + system prompt
   - Call **`api_client.stream()`** → `Vec<AssistantEvent>`
   - Parse events into `ConversationMessage`
   - Push assistant message to session
   - If no tool uses, **break**
   - For each tool use:
     - **`permission_enforcer`** checks if allowed
     - **`tool_executor.execute()`** runs the tool
     - Push tool result to session
     - Run post-tool-use hooks
3. Return `TurnSummary`

### 2. Main Data Structures and Types

#### Session Management (`runtime/src/session.rs`)
- **`Session`**: Core persisted state
  - `messages: Vec<ConversationMessage>` - Conversation history
  - `session_id: String`, `created_at_ms`, `updated_at_ms`
  - `compaction: Option<SessionCompaction>` - Compacted history summary
  - `workspace_root: Option<PathBuf>` - Binds session to a workspace
  - `persistence: Option<SessionPersistence>` - On-disk location

- **`ConversationMessage`**: Single message in the conversation
  - `role: MessageRole` (System, User, Assistant, Tool)
  - `blocks: Vec<ContentBlock>`
  - `usage: Option<TokenUsage>`

- **`ContentBlock`**: Variants for different content types
  - `Text { text: String }`
  - `ToolUse { id, name, input }`
  - `ToolResult { tool_use_id, tool_name, output, is_error }`

#### Configuration (`runtime/src/config.rs`)
- **`ConfigLoader`**: Discovers and merges config from multiple sources
- **`RuntimeConfig`**: Fully merged configuration
- **`RuntimeFeatureConfig`**: Parsed feature-specific settings (hooks, plugins, MCP, OAuth, permissions)
- **`McpServerConfig`**: Enum with variants (Stdio, Sse, Http, Ws, Sdk, ManagedProxy)

#### API Types (`api/src/types.rs`)
- **`MessageRequest`**, **`MessageResponse`** - API request/response
- **`InputMessage`**, **`OutputContentBlock`** - Message content
- **`ToolDefinition`**, **`ToolChoice`** - Tool definitions
- **`StreamEvent`** - SSE streaming events

### 3. Runtime Architecture (`runtime/`)

#### Core Components

##### `ConversationRuntime` (`conversation.rs`)
- Generic over `C: ApiClient` and `T: ToolExecutor`
- Holds: Session, ApiClient, ToolExecutor, PermissionPolicy, SystemPrompt
- Key methods:
  - `new()` / `new_with_features()` - Constructor
  - `run_turn()` - Single user-assistant turn
  - `with_max_iterations()`, `with_session_tracer()`, etc.

##### Permission System (`permissions.rs`, `permission_enforcer.rs`)
- **`PermissionMode`**: ReadOnly, WorkspaceWrite, DangerFullAccess
- **`PermissionPolicy`** evaluates requests
- **`PermissionEnforcer`** integrates with tool execution
- Rules-based with allow/deny/ask patterns

##### Config System (`config.rs`)
**Config Loading Hierarchy** (later overrides earlier):
1. `~/.claw.json` (legacy user)
2. `~/.config/claw/settings.json` (user)
3. `.claw.json` (project legacy)
4. `.claw/settings.json` (project)
5. `.claw/settings.local.json` (local override)

**`ConfigLoader::discover()`** finds config files
**`ConfigLoader::load()`** merges them with validation via `config_validate.rs`

##### Hooks System (`hooks.rs`)
- Lifecycle hooks: `pre_tool_use`, `post_tool_use`, `post_tool_use_failure`
- Configured via settings
- Executes shell commands before/after tool calls

### 4. Tool Implementation and Registration

#### Tool Definitions (`tools/src/lib.rs`)
- **`ToolSpec`**: Name, description, input_schema, required_permission
- **`mvp_tool_specs()`** returns all builtin tool specs
- **`RuntimeToolDefinition`** for dynamic/runtime tools
- **`GlobalToolRegistry`** aggregates:
  - Builtin tools
  - Plugin tools (via `plugins` crate)
  - Runtime tools (MCP, etc.)

#### Tool Execution
```rust
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}
```

Implemented by `StaticToolExecutor` in runtime which:
1. Validates tool name and input
2. Checks permissions
3. Routes to appropriate implementation
4. Returns JSON-serialized result

#### Builtin Tool Categories
1. **File operations**: ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch
2. **Execution**: Bash
3. **Web**: WebSearch, WebFetch
4. **Agent**: Agent, TodoWrite, NotebookEdit, Skill
5. **Other**: LaneCompletion, PdfExtract

### 5. Session Persistence (`runtime/src/session.rs`, `session_control.rs`)

#### Storage Format
- **JSONL (JSON Lines)** format - append-only for each message
- Legacy **JSON** format support for loading
- Files stored in `.claw/sessions/<workspace-fingerprint>/`

#### Session Lifecycle
1. **Create**: `Session::new()` generates UUID
2. **Save**: 
   - `push_message()` appends to in-memory and persists
   - `save_to_path()` writes full snapshot with rotation (256KB per file, max 3 rotated files)
3. **Load**: `Session::load_from_path()` auto-detects JSON/JSONL
4. **Fork**: `session.fork()` creates new session with parent reference

#### Session Store (`session_control.rs`)
- **`SessionStore`** manages per-workspace sessions
- **`workspace_fingerprint()`** creates FNV-1a hash of workspace path
- Methods:
  - `list_sessions()` - Returns `Vec<ManagedSessionSummary>`
  - `load_session(reference)` - Load by ID, path, or alias ("latest")
  - `fork_session()` - Create child session
  - `resolve_reference()` - Handle "latest", paths, or IDs

#### Compaction (`compact.rs`, `summary_compression.rs`)
- **`should_compact()`** checks if session exceeds threshold (default 100K tokens)
- **`compact_session()`** summarizes old messages using model
- Stores summary in `session.compaction` field
- Keeps recent messages intact

### 6. Provider Implementation (`api/src/providers/`)

#### Trait and Architecture
```rust
pub trait Provider {
    type Stream;
    fn send_message<'a>(&'a self, request: &'a MessageRequest) -> ProviderFuture<'a, MessageResponse>;
    fn stream_message<'a>(&'a self, request: &'a MessageRequest) -> ProviderFuture<'a, Self::Stream>;
}
```

#### Supported Providers

##### 1. Anthropic (`anthropic.rs`)
- **`AnthropicClient`** struct
- **`AuthSource`** enum:
  - `None`
  - `ApiKey(String)` (from `ANTHROPIC_API_KEY`)
  - `BearerToken(String)` (from `ANTHROPIC_AUTH_TOKEN`)
  - `ApiKeyAndBearer` (both)
- Features:
  - Retry policy with exponential backoff (max 8 retries)
  - Prompt caching
  - SSE streaming via `SseParser`
  - OAuth token support

##### 2. OpenAI-Compatible (`openai_compat.rs`)
- **`OpenAiCompatClient`**
- Supports:
  - OpenAI (native)
  - OpenRouter
  - xAI (Grok models)
  - DashScope (Qwen, Kimi models)
  - Local models (Ollama, LM Studio, vLLM)
- **`OpenAiCompatConfig`** for custom base URLs
- Translates between Anthropic and OpenAI formats

#### Provider Detection (`detect_provider_kind()`)
Order of precedence:
1. Model prefix (e.g., "gpt-", "openai/", "qwen-", "kimi-", "grok")
2. `OPENAI_BASE_URL` presence → OpenAi
3. Anthropic auth present → Anthropic
4. `OPENAI_API_KEY` → OpenAi
5. `XAI_API_KEY` → Xai
6. `OPENAI_BASE_URL` (without key) → OpenAi
7. Default → Anthropic

#### Model Registry (`MODEL_REGISTRY`)
Maps aliases to full model names:
- `opus` → `claude-opus-4-6`
- `sonnet` → `claude-sonnet-4-6`
- `haiku` → `claude-haiku-4-5-20251213`
- `grok` → `grok-3`
- `kimi` → `kimi-k2.5`

Also tracks context window sizes:
- Claude 4: 200K context
- Grok: 131K context
- Kimi: 256K context

## Key Concepts

- **Session persistence:** Sessions are stored in `.claw/sessions/` as JSONL files
- **Config hierarchy:** Loaded in order: `~/.claw.json` → `~/.config/claw/settings.json` → `.claw.json` → `.claw/settings.json` → `.claw/settings.local.json`
- **Permission modes:** `read-only`, `workspace-write`, `danger-full-access`
- **Model aliases:** `opus` → `claude-opus-4-6`, `sonnet` → `claude-sonnet-4-6`, `haiku` → `claude-haiku-4-5-20251213`

## Authentication

| Credential Type | Env Var | Header |
|-----------------|---------|--------|
| Anthropic API key (`sk-ant-*`) | `ANTHROPIC_API_KEY` | `x-api-key` |
| Anthropic OAuth token | `ANTHROPIC_AUTH_TOKEN` | `Authorization: Bearer` |
| OpenAI-compatible / OpenRouter | `OPENAI_API_KEY` + `OPENAI_BASE_URL` | `Authorization: Bearer` |
| xAI | `XAI_API_KEY` | `Authorization: Bearer` |
| DashScope (Qwen) | `DASHSCOPE_API_KEY` | `Authorization: Bearer` |

## Repository Shape

- **`rust/`** — Canonical Rust workspace and active CLI/runtime implementation
- **`src/` + `tests/`** — Companion Python/reference workspace and audit helpers (not primary runtime)
- **`docs/`** — Additional documentation
- **`assets/`** — Images and media

## Key Files Summary

| Path | Purpose |
|------|---------|
| `rust/crates/rusty-claude-cli/src/main.rs` | CLI entrypoint, REPL, argument parsing |
| `rust/crates/runtime/src/conversation.rs` | Core `ConversationRuntime` and turn loop |
| `rust/crates/runtime/src/session.rs` | Session data structure and persistence |
| `rust/crates/runtime/src/config.rs` | Config loading and merging |
| `rust/crates/runtime/src/permissions.rs` | Permission system |
| `rust/crates/api/src/providers/mod.rs` | Provider routing and model registry |
| `rust/crates/api/src/providers/anthropic.rs` | Anthropic client |
| `rust/crates/api/src/providers/openai_compat.rs` | OpenAI-compatible client |
| `rust/crates/tools/src/lib.rs` | Tool registry and execution |
| `rust/crates/plugins/src/lib.rs` | Plugin system |

## Important Notes

- **Do not use `cargo install claw-code`** — that's a deprecated stub on crates.io. Build from source instead.
- **Windows:** Binary is `claw.exe`, use backslashes in paths
- **First run:** Always run `/doctor` inside the REPL after building
- **ACP/Zed:** Not yet implemented; `claw acp` only shows current status
- **`unsafe_code` is forbidden** in the Rust workspace (enforced at the workspace level)

## Working Agreement

- Prefer small, reviewable changes
- Keep generated bootstrap files aligned with actual repo workflows
- Keep shared defaults in `.claude.json`; reserve `.claude/settings.local.json` for machine-local overrides
- Update both `src/` and `tests/` surfaces together when behavior changes
