# Contributing

RustDroid is a fast-loop tool first.

Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), [SUPPORT.md](SUPPORT.md), and [SECURITY.md](SECURITY.md) before opening a public discussion or issue. Questions and workflow recipes belong in GitHub Discussions; reproducible bugs and setup failures belong in the issue forms; security concerns must be reported privately.

When in doubt, prefer:

- lower latency
- simpler CLI behavior
- deterministic output
- fewer moving parts

## Local Workflow

```bash
cargo fmt
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
./scripts/ci-shell-check.sh
```

For host runtime changes, also run a host smoke:

```bash
./scripts/run-smoke-matrix.sh --skip-build
```

## Change Rules

- do not add features that make the normal APK loop slower without a clear payoff
- keep host backend behavior stable
- keep command output understandable
- preserve reproducible CI and release paths
- include a dry-run, fixture, or host-evidence command for runtime changes
- do not attach private APKs, keys, unredacted logs, or full private filesystem paths

## Starter contributions

Look for `good first issue` or `help wanted`. A starter issue should state scope, acceptance criteria, labels, and a reproducible command before work begins. `docs`, `ci`, `host-runtime`, `fixtures`, and `release` describe the main contribution areas.

## Fixtures

Use `tests/fixtures/` for deterministic coverage instead of depending on a live app build whenever possible.

## Releases

Follow `docs/release-process.md`, [docs/release-security-checklist.md](docs/release-security-checklist.md), and the release checklists already in the repo.
