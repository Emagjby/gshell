# Phase 1 Board

## How to use this board

- This file is both the ordered todo list and the kanban board for Phase 1.
- Work cards from top to bottom unless a dependency note says otherwise.
- Default status for all cards at phase start is `todo`.
- Valid statuses are `todo`, `in progress`, `blocked`, and `done`.

## Ordered cards

## Card P1-01 - Initialize the Rust workspace

### Status

`todo`

### Priority

High

### Size

M

### Depends on

None

### Goal of subphase

Create the Rust project foundation so all later work lands in stable directories with stable tooling.

### The details

1. Create the top-level `Cargo.toml`.
2. Decide whether `gshell` starts as one crate with internal modules or a small workspace with a binary crate and support crates.
3. Create the main binary entrypoint.
4. Create the initial source tree for the architecture modules described in `docs/SPEC.md`.
5. Add only the minimum dependencies needed now:
   - async runtime
   - line editor
   - error handling
   - testing support
   - optional tracing or logging
6. Add repo-standard commands for:
   - `cargo check`
   - `cargo test`
   - `cargo fmt`
   - `cargo clippy`
7. Ensure the binary compiles before any shell behavior is added.
8. Document short setup notes in the crate comments or future docs if required.

### Tests to implement for the current subphase

- `cargo check` passes
- `cargo test` passes with a minimal smoke test
- `cargo fmt --check` passes
- `cargo clippy` passes at the agreed strictness level

### Definition of done

- The Rust project builds from a clean checkout
- The shell binary exists and starts without panicking
- The source tree is aligned with the intended architecture

## Card P1-02 - Define shell state and core interfaces

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P1-01

### Goal of subphase

Define the shared state and contracts used by the REPL, builtins, executor, config loader, and prompt layer.

### The details

1. Create `ShellState` or the equivalent central state container.
2. Include at minimum:
   - current environment map or accessor
   - current working directory accessor
   - last exit status
   - history handle placeholder
   - alias store placeholder
   - function store placeholder
   - runtime services handle
3. Define how mutable state is accessed in async code.
4. Define a builtin trait interface.
5. Define a prompt interface.
6. Define an execution interface for parsed commands.
7. Define common result and error types.
8. Decide how command exit codes are represented and updated.
9. Keep the interfaces explicit enough that later phases do not require broad rewrites.

### Tests to implement for the current subphase

- Unit test for state initialization defaults
- Unit test for environment read and write behavior
- Unit test for last-exit-status update behavior
- Unit test for builtin registry interface basics

### Definition of done

- Core interfaces exist and are documented in code
- The shell state can be constructed in tests and production code
- Future work can target these interfaces without redesigning them immediately

## Card P1-03 - Build the interactive REPL skeleton

### Status

`todo`

### Priority

High

### Size

M

### Depends on

- P1-01
- P1-02

### Goal of subphase

Create the shell loop that reads input, handles session lifecycle, and delegates work to parser and runtime layers.

### The details

1. Integrate `reedline` into the binary.
2. Implement the shell loop with these steps:
   - render prompt
   - read line
   - handle EOF
   - ignore or handle empty input
   - send text to parser entrypoint
   - send parsed command to executor entrypoint
   - update exit status
   - redraw prompt
3. Add safe handling for recoverable errors.
4. Ensure the shell does not terminate on normal parse or runtime failures.
5. Keep loop orchestration separate from parser and executor logic.
6. Add clean shell shutdown for EOF and `exit`.

### Tests to implement for the current subphase

- PTY test: shell launches and waits for input
- PTY test: empty line redraws prompt
- PTY test: EOF exits cleanly
- PTY test: explicit `exit` terminates session cleanly

### Definition of done

- The shell can be opened and interacted with manually
- The REPL loop survives normal user mistakes without crashing
- Control flow is clear and test-covered

## Card P1-04 - Add fallback prompt behavior

### Status

`todo`

### Priority

Medium

### Size

S

### Depends on

- P1-03

### Goal of subphase

Provide a safe internal prompt so the shell is always usable even before custom prompts exist.

### The details

1. Implement a prompt provider that returns `$ `.
2. Ensure prompt rendering is cheap and side-effect free.
3. Make prompt rendering a separate concern from reading input.
4. Leave clear extension points for right prompt, multiline prompt, and transient prompt later.
5. Ensure prompt failure cannot crash the shell.

### Tests to implement for the current subphase

