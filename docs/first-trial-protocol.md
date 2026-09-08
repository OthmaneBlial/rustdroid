# First independent trial

Status: protocol prepared; no participants recruited and no trial outcomes claimed.

Use this after the exact candidate passes the Linux/KVM runtime and installation gates. The maintainer obtains consent and recruits five developers who have not used RustDroid. Do not collect application APKs, logs, identities or recordings without separate permission. Public fixtures are sufficient for this exercise.

## Before the session

- Record the tested release version, immutable action/source SHA and download checksum.
- Agree whether anonymous notes may be retained. Recording is optional, separately consented, and must avoid personal desktop content.
- Ask the participant to use a dedicated supported Linux host, not a production runner or personal Android device.
- Record architecture, KVM access, SDK/AVD readiness and prior Android experience. Treat fresh SDK setup as a separate timed activity.
- Provide the release README and public fixture only. Do not preconfigure RustDroid silently.

## Task and observation

Say: “Use this tool to check whether the supplied APK launches, then find the report you would send to a teammate.” Do not describe which command to choose.

Start the prepared-host timer when the participant begins reading the quickstart. Stop when they open a receipt and correctly identify its outcome. Note the first command, confusing choices, errors, time spent and any intervention. After ten minutes, offer help; record the trial as assisted, not unaided.

Next provide the public missing-launcher fixture. Ask the participant to explain the failed stage from the report. Do not describe a fabricated crash as an observed one. Finally ask what this report adds beyond their existing ADB or CI workflow and whether they would use it again.

## Evidence template

Use a participant-chosen anonymous ID. Store private notes outside the repository; publish only consented, redacted aggregates.

| Field | Recorded evidence |
| --- | --- |
| Consent and permitted retention | |
| Candidate/version/checksum | |
| Prepared host or fresh setup | |
| SDK setup time, separately | |
| Time to first correctly understood receipt | |
| Unaided / assisted / did not finish | |
| First blocking step and exact safe error | |
| Failed-stage explanation in participant's words | |
| Existing workflow and concrete added value | |
| Would repeat? Why? | |
| Follow-up and publication permission | |

## Decision rule

The M4 target is four of five unaided receipts. This is a target, not a current result. Rank observed blockers by number of affected participants, then severity; fix the top two and rerun the affected task before claiming onboarding is validated. Do not lower the threshold by excluding unsuccessful participants.

For M5, request links to three independently owned CI runs and explicit permission to reference them. Check that a failed run still uploads its report and fails its job. After at least seven days, verify two repeat integrations through actual run history or participant confirmation. A maintainer-owned demo repository is not independent adoption.

Choose a P2 feature only from the recorded friction: metadata before emulator setup, an explicitly consented screenshot, or comparison of real receipts. No feature is selected by this protocol alone.
