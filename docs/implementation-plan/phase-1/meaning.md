# Phase 1 Meaning

## Name

Foundation and Interactive Skeleton

## Why this phase exists

Phase 1 creates the minimum shell that can start, read input, run commands, and hold state correctly. It is the platform all later phases depend on.

This phase is not about full bash compatibility yet. It is about building the shell spine:

- project structure
- core state model
- REPL loop
- history
- prompt fallback
- builtin registry
- external command execution
- baseline tests and benchmarks

If Phase 1 is done well, later syntax and runtime work can plug into stable interfaces instead of forcing rewrites.

## What must exist when this phase ends

- A Rust workspace or crate layout that matches the architecture in `docs/SPEC.md`
- A binary that launches into an interactive shell
- A shell state object that owns environment, cwd view, aliases map placeholder, function placeholder, history handle, and last exit status
- A prompt system with a safe internal fallback prompt of `$ `
- Persistent history across sessions
- A trait-based builtin system with the initial builtin set wired in
- External command execution through `PATH`
- Correct exit status propagation for simple commands
- A minimal parser path for simple commands, quotes, and argv splitting sufficient to replace the current C shell baseline
- Unit, integration, and PTY tests for the above

## What is intentionally deferred

- Pipelines
- boolean operators
- redirects beyond any tiny bootstrap support needed internally
- heredocs
- command substitution
- subshells
- globbing
- aliases with full expansion behavior
- functions
- job control
- Starship integration
- completion and syntax highlighting polish

## Quality bar

Phase 1 is complete only when the shell is stable enough to use for simple commands every day while serving as a reliable base for Phase 2.
