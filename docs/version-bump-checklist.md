# RustDroid Version Bump Checklist

Run this before pushing a new release tag.

## Checklist

- Confirm `Cargo.toml` and `Cargo.lock` are up to date for the intended version.
- Run `cargo fmt --check`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo test`.
- Run `cargo build --locked`.
- Run `./scripts/ci-shell-check.sh`.
- Run `./scripts/ci-package-check.sh x86_64-unknown-linux-musl <version>`.
- Run `./scripts/ci-host-check.sh` on a machine with a working host emulator lane.
- Review `dist/` contents and checksum output.
- Review the generated release notes body and install snippet.
- Push the release tag only after the release assets look correct.
