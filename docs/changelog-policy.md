# Changelog Policy

RustDroid keeps a user-facing changelog.

The changelog should answer:

- what changed
- why it matters to daily APK testing
- whether there is any migration or operational risk

## Rules

- keep entries short and user-oriented
- group by release
- include breaking changes explicitly
- mention install, CI, performance, and recovery changes when they matter
- do not dump raw commit history into the changelog

## Format

- `Unreleased` at the top
- tagged versions below it
- short sections only when they help

The canonical file is `CHANGELOG.md`.
