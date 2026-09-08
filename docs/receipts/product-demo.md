# Product demo provenance

The checked-in demo is a real Linux/KVM capture produced on GitHub Actions. It
uses the verified RustDroid `v0.3.2` candidate archive; no Android emulator or
KVM was installed on the maintainer's Mac.

## Evidence

| Item | Evidence |
| --- | --- |
| Android capture | [raw capture from run 34227525161](https://github.com/OthmaneBlial/rustdroid/actions/runs/34227525161) (capture and receipts retained; its first export was correctly rejected above 60 s) |
| Accepted FFmpeg export | [green edit-only run 34228671909](https://github.com/OthmaneBlial/rustdroid/actions/runs/34228671909) |
| Product binary source | `2754c3ee6fc3bb7795f0b2f0a8e100febb4be900` (candidate archive from [34205275762](https://github.com/OthmaneBlial/rustdroid/actions/runs/34205275762)) |
| Recording workflow checkout | `a26a9f63f8e31a7911a0c6ecc343765fa427ed43` |
| Android environment | API 30, `test_avd`, `emulator-5554`, Linux/KVM runner |
| Raw capture | [`rustdroid-product-demo-uncut.mp4`](../../assets/rustdroid-product-demo-uncut.mp4), SHA-256 `503ba3cc1c69ea1bb8d6b401031c3e400db6b7791a13227c72d5b115e4ae482` |
| Final video | [`rustdroid-product-demo.mp4`](../../assets/rustdroid-product-demo.mp4), SHA-256 `3869a1bac28911eab002d95dac2d32738f2aa56a80aaae939d464edd72ec89f1` |
| Poster | [`rustdroid-product-demo-poster.png`](../../assets/rustdroid-product-demo-poster.png), SHA-256 `b0531884dcf52405f3bba7818e4c7a915cc0a688a5ee11269b7d04493c1f4956` |
| Captions | [`rustdroid-product-demo.srt`](../../assets/rustdroid-product-demo.srt), SHA-256 `a755abe65aabcbd9c294039c44273e2d1c03e948be7fe60f5c505f821bb1833c` |
| Live project site | [Pages site](https://othmaneblial.github.io/rustdroid/); MP4, SRT and docs paths returned HTTP 200 after propagation |

The raw capture is 118.28 seconds. The public export is 56.52 seconds,
1280×720, 25 fps, H.264 `yuv420p`, no audio, and 715 KiB. The export uses
FFmpeg's web-safe H.264 profile and `+faststart`. It keeps the real successful
launch result, a readable crop of the real Android surface, the generated
`passed` HTML receipt, the classified missing-launcher failure, and its HTML
receipt. Setup and variable waits are excluded or time-compressed; the
captions say when that happens. The edit script is
[`scripts/edit-product-demo.sh`](../../scripts/edit-product-demo.sh).

The final MP4 is a checked-in copy of the green edit-only artifact. The project
site player is ready and the README keeps a fallback download; GitHub strips a
relative README `<video>` tag, so a maintainer upload is still required for a
native README player. The raw capture and all runtime receipts remain in the
linked Actions artifact for the same run (subject to GitHub's artifact
retention policy). The capture was recorded on GitHub's Linux/KVM runner; no
emulator or KVM was installed on macOS.
