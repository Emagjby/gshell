# gshell v1.0 Specification

## Vision

`gshell` is a fast, beautiful, daily-driver Unix shell written fully in Rust.

It aims for real bash compatibility in interactive use while keeping the internals clean, modular, and easy to extend. The project should copy proven logic and behavior from the current C implementation and from established shells where useful, without carrying forward bad UX or weak architecture.

## Product Goals

- Rewrite `gshell` fully in Rust.
- Target bash-compatible syntax and behavior for normal daily use.
- Provide modern interactive UX: autosuggestions, syntax highlighting, multiline editing, and rich completion.
- Support custom prompts with optional Starship integration.
- Load configuration from `~/.gshrc`.
- Keep the internal structure clear and easy to extend.
- Support macOS and Linux in v1.0.
- Ship with strong tests, compatibility checks, and benchmarks.

## Non-Goals

- Windows support in v1.0.
- Reinventing shell semantics when bash or POSIX behavior is already well understood.
- Vi mode in v1.0.
- A full plugin system in v1.0, though the architecture should leave room for one later.

## Compatibility Target

- Primary target: bash-style syntax and behavior for day-to-day interactive shell use.
- Standards target: POSIX-aligned where sensible, bash-compatible when common real-world behavior matters more.
- The current C `gshell` is a logic reference, not a strict compatibility target.

## v1.0 Feature Scope

### Commands and Execution

- External command execution via `PATH`.
- Builtins including the current set and additional bash-like builtins needed for usability.
- Aliases.
- Shell functions.
- `source` support for running shell files and reloading config.

### Syntax

- Pipelines: `|`
- Boolean chaining: `&&`, `||`
- Sequential execution: `;`
- Redirection: `>`, `>>`, `<`, `2>`, `2>>`, and fd-aware variants
- Heredocs: `<<`
- Command substitution: `$(...)`
- Subshells: `( ... )`
- Environment variable expansion: `$VAR`
- Exit status expansion: `$?`
- Globbing: `*`, `?`, character classes
- Quoting and escaping: single quotes, double quotes, backslash escaping

### Interactive UX

- Multiline editing
- Fish-style autosuggestion accepted with right arrow
- Bash-style tab completion
- Completion for commands, paths, env vars, flags, and other context-aware targets
- Syntax highlighting while typing
- Persistent history
- Prompt fallback to `$` when no custom prompt is available

### Job Control

- Foreground/background jobs
- `jobs`, `fg`, `bg`
- `Ctrl-C`, `Ctrl-Z`, and signal-aware foreground process handling

## Prompt Specification

- Default prompt is `$ `.
- The user may override the prompt in `~/.gshrc`.
- Starship is optional and user-controlled.
- If Starship is enabled and available, `gshell` invokes it externally and renders the result.
- If Starship is missing or fails, `gshell` falls back to the internal prompt.
- Prompt behavior should follow the same general model as Nushell and `reedline`:
  - left prompt
  - indicator prompt
  - right prompt
  - multiline continuation indicator
  - transient prompt support
- Prompt rendering must be cheap on repaint.
- Expensive prompt state should be cached per prompt cycle, not recomputed on every keypress redraw.
- v1.0 does not require `gshell`-specific Starship extensions unless they come nearly for free.

## Completion and Editing UX

- Editor backend: `reedline`
- `RightArrow` accepts autosuggestion only when the cursor is at end-of-line.
- `Tab` opens completion and then cycles forward.
- `Shift-Tab` cycles backward.
- The parser or validator controls whether `Enter` submits input or inserts a newline.
- No inline validation errors in v1.0.
- No vi mode in v1.0.

## Syntax Highlighting

- Highlighting is lightweight and semantic, not a full diagnostic system.
- Builtins are bold.
- `argv[0]` gets a dedicated command color.
- Later arguments get a secondary color.
- Pipes, redirects, operators, and shell syntax get their own color treatment.
- Colors should respect terminal or theme capabilities rather than force a hardcoded palette.
- The goal is a Nushell-like result that feels native to the active terminal theme.

## Configuration

- Primary config file: `~/.gshrc`
- It behaves like a shell rc file, not a static structured config file.
- It may execute shell commands during startup.
- Config errors warn and continue.
- Environment variables override rc-defined values where both apply.
- History must be persistent across sessions.
- `source` reload is supported during an active session.

