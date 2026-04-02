# Release Process

RustDroid releases are tag-driven.

## Before Tagging

Run the normal checks:

```bash
cargo test
./scripts/ci-shell-check.sh
./scripts/check-cargo-distribution.sh /tmp/rustdroid-cargo-install
./scripts/check-performance-baseline.sh
```

Then review:

- `docs/version-bump-checklist.md`
- `docs/release-announcement-checklist.md`
- `docs/release-rollback.md`

## Release Notes

If a curated note exists under `docs/releases/<version>.md`, the release workflow uses it directly.

Otherwise the workflow generates a default note and install snippet.

## Verify Installability

Before or during release prep:

```bash
./scripts/verify-release-install.sh x86_64-unknown-linux-musl v0.1.0
```

## Assets

The release flow is expected to publish:

- Linux archive
- checksum
- install snippet
- release notes

No silent tags. No mystery assets.
