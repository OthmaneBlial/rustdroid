use std::process::Command;
use std::{env, fs, path::Path};

#[test]
fn release_assets_exist_in_repo() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/action-contract.yml",
        ".github/workflows/host-integration.yml",
        ".github/workflows/publish-crate.yml",
        ".github/workflows/release.yml",
        "docs/performance-baselines.json",
        "docs/package-distribution.md",
        "docs/performance-notes/v0.1.0.md",
        "docs/release-announcement-checklist.md",
        "docs/release-rollback.md",
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
        "scripts/generate-support-matrix.sh",
        "scripts/package-release.sh",
        "scripts/verify-release-install.sh",
        "scripts/verify-release-install-container.sh",
        "README.md",
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
fn action_contract_uses_an_immutable_revision() {
    let workflow = std::fs::read_to_string(".github/workflows/action-contract.yml")
        .expect("read action contract workflow");

    assert!(workflow.contains("OthmaneBlial/rustdroid@3f4184ce1117591f9b06cafec48f2ffad1809ecc"));
    assert!(workflow.contains("tests/fixtures/apks/launch-success.apk"));
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
