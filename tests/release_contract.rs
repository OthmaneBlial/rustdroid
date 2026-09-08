use std::process::Command;
use std::{env, fs, path::Path};

#[test]
fn release_version_guard_rejects_mismatched_tags() {
    let workflow = fs::read_to_string(".github/workflows/release.yml").unwrap();
    let body = workflow
        .split("name: Resolve and validate candidate version")
        .nth(1)
        .unwrap()
        .split("run: |\n")
        .nth(1)
        .unwrap()
        .split("\n      -")
        .next()
        .unwrap();
    let script = body
        .lines()
        .map(|line| line.strip_prefix("          ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    for (kind, name, success) in [
        ("branch", "main", true),
        ("tag", version.as_str(), true),
        ("tag", "v0.0.0", false),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let result = Command::new("bash")
            .args(["-c", &script])
            .env("GITHUB_REF_TYPE", kind)
            .env("GITHUB_REF_NAME", name)
            .env("GITHUB_OUTPUT", &output)
            .output()
            .unwrap();
        assert_eq!(result.status.success(), success);
        if success {
            assert_eq!(
                fs::read_to_string(output).unwrap(),
                format!("version={version}\n")
            );
        } else {
            assert!(!output.exists());
        }
    }
}

#[test]
fn release_assets_exist_in_repo() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/codeql.yml",
        ".github/workflows/dependency-security.yml",
        ".github/workflows/action-contract.yml",
        ".github/workflows/host-integration-runtime.yml",
        ".github/workflows/crates-io-readiness.yml",
        ".github/workflows/release.yml",
        ".github/workflows/published-release.yml",
        ".github/workflows/source-less-consumer.yml",
        "docs/performance-baselines.json",
        "docs/package-distribution.md",
        "docs/performance-notes/v0.1.0.md",
        "docs/release-announcement-checklist.md",
        "docs/release-rollback.md",
        "docs/release-security-checklist.md",
        "docs/releases/v0.1.0.md",
        "docs/support-matrix.md",
        "docs/version-bump-checklist.md",
        "install.sh",
        "run.sh",
        "uninstall.sh",
        "scripts/ci-host-check.sh",
        "scripts/ci-package-check.sh",
        "scripts/ci-shell-check.sh",
        "scripts/check-cargo-distribution.sh",
        "scripts/check-performance-baseline.sh",
        "scripts/generate-release-notes.sh",
        "scripts/generate-demo-gif.sh",
        "scripts/generate-support-matrix.sh",
        "scripts/package-release.sh",
        "scripts/verify-release-install.sh",
        "scripts/verify-release-install-container.sh",
        "README.md",
        "deny.toml",
        "LICENSE",
    ] {
        assert!(
            Path::new(path).exists(),
            "expected release asset '{}' to exist",
            path
        );
    }
}

