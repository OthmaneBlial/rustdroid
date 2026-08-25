# Security policy

## Supported versions

Security fixes are made on the current `main` branch and the latest released RustDroid version. Older releases may receive guidance, but they are not guaranteed patches.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability, leaked credential, APK privacy exposure, or unsafe cleanup path.

Use GitHub's **Report a vulnerability** feature in the repository Security tab when it is available. Include a minimal reproduction, affected version/commit, impact, and any safe mitigation. If private reporting is unavailable, contact the repository owner through GitHub with the subject `RustDroid security report`; do not attach private APKs or tokens.

We aim to acknowledge valid reports within seven calendar days and to coordinate disclosure after a fix or mitigation is available. This is a best-effort maintainer project, not a 24/7 response service.

## Scope

In scope: RustDroid source, installers, release artifacts, GitHub Actions, package distribution, and handling of local APK/log artifacts. Out of scope: Android emulator, Docker, GitHub, Android Studio, or a third-party APK unless RustDroid itself causes the issue.
