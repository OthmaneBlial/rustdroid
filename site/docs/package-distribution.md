# RustDroid Package Distribution

RustDroid ships through two package channels:

## Current Ready Paths

- Checked and provenance-attested `x86_64-unknown-linux-musl` release archives attached to GitHub releases.
- The release installer drops both `rustdroid` and `rustdroid-run`.
- `cargo install --path .` for local source installs.
- `cargo publish --dry-run` validation in the package-check lane to keep crates.io publication readiness visible.
- A clean Debian container verifies archive extraction, checksum validation, install, completions, helper help, and the machine-readable `doctor` contract for every package-validation/release run.

## Binary Architecture Policy

- Published prebuilt archives currently target **x86_64 Linux** only.
- The installer never requests a nonexistent ARM archive. On `aarch64`/ARM Linux, automatic installation falls back to a source build and `--release` explains that a prebuilt asset is unavailable.
- Add a new architecture only together with a tested build target, release asset, checksum, container installation check, and support-matrix update.

## crates.io Readiness

- `cargo package`, `cargo publish --dry-run`, and `cargo install --path .` are verified locally and in the package-validation path.
- A `cargo search rustdroid` check on April 2, 2026 returned no existing result locally. Treat that as an inference, not a guarantee. Re-check immediately before the actual publish.
- Actual crates.io publication is intentionally not claimed until a maintainer token, final name-availability check, and a tagged verified release are present.

## Deferred Package Targets

- Linux package managers stay out of scope until maintenance cost is clearly low.
- No package target should be added only to populate a sidebar or badge.