#[test]
fn action_preserves_reports_and_exit_codes() {
    let action = fs::read_to_string("action.yml").unwrap();
    let body = action.rsplit_once("      run: |\n").unwrap().1;
    let script = body
        .lines()
        .map(|line| line.strip_prefix("        ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    for code in [0, 1, 2] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("target/release")).unwrap();
        let binary = root.join("target/release/rustdroid");
        fs::write(&binary, format!("#!/bin/bash\nprintf 'test receipt\\n' > \"$RUSTDROID_ACTION_ARTIFACTS/run-summary.md\"\nexit {code}\n")).unwrap();
        assert!(Command::new("chmod")
            .arg("+x")
            .arg(&binary)
            .status()
            .unwrap()
            .success());
        let artifacts = root.join("receipt with spaces");
        let mut command = Command::new("bash");
        command
            .args(["-c", &script])
            .env("GITHUB_ACTION_PATH", root)
            .env("GITHUB_OUTPUT", root.join("output"))
            .env("GITHUB_STEP_SUMMARY", root.join("summary"))
            .env("RUSTDROID_ACTION_ARTIFACTS", &artifacts);
        for key in ["APK", "PROFILE", "BACKEND", "AVD", "DURATION", "KEEP_ALIVE"] {
            command.env(format!("RUSTDROID_ACTION_{key}"), "test");
        }
        assert_eq!(command.status().unwrap().code(), Some(code));
        assert_eq!(
            fs::read_to_string(root.join("summary")).unwrap(),
            "test receipt\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("output")).unwrap(),
            format!("receipt-dir={}\n", artifacts.display())
        );
    }
}

#[test]
fn composite_action_declares_the_receipt_contract() {
    let action = std::fs::read_to_string("action.yml").expect("read action.yml");

    for snippet in [
        "name: RustDroid APK receipt",
        "apk-path:",
        "artifacts-dir:",
        "receipt-dir:",
        "junit.xml",
        "run-summary.md",
        "cargo build --locked --release",
    ] {
        assert!(
            action.contains(snippet),
            "expected action.yml to contain {snippet}"
        );
    }
}

#[test]
fn security_automation_declares_the_expected_controls() {
    let codeql =
        std::fs::read_to_string(".github/workflows/codeql.yml").expect("read CodeQL workflow");
    let dependency_security = std::fs::read_to_string(".github/workflows/dependency-security.yml")
        .expect("read dependency security workflow");
    let dependabot =
        std::fs::read_to_string(".github/dependabot.yml").expect("read Dependabot config");

    assert!(codeql.contains("github/codeql-action/init@v4"));
    assert!(codeql.contains("languages: rust"));
    assert!(dependency_security.contains("cargo deny check"));
    assert!(dependency_security.contains("npm audit --omit=dev --audit-level=moderate"));
    assert!(dependabot.contains("package-ecosystem: cargo"));
    assert!(dependabot.contains("package-ecosystem: github-actions"));
    assert!(dependabot.contains("package-ecosystem: npm"));
    assert!(dependabot.contains("directory: \"/examples/apps/expo-prebuild\""));
}

#[test]
fn official_actions_use_node24_ready_majors() {
    let sources = [
        ".github/workflows/ci.yml",
        ".github/workflows/action-contract.yml",
        ".github/workflows/crates-io-readiness.yml",
        ".github/workflows/dependency-security.yml",
        ".github/workflows/fresh-machine-contract.yml",
        ".github/workflows/host-integration-runtime.yml",
        ".github/workflows/reference-stack-fixtures.yml",
        ".github/workflows/release.yml",
        ".github/workflows/published-release.yml",
        ".github/workflows/source-less-consumer.yml",
        "examples/workflows/gradle-android-receipt.yml",
        "examples/workflows/flutter-receipt.yml",
        "examples/workflows/react-native-expo-receipt.yml",
        "docs/github-action.md",
    ]
    .map(|path| {
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {path}: {error}"))
    });
    let joined = sources.join("\n");

    for deprecated in [
        "actions/checkout@v4",
        "actions/setup-java@v4",
        "actions/setup-node@v4",
        "actions/upload-artifact@v4",
    ] {
        assert!(
            !joined.contains(deprecated),
            "workflow and documentation sources must not retain deprecated {deprecated}"
        );
    }

    for current in [
        "actions/checkout@v6",
        "actions/setup-java@v5",
        "actions/setup-node@v6",
        "actions/upload-artifact@v7",
    ] {
        assert!(
            joined.contains(current),
            "workflow and documentation sources must contain {current}"
        );
    }
}

#[test]
fn action_contract_exercises_checked_in_source_and_requires_api_level() {
    let workflow = std::fs::read_to_string(".github/workflows/action-contract.yml")
        .expect("read action contract workflow");

    assert!(workflow.contains("uses: ./"));
    assert!(workflow.contains(".emulator.api_level | strings | select(length > 0)"));
    assert!(workflow.contains("tests/fixtures/apks/launch-success.apk"));
    assert!(workflow.contains("push:"));
    assert!(workflow.contains("src/**"));
}

#[test]
fn source_less_consumer_exercises_the_published_action_without_checkout() {
    let workflow = std::fs::read_to_string(".github/workflows/source-less-consumer.yml")
        .expect("read source-less consumer workflow");
    let pinned_action = "OthmaneBlial/rustdroid@ce727e89711958fc09daa57ac17d90bf8743e8c3";

    assert!(workflow.contains("source-less consumer action"));
    assert!(workflow.contains(pinned_action));
    assert!(!workflow.contains("\n      - uses: actions/checkout@"));
    assert!(workflow.contains("rustdroid-fixture-${RELEASE_VERSION}.apk"));
    assert!(workflow.contains("api-level: 35"));
    assert!(workflow.contains(".emulator.api_level == \"35\""));
}

#[test]
fn stack_reference_workflows_boot_and_upload_the_pinned_receipt_action() {
    let pinned_action = "OthmaneBlial/rustdroid@ce727e89711958fc09daa57ac17d90bf8743e8c3";

    for workflow in [
        "examples/workflows/gradle-android-receipt.yml",
        "examples/workflows/flutter-receipt.yml",
        "examples/workflows/react-native-expo-receipt.yml",
    ] {
        let source = std::fs::read_to_string(workflow)
            .unwrap_or_else(|error| panic!("read {workflow}: {error}"));
        for snippet in [
            pinned_action,
            "script: \"true\"",
            "Start the provisioned Android 35 AVD",
            "-avd test_avd",
            "artifacts-dir: artifacts/rustdroid",
            "path: ${{ steps.receipt.outputs.receipt-dir }}",
            "Stop the provisioned Android 35 AVD",
        ] {
            assert!(
                source.contains(snippet),
                "{workflow} must contain {snippet}"
            );
        }
    }
}

#[test]
fn android_emulator_runner_keeps_host_environment_in_one_shell() {
    let host_runtime = std::fs::read_to_string(".github/workflows/host-integration-runtime.yml")
        .expect("read host runtime workflow");
    let fresh_machine = std::fs::read_to_string(".github/workflows/fresh-machine-contract.yml")
        .expect("read fresh-machine workflow");

    assert!(host_runtime.contains("script: >-"));
    assert!(host_runtime.contains("cargo build --locked;"));
    assert!(host_runtime
        .contains("RUSTDROID_HOST_TEST_SERIAL=emulator-5554 RUSTDROID_HOST_TEST_AVD=test_avd"));
    assert!(host_runtime
        .contains("RUSTDROID_SMOKE_BOOT_TIMEOUT_SECS=360\n            ./scripts/ci-host-check.sh"));

    assert!(fresh_machine.contains("script: >-"));
    assert!(fresh_machine.contains(
        "RUSTDROID_RUN_HOST_RUNTIME_TESTS=1 RUSTDROID_HOST_TEST_SERIAL=emulator-5554\n            cargo test"
    ));
}

#[test]
fn package_checksum_contract_is_portable_for_downloaded_assets() {
    let package_script =
        std::fs::read_to_string("scripts/package-release.sh").expect("read package release script");
    let package_check =
        std::fs::read_to_string("scripts/ci-package-check.sh").expect("read package check script");

    assert!(package_script.contains("cd \"$DIST_DIR\""));
    assert!(package_script.contains("sha256sum \"$(basename \"$ARCHIVE_PATH\")\""));
    assert!(package_check.contains("CHECKSUM_VERIFY_DIR=\"$(mktemp -d)\""));
    assert!(package_check.contains("sha256sum --check \"$(basename \"$CHECKSUM_PATH\")\""));
}

#[test]
fn install_and_package_scripts_are_executable() {
    for path in [
        "install.sh",
        "run.sh",
        "uninstall.sh",
        "scripts/ci-host-check.sh",
        "scripts/ci-package-check.sh",
        "scripts/ci-shell-check.sh",
        "scripts/check-cargo-distribution.sh",
        "scripts/check-performance-baseline.sh",
        "scripts/generate-release-notes.sh",
        "scripts/generate-support-matrix.sh",
        "scripts/package-release.sh",
        "scripts/verify-release-install.sh",
        "scripts/verify-release-install-container.sh",
    ] {
        let metadata = std::fs::metadata(path).expect("script metadata should be readable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(
                metadata.permissions().mode() & 0o111 != 0,
                "expected '{}' to be executable",
                path
            );
        }
    }
}

