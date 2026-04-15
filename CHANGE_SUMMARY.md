# oMLX Provider Integration

Adds oMLX as a built-in OSS provider for codex, alongside Ollama and LM Studio.

## What is oMLX

oMLX is a local MLX-based inference server exposing OpenAI-compatible endpoints including `/v1/responses` (the wire protocol codex uses), `/v1/models`, and `/health`. Models are discovered from disk and lazy-loaded on first request.

## Usage

```bash
# Explicit model
codex --oss --local-provider omlx -m qwen3-30b

# Config-based (in ~/.codex/config.toml)
#   oss_provider = "omlx"
#   model = "qwen3-30b"
#   model_context_window = 131072
codex --oss

# Auto-select first available model from server
codex --oss --local-provider omlx
```

## Configuration

Set in `~/.codex/config.toml`:

```toml
oss_provider = "omlx"
model = "qwen3-30b"
model_context_window = 131072
# tool_output_token_limit = 32768  # optional
```

## Key differences from other OSS providers

| | Ollama | LM Studio | oMLX |
|---|---|---|---|
| Default port | 11434 | 1234 | 8000 |
| Default model | `gpt-oss:20b` | `openai/gpt-oss-20b` | None (auto-select) |
| Model setup | Pulls if missing | Downloads if missing | Already on disk |

## Files changed

- `codex-rs/omlx/` -- new crate: client, health check, model listing
- `codex-rs/model-provider-info/src/lib.rs` -- provider constants and registration
- `codex-rs/config/src/config_toml.rs` -- validation and reserved IDs
- `codex-rs/utils/oss/src/lib.rs` -- OSS utilities integration
- `codex-rs/utils/oss/Cargo.toml` -- added codex-omlx dependency
- `codex-rs/tui/src/oss_selection.rs` -- provider selection UI
- `codex-rs/tui/src/lib.rs` -- auto-select model fallback
- `codex-rs/exec/src/lib.rs` -- auto-select model fallback
- `codex-rs/Cargo.toml` -- workspace members and dependencies
