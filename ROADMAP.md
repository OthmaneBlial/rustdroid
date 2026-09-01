# RustDroid Roadmap

> Make RustDroid the smallest trustworthy gate between an Android build and deeper UI or device-cloud testing: **artifact in, launch receipt out**.

This roadmap is about activation, trust, and reuse. RustDroid already has a broad command surface; adding another backend or test language is less valuable than making the first useful result obvious, repeatable, and easy to adopt.

## Current snapshot -- 2026-09-01

| Area | Current evidence | Honest gap |
| --- | --- | --- |
| Product | APK, split APK, `.apks`, and `.xapk` inputs; host and Docker runtimes; install, launch, watch, logs, diagnostics, profiles, and passed/failed receipts | Input-preflight failure is fixture-tested; the remaining failure stages still need deterministic end-to-end fixtures, and backend connection can fail before the orchestrator owns a receipt directory |
| First success | Linux quickstart, non-destructive setup plan, public APK fixture, rewritten README, readable demo, and checked-in [real Gradle receipt](docs/receipts/reference-gradle.md) from `v0.3.1` | A new maintainer has not yet published a timed, consented fresh-machine onboarding session |
| Public proof | Three public source stacks, reproducible timings, immutable action examples, release checksums, and successful Android 35/KVM runs | Independent user workflows and quotes still require real adopters and consent |
| Distribution | Verified x86_64 Linux release plus source install and a reusable root action | GitHub Marketplace and crates.io are prepared paths, not published channels |
| Community | 100% community-health files, Discussions, private vulnerability reporting, templates, and focused repository topics | The project still has 2 stars, 0 forks, and no verified external adopter |
| Automation | Workflow definitions and prior passing evidence remain versioned | GitHub Actions are temporarily disabled in repository settings at the owner's request; no badge should imply a currently running default gate |

