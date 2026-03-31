# gshell

`gshell` is a Rust shell for people who live in the terminal. It aims for familiar Unix behavior, fast interactive use, and a small core that is easy to extend.

## Install

### Cargo

```sh
cargo install gshell --locked
```

### Release binaries

Prebuilt tarballs for macOS and Linux are published from tagged releases.

### Homebrew

Homebrew packaging metadata lives in `packaging/homebrew/gshell.rb.in` and is rendered during release publication.

## Startup

`gshell` loads `~/.gshrc` when that file exists.

Useful docs:

- `docs/install.md`
- `docs/configuration.md`
- `docs/release-checklist.md`

## Development

```sh
make validate
```
