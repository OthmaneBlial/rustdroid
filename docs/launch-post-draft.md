# Launch post draft — RustDroid 0.3.2

Status: **draft only — not published**.

The APK built. That still did not tell us whether it had a launchable activity.

RustDroid is a Linux-first CLI that checks an already-built Android artifact,
observes its launch on a host emulator, and leaves a receipt a teammate can
open. The [real demo](demo.md#full-emulator-demo) shows both sides on the
verified 0.3.2 candidate: a signed fixture that launches, then a
`missing-launcher.apk` run classified as `app_launch` with exit code 1 and an
HTML report containing the package, stage, digest and artifact links.

Try the public fixture on a prepared Linux/KVM host:

```bash
curl -fL https://raw.githubusercontent.com/OthmaneBlial/rustdroid/f03c933084246a112360139bdd1e6ae48de0708d/tests/fixtures/apks/launch-success.apk -o launch-success.apk
echo '5006fcae4718a1998dbba7097283792807284c29a7eff79f1d3a7a072492cf60  launch-success.apk' | sha256sum -c -
rustdroid --profile host-fast --host-avd-name test_avd run launch-success.apk --duration-secs 2 --keep-alive false --artifacts-dir artifacts/rustdroid-demo
```

Links to add only after the maintainer authorizes publication:

- exact tagged `v0.3.2` release and its checksum/provenance;
- the [demo](https://othmaneblial.github.io/rustdroid/) and [source](https://github.com/OthmaneBlial/rustdroid);
- one consented external adopter receipt, with limitations stated clearly.

Do not describe the candidate as a published release, promise a speed number,
or imply that a launch smoke check validates business flows. The supported
runtime is Linux/KVM; macOS is not an emulator target in this release.
