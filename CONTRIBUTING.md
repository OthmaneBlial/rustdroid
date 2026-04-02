# Contributing

RustDroid is a fast-loop tool first.

When in doubt, prefer:

- lower latency
- simpler CLI behavior
- deterministic output
- fewer moving parts

## Local Workflow

```bash
cargo fmt
cargo test
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

## Fixtures

Use `tests/fixtures/` for deterministic coverage instead of depending on a live app build whenever possible.

## Releases

Follow `docs/release-process.md` and the release checklists already in the repo.
