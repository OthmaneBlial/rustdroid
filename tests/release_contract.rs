use std::path::Path;
use std::process::Command;

#[test]
fn release_assets_exist_in_repo() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/host-integration.yml",
        ".github/workflows/publish-crate.yml",
        ".github/workflows/release.yml",
        "docs/performance-baselines.json",
        "docs/package-distribution.md",
        "docs/performance-notes/v0.1.0.md",
        "docs/release-announcement-checklist.md",
        "docs/release-rollback.md",
        "docs/releases/v0.1.0.md",
        "docs/version-bump-checklist.md",
        "install.sh",
        "uninstall.sh",
        "scripts/ci-host-check.sh",
        "scripts/ci-package-check.sh",
        "scripts/ci-shell-check.sh",
        "scripts/check-cargo-distribution.sh",
        "scripts/check-performance-baseline.sh",
        "scripts/generate-release-notes.sh",
        "scripts/package-release.sh",
        "scripts/verify-release-install.sh",
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
fn install_and_package_scripts_are_executable() {
    for path in [
        "install.sh",
        "uninstall.sh",
        "scripts/ci-host-check.sh",
        "scripts/ci-package-check.sh",
        "scripts/ci-shell-check.sh",
        "scripts/check-cargo-distribution.sh",
        "scripts/check-performance-baseline.sh",
        "scripts/generate-release-notes.sh",
        "scripts/package-release.sh",
        "scripts/verify-release-install.sh",
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
    for (script, arg) in [("install.sh", "--help"), ("uninstall.sh", "--help")] {
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
