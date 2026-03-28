# Phase 4 Meaning

## Name

Job Control, Hardening, and Release Readiness

## Why this phase exists

Phase 4 finishes the shell as a real daily-driver release candidate. It focuses on process control, compatibility hardening, performance tuning, packaging, and overall reliability.

This phase turns a feature-rich shell into a shell users can actually install, trust, and keep open all day.

## What must exist when this phase ends

- Job control with `jobs`, `fg`, `bg`, and signal-aware foreground handling
- Better compatibility coverage across common bash behaviors
- Performance work on startup, prompt rendering, parse speed, and command dispatch
- Packaging for `cargo install`, Homebrew, and release binaries
- Clear release validation and operational documentation

## Quality bar

By the end of Phase 4, `gshell` should be a believable v1.0 release candidate for macOS and Linux users.
