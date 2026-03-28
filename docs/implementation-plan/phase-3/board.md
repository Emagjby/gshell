# Phase 3 Board

## How to use this board

- This file is both the ordered todo list and the kanban board for Phase 3.
- Cards are ordered to minimize parser and runtime rewrites.
- Default status for all cards at phase start is `todo`.

## Ordered cards

## Card P3-01 - Extend syntax for command substitution and subshells

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-02

### Goal of subphase

Represent nested command execution forms correctly in the lexer, parser, and AST.

### The details

1. Add nested `$(...)` lexing support.
2. Track nesting depth safely.
3. Distinguish subshell grouping from command substitution.
4. Extend AST nodes to represent both constructs clearly.
5. Preserve incomplete-input detection for partially typed nested forms.

### Tests to implement for the current subphase

- Unit tests for nested `$(...)` tokenization
- Unit tests for subshell parsing
- Unit tests for incomplete nested input
- AST tests for nested execution forms

### Definition of done

- Nested shell execution syntax can be parsed without ambiguity

## Card P3-02 - Implement command substitution runtime

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P3-01

### Goal of subphase

Execute nested commands and substitute their stdout into surrounding words.

### The details

1. Define the execution path for command substitution.
2. Run the nested command in the right shell context.
3. Capture stdout safely.
4. Normalize or trim output according to compatibility rules.
5. Reinsert substituted text into expansion processing.
6. Handle failures and avoid deadlocks.

### Tests to implement for the current subphase

- Integration tests for simple substitution
- Integration tests for nested substitution
- Integration tests for quoted and unquoted substitution
- Negative tests for failed nested commands

### Definition of done

- Command substitution works in practical daily usage cases

## Card P3-03 - Implement heredocs

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-02
- P2-06

### Goal of subphase

Support `<<` with correct parse-time collection and runtime stdin feeding.

### The details

1. Add heredoc descriptors to parser output.
2. Add the follow-up phase that collects heredoc bodies after command structure is known.
3. Implement delimiter handling.
4. Define how delimiter quoting affects expansion inside heredoc bodies.
5. Feed heredoc contents into the target command's stdin.
6. Support multiple heredocs predictably.
7. Keep interactive entry behavior sane.

### Tests to implement for the current subphase

- Unit tests for heredoc descriptor parsing
- Integration tests for basic heredoc execution
- Integration tests for quoted and unquoted delimiters
- Integration tests for multiple heredocs
- PTY tests for interactive heredoc entry

### Definition of done

- Heredocs work reliably in both automated and interactive flows

## Card P3-04 - Implement globbing

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P2-05

### Goal of subphase

Expand wildcard patterns into filesystem matches in the correct phase of execution.

### The details

1. Support `*`, `?`, and character classes.
2. Run glob expansion after parsing and before final argv execution.
3. Respect quoting so quoted wildcards remain literal.
4. Define behavior for unmatched patterns.
5. Keep match ordering deterministic where practical.
6. Verify expansion order relative to env vars and command substitution.

### Tests to implement for the current subphase

- Integration tests for `*`
- Integration tests for `?`
- Integration tests for character classes
- Integration tests for quoted wildcards
- Integration tests for unmatched patterns

### Definition of done

- Wildcard expansion works in common shell workflows

## Card P3-05 - Implement aliases

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P1-02
- P2-02

### Goal of subphase

Add alias storage and expansion behavior that is useful, bounded, and predictable.

### The details

1. Implement `alias` and `unalias` builtins.
2. Add alias state storage.
3. Define when alias expansion runs.
4. Restrict expansion to valid command positions.
5. Prevent infinite recursion.
6. Keep the design compatible with later function and completion lookups.

### Tests to implement for the current subphase

- Unit tests for alias storage
- Integration tests for alias expansion in command position
- Negative tests for recursion prevention
- Integration tests for alias interaction with quoting

### Definition of done

- Aliases are usable in daily work without surprising recursion bugs

## Card P3-06 - Implement shell functions

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-02
- P3-05

### Goal of subphase

Support reusable shell-defined commands with proper lookup and execution behavior.

### The details

1. Parse function definitions in the chosen syntax.
2. Store function bodies in shell state.
3. Define lookup precedence among aliases, functions, builtins, and external commands.
4. Execute functions using the existing runtime machinery.
5. Define recursion guardrails.
6. Keep behavior aligned with the compatibility target.

### Tests to implement for the current subphase

- Unit tests for function-definition parsing
- Integration tests for function definition and invocation
- Integration tests for function interaction with env vars and exit status
- Negative tests for malformed function definitions

### Definition of done

- Users can define and run shell functions reliably

## Card P3-07 - Stabilize expansion order and advanced compatibility

### Status

`todo`

### Priority

High

### Size

M

### Depends on

- P3-02
- P3-03
- P3-04
- P3-05
- P3-06

### Goal of subphase

Make advanced features compose correctly instead of only working in isolation.

### The details

1. Review the full expansion order across aliases, env vars, command substitution, heredocs, and globbing.
2. Make the order explicit in code and docs.
3. Fix interaction bugs.
4. Add representative bash-compat tests.
5. Add regression coverage for every discovered interaction bug.

### Tests to implement for the current subphase

- Combined integration tests mixing env vars, substitution, and globbing
- Combined integration tests mixing aliases and functions
- Bash-compat tests for representative advanced command forms
- Regression tests for interaction bugs

### Definition of done

- Advanced features compose correctly under common usage patterns
