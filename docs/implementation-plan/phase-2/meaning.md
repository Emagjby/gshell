# Phase 2 Meaning

## Name

Core Shell Language and Interactive UX

## Why this phase exists

Phase 2 upgrades the simple shell core into a real shell language engine for daily use. It introduces the syntax and interactivity people expect before they can seriously move into `gshell` full time.

This phase covers the main command language and the first polished user experience layer.

## What must exist when this phase ends

- Full parser and AST path for pipelines, lists, boolean operators, and redirections
- Environment variable expansion and shell word handling at a practical daily-driver level
- Multiline editing driven by parser completeness
- Bash-style tab completion
- Fish-style autosuggestions via right arrow
- Semantic syntax highlighting while typing
- Optional Starship prompt integration with safe fallback
- Better compatibility coverage for common interactive workflows

## What is intentionally deferred

- Heredocs
- command substitution
- subshell execution semantics beyond any parser scaffolding if needed early
- aliases with complete expansion semantics
- functions
- globbing finalization
- job control

## Quality bar

By the end of Phase 2, `gshell` should feel like a real interactive shell for normal command entry, even if advanced shell-programming features are still incomplete.