#[test]
fn install_and_uninstall_help_commands_work() {
    for (script, arg) in [
        ("install.sh", "--help"),
        ("run.sh", "help"),
        ("uninstall.sh", "--help"),
    ] {
        let output = Command::new("bash")
            .arg(script)
            .arg(arg)
            .output()
            .unwrap_or_else(|error| panic!("failed to run {} {}: {}", script, arg, error));

        assert!(
            output.status.success(),
            "{} {} failed:\nstdout:\n{}\nstderr:\n{}",
            script,
            arg,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(unix)]
#[test]
fn release_installer_explains_the_source_only_arm_path() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().expect("tempdir should be available");
    let bin_dir = temp_dir.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("wrapper directory should be created");
    let uname_path = bin_dir.join("uname");
    fs::write(&uname_path, "#!/usr/bin/env sh\nprintf 'aarch64\\n'\n")
        .expect("uname wrapper should be written");
    let mut permissions = fs::metadata(&uname_path)
        .expect("uname wrapper metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&uname_path, permissions).expect("uname wrapper should be executable");

    let inherited_path = env::var("PATH").expect("PATH should be set for installer test");
    let output = Command::new("bash")
        .arg("install.sh")
        .arg("--release")
        .env("PATH", format!("{}:{inherited_path}", bin_dir.display()))
        .output()
        .expect("release installer should run");

    assert!(
        !output.status.success(),
        "an ARM release-only install must not pretend an archive exists"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("no prebuilt RustDroid release is published for aarch64"),
        "installer should explain the missing ARM binary:\n{combined}"
    );
    assert!(
        combined.contains("use --source"),
        "installer should offer the supported source fallback:\n{combined}"
    );
}
