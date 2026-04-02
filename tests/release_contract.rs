use std::path::Path;

#[test]
fn release_assets_exist_in_repo() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "docs/release-announcement-checklist.md",
        "docs/release-rollback.md",
        "docs/releases/v0.1.0.md",
        "docs/version-bump-checklist.md",
        "install.sh",
        "uninstall.sh",
        "scripts/ci-host-check.sh",
        "scripts/ci-package-check.sh",
        "scripts/ci-shell-check.sh",
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