- PTY test: prompt shows `$ `
- Unit test: fallback prompt renderer returns the expected string
- PTY test: prompt still appears after command execution

### Definition of done

- The shell always has a working prompt
- Prompt rendering is isolated enough for later customization

## Card P1-05 - Add persistent history

### Status

`todo`

### Priority

High

### Size

M

### Depends on

- P1-02
- P1-03

### Goal of subphase

Store accepted commands across sessions and provide baseline history access.

### The details

1. Choose the persistent history file location.
2. Load history on startup.
3. Append accepted commands at the correct point in the REPL lifecycle.
4. Skip empty or useless history entries.
5. Warn and continue on history-file errors.
6. Implement baseline history retrieval for later UI integration.
7. Add the `history` builtin with practical minimum behavior.

### Tests to implement for the current subphase

- Unit test for history path resolution
- Unit test for blank command filtering
- Integration test for persistence across sessions
- PTY test for `history` output

### Definition of done

- History persists across restarts
- History failures do not brick the shell
- The `history` builtin is usable

## Card P1-06 - Port the baseline builtins

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P1-02
- P1-03

### Goal of subphase

Port the current builtin command set and wire it through the registry and shell state.

### The details

1. Implement builtin dispatch by command name.
2. Port these builtins first:
   - `cd`
   - `exit`
   - `clear`
   - `type`
   - `echo`
   - `pwd`
   - `history`
3. Ensure builtins can read and mutate shell state where appropriate.
4. Return consistent exit codes.
5. Report usage failures through stderr and exit status instead of panicking.
6. Keep builtin implementations isolated from REPL code.

### Tests to implement for the current subphase

- Unit tests for builtin registry lookup
- Integration tests for each builtin
- PTY tests for `cd` followed by `pwd`
- Negative tests for invalid builtin usage

### Definition of done

- Baseline builtins work interactively and in automated tests
- Builtins are easy to extend later

## Card P1-07 - Implement simple-command lexing and parsing

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P1-02
- P1-03

### Goal of subphase

Support parsing of simple commands with enough quoting behavior to replace the current C shell baseline.

### The details

1. Create a minimal AST for simple commands.
2. Implement tokenization for:
   - words
   - whitespace
   - single quotes
   - double quotes
   - basic backslash escaping
3. Preserve grouping of quoted words into single argv entries.
4. Return structured parse errors for malformed input.
5. Keep parser entrypoints compatible with later expansion into full shell grammar.
6. If useful, keep source spans for debugging and later highlighting.

### Tests to implement for the current subphase

- Unit tests for plain-word tokenization
- Unit tests for single-quoted parsing
- Unit tests for double-quoted parsing
- Unit tests for escaped characters
- Unit tests for unterminated-quote errors
- Integration tests verifying parsed argv seen by executed commands

### Definition of done

- Simple commands parse reliably
- Quoted arguments behave correctly for the supported scope
- Parse failures are recoverable

## Card P1-08 - Implement external command execution

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P1-02
- P1-07

### Goal of subphase

Run non-builtin commands through `PATH` and propagate status back into shell state.

### The details

1. Resolve command names against `PATH`.
2. Spawn external commands with inherited environment.
3. Respect current working directory changes.
4. Capture command exit status.
5. Report command-not-found clearly.
6. Distinguish command-not-found from execution failure.
7. Keep executor code separate from parsing and REPL concerns.
8. Preserve async-friendly internal boundaries.

### Tests to implement for the current subphase

- Integration test for a known system command
- Integration test for command-not-found behavior
- Integration test for exit-code propagation
- PTY test for running an external command interactively

### Definition of done

- External commands run reliably
- Exit status updates are correct
- Builtin and external dispatch share one command path

## Card P1-09 - Add baseline quality gates

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P1-03
- P1-05
- P1-06
- P1-07
- P1-08

### Goal of subphase

Lock in the minimum test and benchmark workflow so Phase 2 starts on a stable base.

### The details

1. Add startup-latency benchmark scaffolding.
2. Add simple command-dispatch benchmark scaffolding.
3. Document local validation commands.
4. Ensure unit, integration, PTY, and benchmark commands all work from a clean tree.
5. Add any missing smoke tests uncovered during assembly.

### Tests to implement for the current subphase

- Benchmark harness smoke test
- Full local validation flow smoke test
- Release-build smoke test

### Definition of done

- Developers can build, test, and benchmark Phase 1 predictably
- Phase 2 can begin without missing core process pieces
