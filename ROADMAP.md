# RustDroid: worth trying, keeping, and sharing

> Make an Android build failure understandable in one command, with evidence a teammate can open.

This is an execution plan, not a forecast of GitHub stars. The repository currently explains more than it demonstrates. The next investment should make the value visible and the first result dependable.

## Current snapshot -- 2026-09-08

Audit baseline: `f03c933`, checked against the live repository and release on September 8, 2026. Counts are a dated snapshot.

| Area | What exists | What holds adoption back |
| --- | --- | --- |
| Audience | Linux/KVM developers and Android CI maintainers with an existing APK | Several workflows compete before one first user is clearly addressed |
| Attention | 2 stars, 0 forks; all 13 dependency PRs processed | No independently verified adopter workflow |
| Core | Host/Docker; APK, splits, APKS/XAPK; watch; diagnostics; five report formats | A large command surface makes the first decision harder |
| Evidence | Public Gradle, Flutter and Expo runs; checked-in Android 35 receipt | Historical success does not validate current source or all failure cases |
| Demo | Eight-second GIF generated from four SVG scenes | The generator supplies text and checkmarks; it does not record execution or Android |
| Installation | Published `v0.3.1` Linux x86_64 archive, checksum and install snippet | `main` is 46 commits ahead of the tag; new failure receipts are not in that binary |
| Quality | Latest local Rust suite, Clippy, packaging and dependency audits passed | Emulator scenarios are opt-in; local totals do not establish device validation |
| Automation | Workflow definitions exist | GitHub Actions are temporarily disabled; no current hosted runtime/release gate |
| Distribution | GitHub Releases and reusable action | Marketplace/crates.io publication is not established by this audit |

