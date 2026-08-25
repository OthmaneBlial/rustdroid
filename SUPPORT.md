# Support

Use GitHub Discussions for setup questions, workflow recipes, and ideas that are not yet reproducible defects. Use the issue forms for an actionable bug, environment/setup failure, or scoped feature proposal.

Before posting, run the narrowest useful command:

```bash
rustdroid --json doctor
rustdroid --dry-run --profile host-fast run app-debug.apk
```

Share stable doctor check IDs, RustDroid version, Linux distribution, backend, AVD/API, and redacted receipt/log excerpts. Do not publish private APKs, access tokens, or full filesystem paths.

Maintainers review support on a best-effort basis and aim to respond within seven days. GitHub Discussions are not a promise of individual support or a substitute for a security report; see [SECURITY.md](SECURITY.md).
