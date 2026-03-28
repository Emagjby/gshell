# Phase 2 Board

## How to use this board

- This file is both the ordered todo list and the kanban board for Phase 2.
- Move through the cards in order unless a dependency note says otherwise.
- Default status for all cards at phase start is `todo`.

## Ordered cards

## Card P2-01 - Expand tokens and AST for shell operators

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P1-07

### Goal of subphase

Extend the syntax model so the shell can represent command composition instead of only simple argv commands.

### The details

1. Add token support for `|`, `&&`, `||`, `;`, `>`, `>>`, `<`, fd redirects, and grouping parentheses.
2. Extend AST nodes for:
   - pipelines
   - boolean chains
   - sequential lists
   - redirections
   - grouped commands or subshell placeholders
3. Keep node design neutral so runtime semantics stay in the executor.
4. Preserve room for heredocs and command substitution in later phases.

### Tests to implement for the current subphase

- Unit tests for operator tokenization
- Unit tests for redirect tokenization
- AST construction tests for representative command forms

### Definition of done

- The AST can represent the core shell language needed in Phase 2

## Card P2-02 - Implement structured parsing with precedence

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-01

### Goal of subphase

Parse shell syntax into structured command trees with correct precedence and associativity.

### The details

1. Implement parser functions or combinators using `winnow`.
2. Parse the language in precedence layers:
   - lists
   - boolean chains
   - pipelines
   - commands
   - redirections
3. Attach redirects to the correct command node.
4. Distinguish invalid input from incomplete input.
5. Keep parser code explicit and debuggable.

### Tests to implement for the current subphase

- Unit tests for pipeline precedence
- Unit tests for `&&` and `||` precedence
- Unit tests for `;` sequencing
- Unit tests for redirect attachment
- Unit tests for incomplete input such as trailing pipe or unclosed group

### Definition of done

- Core shell syntax parses correctly and predictably

## Card P2-03 - Add runtime support for redirections

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-02

### Goal of subphase

Execute file-based input and output redirections correctly for both builtins and external commands.

### The details

1. Implement stdout truncate redirection.
2. Implement stdout append redirection.
3. Implement stderr truncate and append redirection.
4. Implement stdin file redirection.
5. Restore parent shell stdio after running redirected builtins.
6. Handle file-open and permission failures cleanly.
7. Keep redirection code isolated from high-level execution planning.

### Tests to implement for the current subphase

- Integration tests for `>` and `>>`
- Integration tests for `<`
- Integration tests for `2>` and `2>>`
- Integration tests for redirected builtins
- Negative tests for file failures

### Definition of done

- Redirected commands behave safely and predictably

## Card P2-04 - Implement pipelines and boolean execution

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P2-02
- P2-03

### Goal of subphase

Support real command chaining with pipes, short-circuit operators, and sequential execution.

### The details

1. Implement process wiring for pipelines.
2. Define builtin behavior inside pipelines and keep it bash-compatible where practical.
3. Implement `&&` short-circuiting.
4. Implement `||` short-circuiting.
5. Implement `;` sequential execution.
6. Ensure file descriptors and processes are always cleaned up.
7. Make resulting exit status behavior explicit.

### Tests to implement for the current subphase

- Integration tests for two-command and multi-command pipelines
- Integration tests for `&&` success and failure cases
- Integration tests for `||` success and failure cases
- Integration tests for `;` sequencing

### Definition of done

- Core command composition works for real shell usage

## Card P2-05 - Implement environment expansion

### Status

`todo`

### Priority

High

### Size

M

### Depends on

- P2-02

### Goal of subphase

Expand environment variables and status variables in a quote-aware way.

### The details

1. Define a word representation that keeps quote context until expansion.
2. Implement `$VAR`.
3. Implement `$?`.
4. Respect quoting behavior during expansion.
5. Keep expansion ordering compatible with planned later features.

### Tests to implement for the current subphase

- Unit tests for `$VAR`
- Unit tests for `$?`
- Unit tests for quoted versus unquoted behavior
- Integration tests for variable-expanded commands

### Definition of done

- Common variable expansion works predictably

## Card P2-06 - Add parser-driven multiline editing

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P2-02

### Goal of subphase

Make the editor treat incomplete input as continued input instead of hard failure.

### The details

1. Connect parser completeness checks to `reedline` validation.
2. Continue input for trailing pipes, open groups, and unterminated quotes.
3. Add multiline continuation indicators.
4. Preserve manual newline insertion behavior.
5. Keep multiline redraw behavior stable.

### Tests to implement for the current subphase

- PTY test for trailing-pipe continuation
- PTY test for unclosed-quote continuation
- PTY test for grouped-command continuation
- PTY test for successful execution after completion of multiline input

### Definition of done

- Multiline input feels like a real shell instead of a line-oriented CLI

## Card P2-07 - Implement completion and autosuggestions

### Status

`todo`

### Priority

High

### Size

L

### Depends on

- P1-05
- P1-06
- P2-06

### Goal of subphase

Add the main productivity features users will touch constantly while typing commands.

### The details

1. Implement command completion from builtins and `PATH`.
2. Implement path completion.
3. Implement environment-variable completion.
4. Structure completion sources so aliases and functions can be added later.
5. Bind `Tab` to open and cycle completion results.
6. Bind `Shift-Tab` to reverse cycle.
7. Implement fish-style autosuggestions from history.
8. Bind right arrow to accept autosuggestion only at end-of-line.
9. Keep completion and autosuggestion state separate.

### Tests to implement for the current subphase

- Unit tests for completion source resolution
- PTY tests for command completion
- PTY tests for path completion
- PTY tests for autosuggestion acceptance via right arrow

### Definition of done

- Completion and autosuggestions are useful enough for daily shell use

## Card P2-08 - Implement prompt upgrades and Starship fallback integration

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P1-04
- P2-06

### Goal of subphase

Support richer prompt rendering while keeping the shell safe and responsive.

### The details

1. Extend prompt contracts to support left, right, indicator, and multiline prompt parts.
2. Add config-level prompt selection hooks.
3. Add optional Starship invocation.
4. Fall back to internal prompt rendering if Starship fails or is missing.
5. Cache expensive prompt state during a prompt cycle.
6. Avoid blocking hot-path redraws more than necessary.

### Tests to implement for the current subphase

- Unit tests for fallback selection
- Integration test for internal prompt rendering
- Integration test for missing-Starship fallback
- PTY test for prompt redraw stability

### Definition of done

- Prompt customization works without compromising shell stability

## Card P2-09 - Implement semantic syntax highlighting

### Status

`todo`

### Priority

Medium

### Size

M

### Depends on

- P2-01
- P2-06

### Goal of subphase

Add lightweight shell-aware highlighting that improves readability while typing.

### The details

1. Highlight builtins distinctly and in bold.
2. Highlight command position differently from later arguments.
3. Highlight operators and redirections distinctly.
4. Respect terminal-theme capabilities rather than forcing a hardcoded palette.
5. Make incomplete input highlight safely.

### Tests to implement for the current subphase

- Unit tests for token-to-style mapping
- Unit tests for builtin highlighting
- Unit tests for operator highlighting
- Renderer or PTY tests for incomplete-input stability

### Definition of done

- Highlighting is useful, stable, and fast enough for interactive use
