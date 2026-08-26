# Operation plans and dry runs

Runtime commands can start, reuse, stop, install into, or clear an emulator. Put `--dry-run` before or after the command to review that stateful plan without contacting Docker, ADB, an emulator, or an APK file.

```bash
rustdroid --dry-run --runtime-backend host --host-avd-name test_avd \
  run app/build/outputs/apk/debug/app-debug.apk \
  --keep-alive false --artifacts-dir artifacts/rustdroid

rustdroid --json --dry-run --profile host-fast stop --all
```

The JSON plan has schema version `1`, selected backend/profile/serial/AVD, safe input file names, and expected state effects. It intentionally excludes full local paths.

`--dry-run` covers `bench`, `fast-local`, `start`, `open`, `install`, `launch`, `uninstall`, `clear-data`, `run`, `watch`, `logs`, and `stop`. It never creates a container, starts an emulator, uploads an APK, clears data, or changes files. `clean --dry-run` continues to preview managed cleanup through its existing safe path.
