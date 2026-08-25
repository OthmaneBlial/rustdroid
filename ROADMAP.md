# RustDroid Roadmap

> Make RustDroid the local Android APK loop that developers recommend when they need one trustworthy answer: **did this APK boot, install, launch, or fail -- and what evidence do I have?**

RustDroid should earn attention because it removes a painful step from everyday Android work, not because it imitates a device farm or chases empty star metrics. The product boundary remains deliberate: fast, local, Linux-first APK validation with a host-emulator fast path; Docker, browser, and VNC are supporting paths.

## Baseline audit snapshot -- 2026-08-25

This was the pre-delivery audit. The delivery status below supersedes its claims about missing workflows, metadata, documentation, and community health; it remains here to preserve the prioritisation rationale.

### What is already genuinely strong

- A real Rust CLI, not a wrapper mock: host and Docker runtimes, `open`, `install`, `launch`, `run`, `watch`, logs, cleanup, profiles, JSON output, diagnostics, and artifacts.
- Practical APK handling: single APKs, split installs, `.apks`, `.xapk`, ABI inspection, package resolution, data reset, and OBB staging.
- Better engineering foundations than most new CLI repositories: deterministic APK fixtures, package/release checks, a host integration lane, checksums, install/uninstall scripts, shell completions, performance guardrails, and run-summary HTML/JSON artifacts.
- A useful positioning wedge: a narrow, scriptable local loop before a team needs a device cloud. This is meaningfully different from the broader Docker Android platform.

### Why it is not yet a project people naturally star or share

1. **The central claim is not publicly trusted yet.** The last six scheduled host-integration runs failed at the host-runtime/smoke step. A recurring red check against the main value proposition is more damaging than a missing feature. Locally, `cargo fmt`, Clippy, packaging, shell checks, and the focused integration suite pass, but `cargo test` is not portable to this macOS workspace because one unit test assumes Linux `/proc/<pid>/cmdline` exists.
2. **The first successful outcome is hard to picture.** The README lists a lot of power, but starts with requirements and many modes. It has no terminal recording, no report screenshot, no tiny sample APK journey, no result card, and no before/after comparison. A visitor cannot see the payoff in the first 20 seconds.
3. **Distribution makes promises it does not fully fulfil.** The installer accepts `aarch64`, while the release workflow and `v0.2.0` assets publish only `x86_64-unknown-linux-musl`. crates.io is prepared but not a shipped channel. The repository documentation setting still points at a `master` docs URL although the default branch is `main`.
4. **Discovery and community are almost empty.** At this snapshot the public repository has 2 stars, 0 forks, 0 open issues, no GitHub topics, no Discussions, and a 57% GitHub community-health profile. It lacks issue and pull-request templates, a code of conduct, a security policy, contribution labels, and a clearly maintained public feedback path.
5. **The repository contains distracting material.** `sample-pdfs/` and its helper scripts are unrelated to Android APK testing. They make a small repository look less intentional and reduce confidence in the project story.
6. **The product surface is already broad enough.** Adding modes, flags, or a full UI-test DSL before proving activation would make the CLI harder to understand. The next gains must come from trust, activation, evidence, and reuse.

## Delivery status -- 2026-08-26