Recent public evidence includes the [September 1 Gradle/Flutter/Expo matrix](https://github.com/OthmaneBlial/rustdroid/actions/runs/33519017529), the earlier [fresh-machine contract](https://github.com/OthmaneBlial/rustdroid/actions/runs/32905479252), [pinned action contract](https://github.com/OthmaneBlial/rustdroid/actions/runs/32907241889), and three complete host proofs: [one](https://github.com/OthmaneBlial/rustdroid/actions/runs/32905475830), [two](https://github.com/OthmaneBlial/rustdroid/actions/runs/32907975602/attempts/2), and [three](https://github.com/OthmaneBlial/rustdroid/actions/runs/32915494009).

## Can this earn GitHub stars?

Yes, the problem is real and the product has a credible wedge: developers already invest heavily in emulator provisioning, KVM, clean state, timeouts, ADB glue, and failure diagnosis. RustDroid can be useful because it accepts a built artifact without requiring a UI-test script and produces one normalized evidence bundle.

That does not justify a star forecast. Stars are a lagging signal, not a deliverable. RustDroid must first prove that unfamiliar users can install it, reach a receipt, understand failures, and reuse the action. External adoption is not a build artifact.

## What not to build or promote

- No UI assertion language, recorder, or attempt to replace Maestro, Appium, Espresso, or Gradle Managed Devices.
- No remote device inventory, hosted dashboard, iOS promise, or imitation device farm.
- No new backend until host and Docker receipt behavior are consistent and independently used.
- No performance superlatives without a reproducible environment, sample count, and variance.
- No reliability badge while repository automation is disabled or the scheduled evidence clock is paused.
- No extra documentation page unless it removes a real onboarding ambiguity; prefer improving the first-install, troubleshooting, recipes, or receipt guides.
- No package-manager badge before that package is live and a clean install has been verified.
- No paid, automated, reciprocal, or otherwise synthetic stars. Do not promise a star count.

## P0 -- Make every result trustworthy

### 1. One failure receipt contract

Successful runs already emit JSON, HTML, JUnit, Markdown, and logs. The next product release should make failures equally portable.

The current implementation writes a path-free schema-v1 failure receipt after backend selection and fixture-tests the `input_preflight` path. The remaining work below is broader stage coverage, not a claim that failure receipts are absent.

- Emit a canonical `run-summary.json` whenever RustDroid has enough context to do so, including failed preflight, emulator boot, ADB readiness, install, launch, crash/ANR, capture, and cleanup stages.
- Add stable fields for `status`, `failure_stage`, `failure_classification`, safe error context, and the last completed stage.
- Keep local paths, APK contents, tokens, and uncontrolled log excerpts out of the canonical JSON.
- Render the same failure accurately in HTML, JUnit, and Markdown.
- Add fixture-backed contracts for every failure class and reject unknown schema majors.

**Done when:** a CI consumer can route or compare a failed run without scraping console text, and each supported failure class has a deterministic test.

### 2. Prove first success on a clean Linux host

- Record one silent, captioned, real terminal walkthrough from release install to public fixture receipt.
- Measure time to first receipt separately from Android SDK/AVD provisioning time.
- Validate every copied command from a clean checkout and the published release archive.
- Keep the current generated walkthrough as a readable overview, but label generated and recorded proof distinctly.

**Done when:** an unfamiliar Linux user can follow one page without choosing a backend, profile, artifact format, or report format before the first pass.

### 3. Re-enable automation intentionally

This is an owner-controlled gate. Until it happens, local validation is mandatory before every push.

- Re-enable GitHub Actions only when the owner wants automated gates again.
- Start with fast checks, dependency security, and the action contract; schedule expensive emulator matrices separately.
- Require four consecutive scheduled host runs before restoring a reliability badge.
- Preserve actionable artifacts for emulator, ADB, install, launch, capture, and infrastructure failures.

**Done when:** the enabled default checks are useful, consistently maintained, and do not advertise reliability beyond their evidence.

## P1 -- Put RustDroid where Android CI is assembled

### 1. GitHub Marketplace

The existing composite action is the best-fit distribution surface because it places the receipt contract beside Android emulator provisioning.

- Validate one copyable consumer workflow from APK build through KVM/AVD provisioning, RustDroid, and artifact upload.
- Keep the listing narrow: install/launch evidence on a prepared Linux/KVM runner.
- Confirm a unique Marketplace name, category, tagged release, two-factor authentication, and the Marketplace Developer Agreement.
- Publish only after the owner completes those account-level gates.

**Done when:** a repository that does not contain RustDroid source can add the action from Marketplace and receive a passed receipt from its own APK.

### 2. crates.io

- Re-check the live crate name immediately before publication.
- Inspect `cargo package --list`, run `cargo publish --dry-run`, and verify the package stays within registry limits.
- Align the crate version, Git tag, release notes, and tested source install.
- Treat the token and irreversible publication decision as maintainer-only actions.

**Done when:** `cargo install rustdroid --locked` succeeds from a clean supported environment and the README can truthfully show the command.

### 3. Defer low-signal packaging

- Keep GitHub Releases as the binary source of truth.
- Add GHCR only if a measured Docker workflow becomes materially simpler.
- Add a Homebrew tap only after repeated user requests; do not pursue Homebrew Core as an early discovery tactic.

## P2 -- Earn independent workflows

- Help three external repositories produce a receipt from Gradle, Flutter, Expo/React Native, or an artifact-only pipeline.
- Ask for the smallest shareable receipt or issue link, never a private APK or private logs.
- Convert each consented setup failure into a troubleshooting improvement or stable diagnostic ID.
- Publish focused technical material: KVM/emulator diagnosis, artifact-to-receipt CI, split APK handling, and failure-receipt design.
- Measure successful third-party receipts, repeat users, actionable issues, and accepted integrations before referral traffic or stars.

**Done when:** three independent workflows are publicly verifiable and at least one contribution or documentation correction comes from outside the owner account.

## P3 -- Improve the artifact gate without broadening it

Consider these only after P0 evidence is complete:

- A fast `inspect` path that reports package, launcher, ABI, splits, digest, and archive structure without booting an emulator.
- Receipt comparison that highlights environment drift and stage-duration changes without uploading data.
- Redaction controls and a receipt privacy audit for teams that upload logs.
- A stable machine-readable capability command for integrations and package managers.

Do not turn these into a general Android automation platform.

## Measures

Review monthly and publish only evidence that can be linked or reproduced.

| Measure | Why it matters | Evidence |
| --- | --- | --- |
| Time to first receipt | Tests whether onboarding is actually short | Timed clean-machine session with environment details |
| Receipt completion rate by stage | Shows where users get stuck | Redacted, consented issues or CI workflows |
| Independent workflows | Proves reuse outside the owner repository | Public links or maintainer-confirmed examples |
| Repeat usage | Separates a trial from a useful tool | Returning workflow runs or follow-up reports |
| Action/package installs | Validates distribution choices | Marketplace/registry data after publication |
| Stars and forks | Useful attention signal only | GitHub insights, interpreted after activation evidence |

## Local release gate while Actions are disabled

Run from a clean checkout before pushing product or release changes:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
./scripts/ci-shell-check.sh
node --check site/app.js
node --check site/docs.js
```

Host-emulator, package-container, release-attestation, Marketplace, crates.io, and independent-adopter gates remain separate. A green local suite cannot substitute for them.