## Architecture

The codebase should be organized as a workspace or as clearly separated modules with equivalent boundaries.

### Core Components

- `lexer`
  - shell tokenization
  - quote and escape state
  - heredoc bookkeeping
  - command substitution boundaries

- `parser`
  - transforms tokens into AST structures
  - uses `winnow` as the primary parser library
  - may fall back to `chumsky` later if diagnostics ergonomics become more important than parser control

- `ast`
  - shell grammar nodes for simple commands, pipelines, lists, subshells, redirections, functions, and related constructs

- `expand`
  - environment variable expansion
  - command substitution
  - glob expansion
  - quote-aware word expansion rules

- `runtime`
  - execution engine
  - process spawning
  - pipelines
  - redirections
  - async orchestration
  - exit-status propagation

- `builtins`
  - builtin registry
  - trait-based command implementation
  - easy builtin addition

- `jobs`
  - job table
  - foreground/background control
  - signal handling

- `completion`
  - command completion
  - path completion
  - env var completion
  - context-aware completion sources

- `prompt`
  - internal prompt renderer
  - Starship integration
  - prompt cache and transient prompt logic

- `config`
  - startup loading
  - rc execution hooks
  - environment and config precedence

- `ui`
  - `reedline` integration
  - editor bindings
  - hints and highlighting plumbing

- `compat`
  - bash compatibility tests and behavior fixtures

## Parser Strategy

- Use a hybrid approach.
- Implement a handwritten lexer and state machine for shell words and context-sensitive tokenization.
- Use `winnow` for higher-level parsing structure.
- Handle heredocs as a dedicated parser phase rather than as ordinary tokens alone.

This approach is chosen because shell syntax is too context-sensitive to model cleanly as a pure grammar-only problem. Heredocs, nested `$(...)`, redirects, and quoting need parser-state awareness.

## Execution Model

- The runtime should be async-first from day one.
- Async orchestration should support prompt responsiveness, job control, and future extensibility.
- Process execution still relies on normal Unix process APIs underneath.
- v1.0 targets macOS and Linux first, but internal boundaries should avoid making future portability impossible.

## Builtin Strategy

The Rust rewrite should preserve and port the current builtins:

- `cd`
- `exit`
- `clear`
- `type`
- `echo`
- `pwd`
- `history`

It should also add key shell builtins needed for bash-like operation:

- `source`
- `alias`
- `unalias`
- `export`
- `unset`
- `jobs`
- `fg`
- `bg`

Builtins should be trait-based and registry-driven so that adding new builtins is straightforward.

## Behavioral Principles

- Prefer copying known shell semantics over inventing new ones.
- Prefer internal clarity over clever parser tricks.
- Keep the interactive shell responsive even when prompt and completion logic are rich.
- Treat syntax support and execution semantics as equally important.
- Favor warnings and recovery in the REPL over fatal startup failure when reasonable.

## Testing Requirements

- Unit tests for lexer, parser, expander, and builtins
- Integration tests for execution and pipelines
- PTY-driven tests for interactive behavior
- Compatibility tests against bash for selected syntax and semantics
- Benchmarks for startup latency, prompt render latency, completion latency, parse speed, and command dispatch overhead

## Packaging

v1.0 should support:

- `cargo install`
- Homebrew
- standalone release binaries

Supported platforms for v1.0:

- macOS
- Linux

## Recommended Technical Choices

- Line editor: `reedline`
- Parser library: `winnow`
- Compatibility model: bash-like, POSIX-aware
- Config model: executable `~/.gshrc`
- Prompt model: internal fallback plus optional Starship
- Architecture: modular workspace, async-first runtime, trait-based builtins

## Delivery Plan

### Phase 1

- Rust shell skeleton
- REPL
- prompt fallback
- history
- external commands
- current C builtins

### Phase 2

- parser and AST rewrite with pipelines, redirects, quoting, and env vars
- completion and syntax highlighting
- multiline editing
- Starship integration

### Phase 3

- heredocs
- command substitution
- subshells
- aliases
- functions
- globbing

### Phase 4

- job control
- `fg`/`bg`/`jobs`
- stronger bash compatibility
- performance pass
- packaging and release polish
