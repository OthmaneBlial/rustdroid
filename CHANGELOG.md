# Changelog

## Unreleased

- Added schema-v1 failure receipts for input preflight, emulator boot, install, launch, runtime observation, artifact capture, and cleanup failures after backend selection. JSON, HTML, JUnit, and Markdown now report the same stable stage and path-free summary.
- Made runtime artifact collection best-effort so an unavailable log source does not erase the primary run failure, while still marking a requested capture failure when no earlier failure exists.
- Escaped ABI values in generated HTML receipts and rendered an empty ABI set as text instead of an invalid `<none>` element.

## v0.3.1

- Fixed the release checksum manifest so downloaded assets can be verified from any directory. The v0.3.0 archive itself is valid, but its attached checksum file is not portable; use v0.3.1 for verified installs.

## v0.3.0

- Receipt-first v0.3.0 release preparation: guided Linux setup, stable diagnostics, dry-run plans, reusable CI receipts, and reproducible benchmark artifacts.
- Trust and contributor foundations: public support/security paths, community templates, CodeQL, Dependabot, and `cargo deny` checks.
- Distribution clarity: x86_64 Linux release artifacts, source-only ARM fallback, and an explicit non-publishing crates.io readiness gate.

## v0.2.0

- Added archive-aware install flows for split APKs, `.apks`, and `.xapk` packages.
- Added `watch`, `launch`, `open`, `clear-data`, and `uninstall` commands for faster local rerun loops.
- Added profile inheritance, environment overrides, JSON output, and richer run artifacts.
- Added release packaging checks, install verification, split CI lanes, and more contributor-facing documentation.

## v0.1.0

- First tagged public release.
- Added release notes, install verification, release checklists, and rollback guidance.
- Published the first release asset flow and install snippet generation.
