# Product demo provenance

The checked-in demo is a real Linux/KVM capture produced on GitHub Actions. It
uses the verified RustDroid `v0.3.2` candidate archive; no Android emulator or
KVM was installed on the maintainer's Mac.

## Evidence

| Item | Evidence |
| --- | --- |
| Android capture | [full capture run 34209082936](https://github.com/OthmaneBlial/rustdroid/actions/runs/34209082936) |
| Accepted FFmpeg export | [green edit-only run 34210495967](https://github.com/OthmaneBlial/rustdroid/actions/runs/34210495967) |
| Product binary source | `2754c3ee6fc3bb7795f0b2f0a8e100febb4be900` (candidate archive from [34205275762](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205275762)) |
| Recording workflow checkout | `d6e49b141c1c8b73a61cf7d4cda7d3ede5cb5376` |
| Android environment | API 30, `test_avd`, `emulator-5554`, Linux/KVM runner |
| Raw capture | [`rustdroid-product-demo-uncut.mp4`](../../assets/rustdroid-product-demo-uncut.mp4), SHA-256 `c36ff99761d4f2f5f736b00f145639633395a1445c6ef0a1a22cd6223ed6c36d` |
| Final video | [`rustdroid-product-demo.mp4`](../../assets/rustdroid-product-demo.mp4), SHA-256 `65824832e8e1f032a95d6cbd0d5731973d3f28ec17ef0bcf3ba6101f7281edef` |
| Poster | [`rustdroid-product-demo-poster.png`](../../assets/rustdroid-product-demo-poster.png), SHA-256 `68c3a614891d13d9297ecce40127c48ccbc4fe003f303ba91b8fa506bf8d0842` |
| Captions | [`rustdroid-product-demo.srt`](../../assets/rustdroid-product-demo.srt), SHA-256 `c39271c6be0c8c5a211ed598e1c02aa63cfe54c38b48d0ccec17235616c1a5f3` |
| Live project site | [Pages site](https://othmaneblial.github.io/rustdroid/), deployed in `OthmaneBlial/OthmaneBlial.github.io` commit `5e21b76197e8de9c93d5b52afbef7304c24a0682`; MP4, SRT and docs paths returned HTTP 200 after propagation |

The raw capture is 115.36 seconds. The public export is 57.36 seconds,
1280×720, 25 fps, H.264 `yuv420p`, no audio, and 552 KiB. The export uses
FFmpeg's web-safe H.264 profile and `+faststart`. It keeps the real launch,
classified missing-launcher failure, and generated HTML report. Setup and
variable waits are excluded or time-compressed; the captions say when that
happens. The edit script is [`scripts/edit-product-demo.sh`](../../scripts/edit-product-demo.sh).

The final MP4 is a checked-in copy of the green edit-only artifact. The project
site player is ready and the README keeps a fallback download; GitHub strips a
relative README `<video>` tag, so a maintainer upload is still required for a
native README player. The raw capture and all runtime receipts remain in the
linked Actions artifact for the same run (subject to GitHub's artifact
retention policy).
