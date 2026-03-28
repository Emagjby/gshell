# Phase 3 Meaning

## Name

Advanced Shell Features

## Why this phase exists

Phase 3 adds the advanced language and expansion features that move `gshell` from a good interactive shell to a serious bash-compatible environment.

These features are the ones that usually force redesigns if the earlier architecture is weak. The purpose of this phase is to implement them on top of the stable foundation from Phases 1 and 2.

## What must exist when this phase ends

- Heredocs
- Command substitution
- Subshell execution
- Aliases with workable expansion semantics
- Shell functions
- Globbing
- Stronger compatibility coverage for shell programming patterns

## What is intentionally deferred

- Full job control polish and release-hardening tasks from Phase 4
- Plugin system work
- Vi mode

## Quality bar

By the end of Phase 3, `gshell` should handle a large share of real bash-like command composition and scripting patterns used by advanced users.
