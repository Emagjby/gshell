# Phase 4 Board

## How to use this board

- This file is both the ordered todo list and the kanban board for Phase 4.
- Cards are ordered by technical dependency and release risk.
- Default status for all cards at phase start is `todo`.

## Ordered cards

## Card P4-01 - Build the job table and process-group model

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-04

### Goal of subphase

Create the internal process and job model required for real shell job control.

### The details

1. Define job-table structures for job id, process group id, summary, and state.
2. Track foreground and background state.
3. Implement process-group creation for pipelines and spawned jobs.
4. Separate bookkeeping from user-facing formatting.
5. Keep Unix behavior portable across macOS and Linux where possible.

### Tests to implement for the current subphase

- Unit tests for job insertion and updates
- Unit tests for state transitions
- Integration tests for process-group creation where practical

### Definition of done

- The shell has stable internal primitives for job control

## Card P4-02 - Implement signal-aware foreground control

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P4-01

### Goal of subphase

Hand terminal control to foreground jobs correctly and recover it after they stop or exit.

### The details

1. Implement terminal handoff to foreground process groups.
2. Handle `Ctrl-C` for foreground jobs.
3. Handle `Ctrl-Z` for foreground jobs.
4. Restore shell terminal control afterward.
5. Keep prompt redraw stable after signal-driven state changes.

### Tests to implement for the current subphase

- PTY tests for `Ctrl-C`
- PTY tests for `Ctrl-Z`
- PTY tests for shell recovery after signal events

### Definition of done

- Foreground signal behavior matches user expectations for a real shell

## Card P4-03 - Implement `jobs`, `fg`, and `bg`

### Status

`todo`

### Priority

High

### Size

M

### Depends on

- P4-01
- P4-02

### Goal of subphase

Expose job control through user-facing builtins.

### The details

1. Implement `jobs` output.
2. Implement `fg` to resume and foreground a job.
3. Implement `bg` to resume in the background.
4. Define error handling for invalid job references.
5. Keep job-selection syntax small and well documented for v1.0.

### Tests to implement for the current subphase

- Integration tests for `jobs`
- PTY tests for stopping and resuming with `bg`
- PTY tests for bringing jobs back with `fg`
- Negative tests for invalid job ids

### Definition of done

- Users can inspect and control jobs during normal shell use

## Card P4-04 - Run compatibility hardening

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P3-07
- P4-03

### Goal of subphase

Reduce semantic gaps between `gshell` and the intended bash-compatible behavior for normal workflows.

### The details

1. Create a compatibility checklist from `docs/SPEC.md`.
2. Add comparison tests for lists, pipelines, redirects, expansions, heredocs, aliases, functions, and job control basics.
3. Fix true bugs.
4. Document intentional differences.
5. Add regressions for every fixed compatibility issue.

### Tests to implement for the current subphase

- Compatibility suite across the major v1.0 features
- Regression tests for fixed incompatibilities
- PTY tests for compatibility-sensitive interactive cases

### Definition of done

- The shell has a clear, tested compatibility story

## Card P4-05 - Optimize performance hot paths

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P2-07
- P2-08
- P2-09
- P4-04

### Goal of subphase

Make the shell feel fast enough to be kept open all day without friction.

### The details

1. Benchmark startup latency.
2. Benchmark prompt rendering latency.
3. Benchmark parse speed.
4. Benchmark completion latency.
5. Benchmark command-dispatch overhead.
6. Profile hot paths.
7. Optimize without making the codebase opaque.
8. Re-check prompt, completion, and shared-state lock usage for avoidable stalls.

### Tests to implement for the current subphase

- Benchmark comparisons before and after optimization
- Smoke tests for optimized code paths
- Regression tests for performance-related fixes

### Definition of done

- Performance is measured and acceptable on the main hot paths

## Card P4-06 - Finalize packaging and install paths

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P4-04

### Goal of subphase

Ship the shell through the planned installation channels with repeatable release steps.

### The details

1. Ensure `cargo install` works cleanly.
2. Prepare release binaries for macOS and Linux.
3. Prepare Homebrew packaging metadata.
4. Write installation docs.
5. Write startup and configuration docs.
6. Define a release checklist.

### Tests to implement for the current subphase

- Install smoke test for `cargo install`
- Release-build smoke tests
- Packaging validation for Homebrew metadata where practical

### Definition of done

- Users can install `gshell` through the planned channels

## Card P4-07 - Run final release-candidate validation

### Status

`todo`

### Priority

High

### Size

M

### Depends on

- P4-05
- P4-06

### Goal of subphase

Validate the shell end to end before calling it v1.0-ready.

### The details

1. Run unit, integration, PTY, compatibility, and benchmark suites.
2. Test interactive workflows manually on macOS and Linux.
3. Validate startup with and without config.
4. Validate prompt behavior with and without Starship.
5. Validate history, multiline editing, completion, advanced syntax, and job control together.
6. Fix release blockers.
7. Record acceptable known limitations.

### Tests to implement for the current subphase

- Full release-candidate validation suite
- Manual verification checklist results
- Regression tests for all release blockers fixed here

### Definition of done

- The shell is stable enough to tag as a v1.0 release candidate