Preserve the September 1 [Gradle/Flutter/Expo proof](https://github.com/OthmaneBlial/rustdroid/actions/runs/33519017529), which uses the pinned release-era action, and earlier host evidence: [first](https://github.com/OthmaneBlial/rustdroid/actions/runs/32905475830), [second](https://github.com/OthmaneBlial/rustdroid/actions/runs/32907975602/attempts/2), [third](https://github.com/OthmaneBlial/rustdroid/actions/runs/32915494009). These are historical receipts, not current CI badges.

## The positioning decision

**First user:** the maintainer of a Linux Android CI pipeline who receives an APK and needs to check installation and launch before running a larger test suite.

**Promise to test:** “Your APK built. Find out whether it launches, and keep the reason when it does not.” Add “stays alive” publicly only after the observation-window checks below pass.

**Reason to keep it:** the same classified result and evidence directory locally and in CI. Avoiding ambiguous green builds and opaque timeouts matters more than a speed claim.

**Reason to share it:** an inspectable report showing the failed stage, APK digest, environment and a useful next step. Show a failure solved, not just a list of commands.

| Existing choice | What it already does | RustDroid must add |
| --- | --- | --- |
| [android-emulator-runner](https://github.com/ReactiveCircus/android-emulator-runner) | Provisions an emulator and executes a script | Artifact inspection, install/launch observation and a portable result after provisioning |
| [Gradle-managed devices](https://developer.android.com/studio/test/managed-devices) | Runs project-owned instrumented tests | A gate for already-built artifacts from different build stacks |
| [Maestro](https://github.com/mobile-dev-inc/Maestro) | Automates UI flows | Packaging/launch diagnosis before those flows |
| A short ADB shell script | Installs and starts an app cheaply | Better failure semantics, lifecycle ownership and useful evidence worth an extra dependency |

**Go/no-go question:** after three independent trials, can maintainers name something the receipt saved them from debugging manually? If not, improve that outcome before adding distribution channels.

## P0 -- Make every result trustworthy

### Implementation evidence

GitHub verification, September 8 (after owner-authorized Actions reactivation):

- Source `c6c3df9`: [action contract](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202434356) passed with a downloaded schema-v1 receipt identifying RustDroid 0.3.2, host backend and API 35; [fresh runner](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202437067) executed both opt-in Android runtime tests successfully.
- Source `d1ad34c`: [locked source-package installation](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202677019) passed on Linux. [CI](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202677506) passed fast checks and built the Linux musl archive, verified its checksum and installed it in a clean container, reporting 0.3.2. The archive used the CI label, not a published release tag.
- [Dependency security](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202677442) and [CodeQL](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202677495) passed for `d1ad34c`.
- Source `c6c3df9`: the downloaded `runtime-failure-matrix` artifact from [host runtime](https://github.com/OthmaneBlial/rustdroid/actions/runs/34202431887) verifies all four scenarios on API 30: successful launch, process exit, Java crash and missing launcher. The latter three return nonzero and retain the expected stage in JSON/HTML/JUnit/Markdown. The complete workflow, including performance, subsequently passed.
- Follow-up timeout hardening: host command execution now requests process termination when its future is cancelled. A real subprocess regression test verifies that a command starts, times out and cannot perform its delayed write. This tests cancellation, not the full Android timeout-receipt contract.
- Cleanup precedence is now exercised through the production selection helper and actual written JSON/HTML/JUnit/Markdown reports: crash, ANR, reader and artifact failures retain their original stage and message with and without a secondary cleanup failure. A standalone cleanup failure remains a failure. This is deterministic policy/serialization coverage, not an injected emulator cleanup failure.
- Source `1be2d25`: [five-case failure matrix](https://github.com/OthmaneBlial/rustdroid/actions/runs/34203908475) passed on API 30. The freshly compiled ANR fixture blocks a registered receiver during a foreground broadcast. Downloaded logs show Android's broadcast timeout after 10001 ms, followed by `ANR in com.rustdroid.fixture.anr`; the receipt is `failed/app_runtime/anr`, with nonzero CLI exit and matching report stages. No simulated ANR logs are used. Success, process exit, Java crash and missing launcher also passed their assertions in this run.
- These successes do not establish every M1 failure case (including injected ANR, reader, timeout and cleanup failures), a published release, external adoption or a final video. Earlier local-only notes below describe the implementation sequence, not the latest hosted validation status.

Implementation is in progress; the audit snapshot above remains historical. The owner has authorized source commits directly to main. This does not authorize or establish a tagged release, hosted runtime validation or a final Android demonstration.

- Reader tasks are now supervised; startup errors, unexpected EOF and panics cannot silently pass. Readiness precedes the observation timer.
- PID discovery precedes the timer, with bounded lookup for application runs. Application runs monitor PID continuity and check it again at the deadline; interactive log commands retain interrupt-to-stop behavior.
- Local regression tests cover startup failure, EOF, panic, delayed readiness, runtime error, ANR notification, absent readers and the distinction between cancelling a run and stopping interactive logs. These tests exercise the observer, not Android itself. The full local suite and Clippy passed after the observer changes; real emulator tests remain opt-in and unverified here.
- Each application launch now writes a unique logcat marker. Streaming and final-log parsing ignore earlier entries and match the target package; a missing marker fails capture. Whole-device diagnostic dumps no longer supply unscoped crash/ANR summaries. Fifteen observer/parser tests cover these behaviors, including Docker chunk boundaries and similarly named packages; Clippy passes.
- Written-receipt tests verify JSON, HTML, JUnit and Markdown for nine stage/classification pairs. They validate serialization, not runtime injection of all failures.
- The process-exit and Java-crash APK fixtures are compiled and signed; signature verification and AAPT metadata/inventory tests pass using the installed Android SDK. `scripts/check-runtime-failures.py` and the host workflow now exercise success, exit, crash and missing launcher on an explicitly selected Linux/KVM emulator. YAML parsing and documentation/release contract tests pass. Actual execution was refused on macOS before device access, as intended; the runtime matrix is not yet verified.
- M1's specified contracts now have the local regressions and hosted matrix evidence listed in the checked items below. Earlier unverified-runtime notes document the initial macOS-only stage. The earlier CLI-only video is not the final demo; final video work follows the other feasible phases.

### M1. Close the gap between “command finished” and “app passed”

Release blocker. Estimate: 3–5 focused engineering days plus Linux/KVM verification. Estimates describe effort, not a promised delivery date.

The code review found specific risks to reproduce, not confirmed field incidents:

- [x] Propagate reader errors/early termination in `src/logs/mod.rs`. Supervised readers report startup errors, EOF and panics instead of passing on timeout; local observer regression tests verify those paths. The phase-level Linux gate remains open.
- [x] Define when observation starts. PID discovery precedes reader readiness, and readiness precedes the observation deadline. The elapsed-time regression proves the requested interval excludes startup delay; API 30/35 real runs exercise this path. Readiness timeout and closed-channel regressions reject incomplete observation.
- [x] Add a “launches, then exits” fixture and an end-of-window liveness policy; distinguish intended activity changes from process death. Main-process PID continuity, rather than foreground activity identity, is required. The real API 30 exit fixture returns `failed/app_runtime/crash` in run 34202431887; a normally running fixture passes.
- [x] Verify crash/ANR attribution to the current package and run. Unique launch markers scope streaming and the final dump; parser regressions reject historical and unrelated-package failures. Real crash and broadcast-ANR receipts passed on API 30. Whole-device diagnostic dumps do not supply canonical failure attribution.
- [x] Add deterministic launch, reader, timeout, crash, ANR, missing-input and cleanup-failure contracts across JSON/HTML/JUnit/Markdown. [Nine-case API 30 matrix](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205061851), source `d9022f9`, passed with downloaded receipts: success, exit, crash, missing launcher, ANR, reader exit, marker timeout, cleanup failure and reader-plus-cleanup failure. Every failure returns nonzero and has matching report stages. The combined case preserves `log_capture/capture` and logs the secondary cleanup error. Missing-input CLI and four-format serialization regressions cover the pre-runtime input contract. Fault wrappers are explicitly test-only; the application/Android runtime is real.
- [x] Define a stable contract for parse/configuration/backend-connection errors before receipt ownership: documented stderr/exit codes in `docs/receipt-schema-v1.md`, with CLI tests for parse/configuration failures and absence of synthetic receipts.

**Acceptance:** every injected failure returns nonzero; no tested observer failure returns `passed`; the same stage appears in all reports. Verify current source on a real Linux/KVM emulator. A local fixture suite alone is insufficient.

### M2. Ship a release that matches the README

Local candidate evidence: Cargo/lockfile now identify 0.3.2. The source package was reduced from 902 to 99 files by anchoring inclusion paths; nested dependency licenses and demo environments are excluded. Cargo package verification and optimized installation from that package into a fresh local prefix pass on macOS. The installed binary reports 0.3.2 and writes a failed input-preflight receipt with exit code 1. A checksummed source candidate is in `dist/release-candidate-0.3.2/`. This is a local source-install proof, not a Linux binary release, a clean OS test, registry publication or emulator validation.

Before launch promotion. Estimate: 1–2 days after M1.

- [ ] Prepare `v0.3.2` as the reliability release; choose a minor version if public commands are added. See draft `docs/releases/v0.3.2.md`.
- [ ] Include failure receipts and September 8 fixes: SHA-256 encoding, dependencies and Expo-compatible React Native.
- [ ] Align Cargo version/lockfile, tag, changelog, CLI version, notes, immutable action examples and tested source revision.
- [x] Build the Linux x86_64 archive on Linux; verify its checksum and install in a clean container, then attach provenance. [Candidate build 34205275762](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205275762), source `2754c3e`, passed. The downloaded archive's SHA-256 is `b8771783960ad4e14f36dc97daca3cd48b6b73d234d72e03e937d86f01ef98b2`; `VERSION` is `v0.3.2`. `gh attestation verify` verified the GitHub-hosted release workflow and exact source SHA. This is a workflow artifact, not a published GitHub Release.
- [x] Run the host lane against the exact candidate commit, not only the old pinned action. [Run 34205810640](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205810640) checked out `2754c3ee6fc3bb7795f0b2f0a8e100febb4be900` and passed runtime/smoke, the nine-case failure matrix and performance. Downloaded matrix receipts verify every expected outcome.
- [ ] Download the published archive again and execute copied README commands on a clean supported host.
- [ ] Attach the actual MP4, poster and minimal public receipts with recording commit and environment.

**Acceptance:** the “latest” binary performs the behavior shown in the video and README. A draft, macOS build, source archive or tag alone is not a finished Linux release.

### M3. Replace the illustrative GIF with watchable proof

Before social launch. Estimate: 1–2 days once M1/M2 pass.

Use FFmpeg, preserving raw output and the original recording. Keep the existing SVG-generated GIF explicitly labelled as an illustration.

| Time | Picture | What the viewer learns |
| --- | --- | --- |
| 0–5 s | Actual terminal, public APK and provenance | The input is reproducible |
| 5–18 s | One command beside the real emulator screen | The app installs and visibly launches |
| 18–30 s | Broken fixture and classified failure | Why a successful build is insufficient |
| 30–42 s | Generated HTML report, stage, digest and files | What a teammate receives |
| 42–50 s | Copyable quickstart and release URL | How to try it |

- [ ] Record Linux/KVM execution from the candidate; keep terminal output, Android screen recording, source SHA and receipts together.
- [ ] Retain uncut source. Label edits, warmed emulator state and speed changes; measure SDK installation separately.
- [ ] Export captioned 1280×720 or 1920×1080 H.264 MP4, `yuv420p`, `+faststart`, approximately 45–60 seconds; target under 15 MB if text remains legible.
- [ ] Make it understandable without audio. Keep one readable command on screen instead of rapidly scrolling logs.
- [ ] Provide play/pause/seek controls on the site; upload and verify a native GitHub README video. A GIF or raw MP4 link does not prove an inline player works.
- [ ] Inspect beginning, launch, failure, report and final frames; decode the entire file and verify playback.

**Acceptance:** a stranger explains the value after one viewing and can inspect the same run's evidence. A local CLI-only recording is an interim asset, not completion of the emulator-demo milestone. See `docs/demo.md` for the current asset's scope.

## P1 -- Put RustDroid where Android CI is assembled

### M4. Make the first trial cost one decision

Estimate: 2–3 days after release and demonstration agree.

- [ ] Put one Linux quickstart below the video: supported host/prerequisites, one public fixture command, one report path.
- [x] Provide a versioned fixture download and digest so trying the binary does not require cloning the repository. README uses an immutable source SHA; the downloaded APK digest was verified locally. The candidate release workflow also packages this fixture and checksum; those release assets are not yet published.
- [ ] Diagnose KVM/SDK/AVD problems before runtime mutation. Keep setup reviewable and non-destructive.
- [ ] Move backend choices, profiles and exhaustive commands below the first success path.
- [ ] Observe five unfamiliar Linux developers trying it, with consent; fix the top two blockers before adding documentation.

**Acceptance:** four of five reach a receipt unaided. Target under five minutes on a prepared host; record fresh SDK/AVD setup separately. These are experiment targets, not current claims.

The consent, timing, anonymous evidence and decision protocol is prepared in [first-trial-protocol.md](docs/first-trial-protocol.md). No recruitment, trial or adoption result is implied.

### M5. Turn one CI integration into repeat usage

Local action regression: the receipt step now exports its directory and available Markdown report on both success and failure, preserving the original command exit code. A shell execution test covers exit codes 0, 1 and 2 with paths containing spaces. This does not prove hosted GitHub runner behavior; the external workflow gate below remains open.

Estimate: 2–4 days plus external response time.

- [x] Offer one complete consumer workflow: immutable tested action SHA, emulator provisioning, APK build, receipt and `if: always()` artifact upload. See `examples/android-receipt-workflow.yml`; the action pin has a verified API 35 contract. Adopter-specific Gradle/Java/APK settings and external execution remain separate gates.
- [x] Show a failing run: its report survives a nonzero exit and the overall job still fails. [Intentional failed job 34206059316](https://github.com/OthmaneBlial/rustdroid/actions/runs/34206059316) fails only the RustDroid receipt step, successfully uploads `action-contract-receipt`, and remains red. The downloaded API 35 receipt identifies `failed/app_launch/launch` for the missing-launcher fixture.
- [ ] Validate the workflow in a repository without RustDroid source.
- [ ] Publish to GitHub Marketplace after listing/account requirements are satisfied; explicitly scope prepared Linux/KVM runners.
- [ ] Validate packaging and clean installation before crates.io publication; check name/account access immediately before publishing.
- [ ] Keep GitHub Releases the binary source of truth. Add other channels only when trial users ask.

**Acceptance:** three external repositories produce useful receipts, and two run the integration again the following week. Track [issue #20](https://github.com/OthmaneBlial/rustdroid/issues/20). External adoption is not a build artifact.

## P2 -- Add one memorable capability

Choose from observed friction after P0. Do not build all three by default.

| Candidate | Why it could be shared | Evidence before implementation |
| --- | --- | --- |
| Offline `inspect app.apk` | Immediate package/launcher/ABI/split/digest feedback lowers trial cost | Three trial users need metadata before emulator setup; define inspector host support separately |
| Optional receipt screenshot | A teammate understands the result without reading logs | Public fixture, timestamp, package attribution and explicit privacy controls; no claim of UI correctness |
| Compare two local receipts | Explains environment/artifact drift between good and bad runs | Two maintainers compare runs manually; normalize volatile fields and avoid single-sample speed conclusions |

A screenshot-rich report is the strongest visual hypothesis. An offline inspector is the strongest low-friction hypothesis. Let trials determine which ships first.

## P3 -- Launch a useful demonstration

Do not promise a star count. No purchased stars, automated outreach or invented testimonials.

1. Prepare a release post around a concrete failure: “The APK built, but it had no launchable activity. Here is the command and receipt that caught it.” Link demo, exact release and public fixture.
2. Share where Android/Flutter/React Native CI maintainers discuss those failures. Check current community rules and disclose authorship. Prepare drafts; publish only to channels the maintainer authorizes.
3. Answer setup failures with reproductions and patches. Turn repeated questions into improvements to the quickstart.
4. Publish consented adopter workflows and limitations. Ask whether the receipt helped.
5. Consider Show HN or a Rust-focused write-up after an independent success; adapt the technical story to the audience.

**Launch threshold:** verified current installation, full playable real demo, an external successful trial and M1 failure contracts. Defer broad promotion when these are absent.

## Sequence and scorecard

Assume one maintainer. External adoption and four scheduled weeks cannot be compressed into an engineering sprint.

| Window | Deliverable | Gate |
| --- | --- | --- |
| Week 1 | M1 failure/liveness contracts; candidate | No false pass in the injected matrix |
| Week 2 | M2 release, M3 video, M4 trial | Source/release/video agree; 4/5 trials succeed |
| Weeks 3–4 | M5 integration and focused launch | Three external receipts; repeat use |
| After evidence | One P2 capability | Observed user friction supports it |

Review weekly through public links or consented feedback. No CLI telemetry is required.

| Metric | Baseline | Initial target | Evidence |
| --- | --- | --- | --- |
| Unaided first receipts | Not measured | 4/5 trials | Consented onboarding notes |
| Prepared-host time to receipt | Not measured for new users | Median below 5 minutes | Commands and prerequisites recorded separately |
| External workflows | None verified | 3 | Linked adopter runs |
| Repeat use | Not measured | 2 integrations run again next week | Workflow history or confirmation |
| Demo clarity | Not measured | 4/5 viewers explain value and next step | Short feedback session |
| Downloads and visitor-to-star trend | Not collected in this audit | Establish baseline and compare launches | Release counters and owner traffic insights |
| Stars | 2 | Observe, no forecast | Weekly dated snapshot |

Attention without successful trials means activation needs work. Trials without returns mean recurring value needs work. Repeat usage with low reach justifies more distribution.

## Automation and release gates

Actions was re-enabled with explicit owner authorization on September 8. Android/KVM execution is restricted to GitHub runners; no KVM or emulator is to be installed on the owner's Mac. Keep lightweight checks on changes and run emulator/release lanes deliberately with timeouts and failure artifacts.

[Issue #19](https://github.com/OthmaneBlial/rustdroid/issues/19) has one historical [scheduled success on August 31](https://github.com/OthmaneBlial/rustdroid/actions/runs/33393692395), not four consecutive weekly proofs. Manual validation runs do not replace that scheduled evidence.

Local gate:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo deny check
./scripts/ci-shell-check.sh
cargo package --locked
cd examples/apps/expo-prebuild
npm ci --ignore-scripts --no-audit --no-fund
npm audit --omit=dev --audit-level=moderate
npx --no-install expo install --check
```

Separate gates: Linux/KVM failure matrix, exact-candidate host run, Linux archive/clean-container install, attestation, published-asset download verification, full video playback, registry/account publication, and external users. Keep each pending until evidence exists.

## Defer

- Desktop dashboard, hosted service, AI layer, UI-test language and device farm.
- Broad macOS/Windows runtime claims before dedicated implementation and real-host validation.
- More badges, exhaustive packaging channels or another documentation microsite as substitutes for use.
- Speed rankings without equal environments, multiple cold/warm samples and published inputs.

The next compelling RustDroid is a dependable small gate with a memorable demonstration. Expand after people can install it, trust it and explain why they kept it.