The implementation has been consolidated into [`v0.3.1`](https://github.com/OthmaneBlial/rustdroid/releases/tag/v0.3.1) rather than creating empty `v0.4.0` and `v0.5.0` tags. This status deliberately separates completed engineering work from external or time-based evidence that cannot honestly be manufactured.

| Roadmap scope | Current state | Evidence and remaining gate |
| --- | --- | --- |
| P0 host trust | Implemented and manually verified | The host runner is isolated, schedules every Monday at 06:00 UTC, emits classified failure artifacts, and passed a full manual host/smoke/performance run. Do not add a reliability badge until four consecutive scheduled runs pass. |
| P0 release and proof | Released, with one external metadata action pending | [`v0.3.1`](https://github.com/OthmaneBlial/rustdroid/releases/tag/v0.3.1) ships the x86_64 Linux archive, portable checksum, clean-container installation proof, and provenance. The generated social-preview asset is ready; it still needs an explicit repository-settings upload confirmation. |
| P1 onboarding and CI receipts | Implemented and verified | Reviewable `setup`, stable `doctor --json`, Linux quickstart/config examples, fresh-machine Android 35 fixture proof, schema v1 JSON/HTML/JUnit/Markdown receipts, and a pinned reusable action are present. |
| P2 adoption and speed evidence | Implemented and verified | Gradle, Flutter, and React Native/Expo templates now boot a persistent AVD before invoking the pinned action; recipes, generated support matrix, dry-run plans, and benchmark receipts are checked into the repository. |
| P3 contributor and security loop | Implemented and configured | Community health is 100%, Discussions and private vulnerability reporting are enabled, issue/PR templates and labelled starter work exist, and CodeQL, Dependabot, `cargo deny`, and release-security checks run in CI. |
| External adoption and public maturity | Intentionally unclaimed | Four weekly host runs, independent adopters, organic stars, community sharing, and user quotes require time and consent. They are success measures, not build artifacts, and remain tracked operating work. |

### Current evidence links

- [Host integration proof](https://github.com/OthmaneBlial/rustdroid/actions/runs/32905475830), [fresh-machine proof](https://github.com/OthmaneBlial/rustdroid/actions/runs/32905479252), and [pinned action proof](https://github.com/OthmaneBlial/rustdroid/actions/runs/32907241889).
- [v0.3.1 release workflow](https://github.com/OthmaneBlial/rustdroid/actions/runs/32906999443): archive, checksum, clean-container install, and provenance all passed. The published archive and checksum were downloaded into a clean directory and verified with `sha256sum --check`; the provenance attestation verified the release workflow and tag.
- The starter-work issues for doctor remediation, fixture documentation, and workflow contracts were implemented and closed in `5c0674b`.

## Product thesis

### The one-sentence promise

**From an APK path to a reproducible local launch receipt in one command.**

The winning experience is not merely "start an emulator." It is:

```text
APK path -> preflight -> boot/reuse -> install -> launch -> logs/artifacts -> clear result
```

The result should be useful to three audiences:

- Android developers who need a fast loop after every local build.
- Mobile CI owners who need an inexpensive pre-cloud smoke check with inspectable artifacts.
- Maintainers/reverse engineers validating APK, split APK, or XAPK artifacts without opening Android Studio.

### Explicit non-goals

- Do not become a remote device farm, an iOS tool, or a general Android automation platform.
- Do not compete with Appium, Maestro, or Firebase Test Lab by building a second UI-test language.
- Do not add invisible telemetry, paid-hosting dependencies, or a browser-first path to manufacture growth.
- Do not buy stars, automate engagement, or publish performance claims without a reproducible benchmark and environment details.

## Success measures

Metrics are decision tools, not promises. Review them monthly and publish only the evidence that can be verified.

| Area | 90-day target | Evidence |
| --- | --- | --- |
| Trust | Four consecutive green scheduled host runs; no known red default workflow | Actions history and linked failure artifacts |
| Activation | A clean supported Linux machine reaches a verified fixture run in 10 minutes or less | Fresh-machine CI/checklist and recorded walkthrough |
| Install quality | Every advertised architecture either receives a tested release archive or a clear, intentional source-only message | Release matrix and install tests |
| Reuse | Three public repositories or maintainers report using the workflow or action | Linked issues, discussions, or public references |
| Community | 90%+ community-health profile, three external feedback/contribution events, and labelled starter issues | GitHub community profile and issue history |
| Attention | 50 genuine stars is a useful early signal; pursue larger reach only after activation and trust are proven | GitHub repository insights, never paid or synthetic growth |

## Roadmap

### P0 -- Restore trust and make the payoff visible (next release, `v0.3.0`)

#### 1. Make the host fast path credibly green

This is the release blocker.

- Reproduce the scheduled `host-integration` failure locally or in an isolated Actions run. Preserve the failing `run-summary`, emulator log, logcat, and workflow log in the issue that tracks it.
- Fix the root cause, then require three manual reruns plus four scheduled weekly runs to pass before advertising a host-integration badge.
- Keep the fast checks and host lane separate, but do not let a permanently red scheduled lane silently normalize. If a hosted runner limitation remains, report it explicitly and make the failure artifact/actionable fallback obvious.
- Make the `/proc`-dependent unit test platform-aware or inject the process reader so `cargo test` is useful on supported contributor hosts. Linux remains the runtime target; contributors should not see a misleading test failure on macOS.
- Add a minimal failure taxonomy to host artifacts: emulator boot, ADB bridge, APK install, launch/foreground, log capture, cleanup, or infrastructure.

**Done when:** the default CI is green, the host history demonstrates reliability, and a failure gives a contributor an actionable reason rather than a generic timeout.

#### 2. Align release, installer, and support claims

- Decide the support matrix publicly: either ship and test `aarch64-unknown-linux-musl`, or state `x86_64 Linux` as the only binary-release target and guide ARM users to a tested source build.
- Add clean-container install verification for every release asset, checksum verification, `rustdroid version`, `doctor`, completion generation, and `rustdroid-run help`.
- Publish to crates.io only after a live name-availability check and a tagged, verified release; otherwise describe the channel as "planned", never "ready".
- Add build provenance/attestation to release artifacts and keep checksums alongside every archive.
- Correct the public documentation URL, homepage metadata, social preview, and repository topics: `android`, `android-emulator`, `apk`, `adb`, `mobile-testing`, `devtools`, `rust`, `cli`, `github-actions`.

**Done when:** every install instruction resolves to a real tested artifact or a precise fallback, and the GitHub sidebar contains no stale links.

#### 3. Replace the feature list with a 45-second proof

- Rework the README opening around the outcome: "APK path in; launch receipt out." Put the host-fast path first, with Docker described as an optional reproducible backend.
- Add a short, captioned terminal recording/GIF showing `doctor`, a fixture run, a success receipt, and the generated HTML/JSON artifact. Keep it under a minute and readable without sound.
- Add one copyable “I have an APK” command, one “I build with Gradle” command, and one “I need CI artifacts” command. Each must use a command tested in CI.
- Commit a small, purpose-built demo/fixture journey and a golden result screenshot. Do not require Android Studio or a private APK to see the product working.
- Move unrelated `sample-pdfs/` material out of the repository after confirming it is not part of a documented release contract. Replace it only with APK-relevant fixtures or examples.
- Add honest badges only for workflows that are presently green, the latest release, license, and Rust version. Avoid vanity badges.

**Done when:** a new visitor can understand the problem, see the output, and run a documented fixture flow without choosing among ten modes.

### P1 -- Make first use boringly successful (`v0.4.0`)

#### 4. Deliver a supported-machine onboarding path

- Write a distro-specific quickstart for Ubuntu/Debian and Fedora: KVM permissions, Android command-line tools, emulator image/AVD creation, `adb`, `aapt`/`apkanalyzer`, and optional `scrcpy`.
- Provide a non-destructive `rustdroid setup`/bootstrap experience only if it can explain every proposed change before applying it. A generated shell plan is safer than hidden `sudo` automation.
- Extend `doctor --json` with stable check IDs, remediation commands, the selected backend, and an explicit distinction between mandatory and optional tools.
- Create a fresh-machine contract test that installs the documented dependencies, creates the required AVD, and runs a fixture. It may be scheduled or manually dispatched if it is expensive.
- Explain project config ownership: checked-in `.rustdroid.toml` examples for host-fast, headless CI, and low-RAM environments, plus precedence rules for flags, environment variables, and profiles.

**Done when:** an Android developer can reach one verified run from a clean supported Linux machine without guessing which tool or AVD is missing.

#### 5. Turn receipts into a reusable CI primitive

- Define a stable, versioned run-receipt schema for JSON and HTML: input digest, emulator/API/profile, timings, package/activity, pass/fail classification, links to logs, and tool version. Never expose APK contents or private paths by default.
- Add optional JUnit output and a concise Markdown job summary so CI systems can mark a smoke test as passed or failed without parsing human logs.
- Provide a tested `action.yml` or a small dedicated composite action that installs RustDroid, accepts an APK path/profile, uploads the receipt, and works on the documented Linux/KVM runner shape.
- Demonstrate the action against one open reference Android project or a public fixture project. Pin action versions and test the exact `uses:` snippet.

**Done when:** a repository can adopt RustDroid with one documented CI step and receive useful failure evidence in the same run.

### P2 -- Grow the useful ecosystem without diluting the core (`v0.5.0`)

#### 6. Publish reference workflows, not speculative integrations

- Maintain small, executable examples for a Gradle Android app, Flutter APK output, and React Native/Expo prebuilt APK output. Each should explain the APK path and run the same receipt contract.
- Add recipe pages for split APKs, `.apks`, `.xapk`, cold vs warm startup checks, crash/ANR triage, and running an existing Maestro/Appium command after RustDroid has prepared the emulator.
- Publish a transparent support matrix for backend, API image, ABI, UI mode, and CI runner. Generate it from tested combinations where possible.
- Add a `--dry-run`/plan view wherever an operation can create, remove, or reuse state, and make cleanup ownership obvious.

**Done when:** users can recognize their stack in an example and adopt the project without asking whether it replaces their entire test framework.

#### 7. Make speed claims reproducible and comparable

- Version the benchmark environment: host CPU class, Linux image, Android API image, AVD configuration, cold/warm state, APK fixture, and command line.
- Publish a small benchmark table in each release: boot, install, launch, total, and variance. Compare host-fast with Docker only when the hardware and conditions are equivalent.
- Add an opt-in benchmark command that writes a receipt; never collect machine data remotely.
- Investigate snapshot reuse, APK fingerprinting, and incremental reinstall only when measurements show a user-visible win and correctness stays intact.

**Done when:** "fast" means a contributor can rerun and challenge the measurement, not just read a claim.

### P3 -- Build a contributor and distribution loop (continuous)

#### 8. Make contribution safe and inviting

- Add `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md`, issue forms for bug reports, environment/setup failures, and feature proposals, plus a pull-request template.
- Label and document starter work: `good first issue`, `help wanted`, `docs`, `ci`, `host-runtime`, `fixtures`, and `release`. Every starter issue needs scope, acceptance criteria, and a reproducible command.
- Enable GitHub Discussions for setup questions and workflow recipes; reserve Issues for reproducible defects and accepted work. Publish a respectful response expectation instead of pretending to provide 24/7 support.
- Add dependency/security maintenance appropriate for a Rust CLI: CodeQL, Dependabot/Renovate, `cargo deny` or equivalent license/advisory checks, and a release-security checklist.

**Done when:** an outside developer knows where to ask, report, contribute, and disclose a security issue without reading the whole codebase first.

#### 9. Distribute proof where the right developers already learn

- Ship `v0.3.0` with a one-command demo, a screenshot, a clear changelog, and a short release post focused on the problem it removes.
- Write one technically useful comparison: the local host fast loop versus manually wiring an Android emulator or a broader Docker Android stack. Show commands, environment, limits, and measurements; do not attack alternatives.
- Share the evidence in relevant Rust, Android, mobile CI, and self-hosted tooling communities only where the community rules allow it. Lead with the reproducible demo or a useful troubleshooting lesson, not “please star my repo.”
- Ask early users for a concrete reference workflow, issue, or short quote. Add public “used by” entries only with permission and an actual link.
- Treat every release, recipe, benchmark, and solved setup issue as reusable content. A maintainable evidence library compounds better than one launch post.

**Done when:** discovery brings people to a verified use case, and each new user can produce feedback that makes the next user's path shorter.

## Release and maturity gates

| Release | Purpose | Required proof |
| --- | --- | --- |
| `v0.3.1` | Consolidated trust, activation, and reference-workflow release | Green verified release; aligned x86_64/source-only ARM claims; fixture proof; fresh-machine proof; receipt action; stack-specific templates; portable checksum and provenance |
| Next feature release, if warranted | Improvements driven by real feedback | A scoped user problem, testable acceptance criteria, and updated evidence; do not cut a version merely to match a roadmap label |
| `v1.0.0` | Public maturity | Four healthy host runs, supported install matrix, durable release/security process, external use evidence, no known P0 activation or data-loss defect |

Do not cut `v1.0.0` because the feature list feels long. Cut it only when a new user can install, run, understand a failure, and adopt RustDroid in CI with public evidence.

## Operating cadence from here

1. Let the scheduled host lane collect four healthy weekly runs; investigate any failure through its uploaded taxonomy before changing product claims.
2. Keep `v0.3.1` as the verified install target. Do not publish crates.io until a separately reviewed live publication decision and name-availability check.
3. Upload the prepared social preview only after explicit approval, then verify the GitHub sidebar shows it.
4. Share the reproducible release proof where community rules permit, and record only independently verifiable adoption, quotes, and workflow links.
5. Cut the next release only in response to a concrete, reproduced user problem.

This order still postpones new emulator modes and “viral” features. A small Android developer tool becomes recommendable when its first run is obvious, its failures are explainable, and its evidence is trustworthy.
