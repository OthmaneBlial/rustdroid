use std::process::Command;
use std::{env, fs, path::Path};

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
fn action_contract_uses_an_immutable_revision() {
    let workflow = std::fs::read_to_string(".github/workflows/action-contract.yml")
        .expect("read action contract workflow");

    assert!(workflow.contains("OthmaneBlial/rustdroid@964ed16d32d4fa12b52dea21b95484a7b96e9854"));
    assert!(workflow.contains("tests/fixtures/apks/launch-success.apk"));
}

#[test]
fn stack_reference_workflows_boot_and_upload_the_pinned_receipt_action() {
    let pinned_action = "OthmaneBlial/rustdroid@964ed16d32d4fa12b52dea21b95484a7b96e9854";

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
