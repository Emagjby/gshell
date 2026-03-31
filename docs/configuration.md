# Startup and configuration

## Startup file

`gshell` loads `~/.gshrc` at startup when the file exists.

- It is executed as shell code.
- `source ~/.gshrc` reloads it inside a running session.
- If the file does not exist, startup continues normally.

Example `~/.gshrc`:

```sh
alias ll='ls -lah'
alias gs='git status -sb'
```

## Prompt configuration

Prompt behavior is controlled with environment variables:

- `GSHELL_PROMPT=auto|internal|starship`
- `GSHELL_STARSHIP_BIN=/path/to/starship`

Defaults:

- `GSHELL_PROMPT=auto`
- `GSHELL_STARSHIP_BIN=starship`

`auto` tries Starship first and falls back to the internal prompt when Starship is missing or fails.

## Syntax highlighting configuration

These environment variables override the interactive highlighter colors:

- `GSHELL_HIGHLIGHT_COMMAND`
- `GSHELL_HIGHLIGHT_BUILTIN`
- `GSHELL_HIGHLIGHT_ARGUMENT`
- `GSHELL_HIGHLIGHT_FLAG`
- `GSHELL_HIGHLIGHT_HINT`
- `GSHELL_HIGHLIGHT_OPERATOR`
- `GSHELL_HIGHLIGHT_REDIRECT`

Values may be named colors such as `blue` or `light-purple`, or RGB hex like `#31748f`.

## History

History is stored at:

- `$XDG_DATA_HOME/gshell/history.txt` when `XDG_DATA_HOME` is set
- `~/.local/share/gshell/history.txt` otherwise

The parent directory is created automatically.

## Logging

Tracing uses `RUST_LOG`.

Example:

```sh
RUST_LOG=gshell=debug gshell
```
