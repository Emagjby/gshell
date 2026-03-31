# Release checklist

## Before tagging

1. Run `make validate`.
2. Run `cargo install --path . --locked --root "$(mktemp -d)/gshell-install"`.
3. Run `cargo build --release --locked` on macOS and Linux, or let CI cover both platforms.
4. Confirm `gshell --version` reports the expected crate version.
5. Review `README.md`, `docs/install.md`, and `docs/configuration.md` for release accuracy.

## Cut the release

1. Create and push a `v<version>` tag.
2. Wait for the release workflow to publish archives for each supported target.
3. Capture the generated SHA-256 values for each archive.

## Publish packaging metadata

1. Render `packaging/homebrew/gshell.rb.in` with the new version and release checksums.
2. Run `ruby -c` on the rendered formula.
3. Publish the formula to the Homebrew tap or formula repository.

## Final verification

1. Install via `cargo install gshell --locked`.
2. Install via the release tarball and run `gshell --version`.
3. Install via Homebrew and run `gshell --version`.
4. Launch `gshell`, confirm `~/.gshrc` loads, and run a simple command such as `echo ok`.
