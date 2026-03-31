# Installation

## Supported platforms

- macOS
- Linux

## Install with Cargo

`gshell` ships as a normal Rust crate.

```sh
cargo install gshell --locked
```

For local validation during development:

```sh
cargo install --path . --locked
```

After install, verify the binary is on `PATH`:

```sh
gshell --version
```

## Install from a release binary

Tagged releases publish tarballs for:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`

Each archive contains a single `gshell` binary plus the main docs.

Example:

```sh
tar -xzf gshell-<version>-<target>.tar.gz
install "gshell-<version>-<target>/gshell" "$HOME/.local/bin/gshell"
```

## Install with Homebrew

Homebrew support is release-driven.

The repository keeps a template formula at `packaging/homebrew/gshell.rb.in`. During release publication, render it with the release version and archive checksums, then publish the result in the tap or formula repository you use for distribution.

After the formula is published:

```sh
brew install gshell
```

## First start

Start an interactive session with:

```sh
gshell
```

If `~/.gshrc` exists, it is sourced during startup.
