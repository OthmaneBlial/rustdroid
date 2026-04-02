# RustDroid Package Distribution

RustDroid ships through two package channels:

## Current Ready Paths

- Release archives attached to GitHub releases.
- `cargo install --path .` for local source installs.
- `cargo publish --dry-run` validation in the package-check lane to keep crates.io publication ready.

## crates.io Readiness

- `cargo package`, `cargo publish --dry-run`, and `cargo install --path .` are verified locally and in the package-validation path.
- A `cargo search rustdroid` check on April 2, 2026 returned no existing result locally. Treat that as an inference, not a guarantee. Re-check immediately before the actual publish.
- Actual crates.io publication still requires a maintainer token and a final name-availability check.

## Deferred Package Targets

- Linux package managers stay out of scope until maintenance cost is clearly low.
- No package target should be added only to populate a sidebar or badge.
