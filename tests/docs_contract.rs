use std::path::Path;

#[test]
fn contributor_and_guide_docs_exist() {
    for path in [
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
        "SUPPORT.md",
        "CHANGELOG.md",
        "docs/1.0-checklist.md",
        "docs/benchmarking.md",
        "docs/changelog-policy.md",
        "docs/community.md",
        "docs/demo.md",
        "docs/configuration.md",
        "docs/first-install.md",
        "docs/host-backend.md",
        "docs/host-fast-vs-docker.md",
        "docs/support-scope.md",
        "docs/support-matrix.md",
        "docs/troubleshooting.md",
        "docs/releases/v0.3.0.md",
        "docs/releases/v0.3.1.md",
        "docs/versioning-policy.md",
        "docs/ci-examples.md",
        "docs/fixture-testing.md",
        "docs/github-action.md",
        "docs/release-process.md",
        "docs/release-security-checklist.md",
        "docs/receipt-schema-v1.md",
        "docs/quickstart-linux.md",
        "docs/operation-plans.md",
        "docs/recipes.md",
        "docs/reference-workflows.md",
        "docs/benchmarking.md",
        "docs/support-matrix.json",
        ".github/workflows/action-contract.yml",
        ".github/ISSUE_TEMPLATE/bug-report.yml",
        ".github/ISSUE_TEMPLATE/setup-failure.yml",
        ".github/ISSUE_TEMPLATE/feature-request.yml",
        ".github/PULL_REQUEST_TEMPLATE.md",
        "examples/configs/host-fast.rustdroid.toml",
        "examples/configs/headless-ci.rustdroid.toml",
        "examples/configs/low-ram.rustdroid.toml",
        "examples/workflows/gradle-android-receipt.yml",
        "examples/workflows/flutter-receipt.yml",
        "examples/workflows/react-native-expo-receipt.yml",
    ] {
        assert!(Path::new(path).is_file(), "missing required doc: {path}");
    }
}

#[test]
fn readme_links_to_the_main_guides() {
    let readme = std::fs::read_to_string("README.md").expect("read README");

    for snippet in [
        "watch build/outputs/apk/debug",
        "assets/rustdroid-proof.svg",
        "docs/demo.md",
        "docs/configuration.md",
        "docs/1.0-checklist.md",
        "docs/first-install.md",
        "docs/quickstart-linux.md",
        "docs/host-backend.md",
        "docs/host-fast-vs-docker.md",
        "docs/support-scope.md",
        "docs/support-matrix.md",
        "docs/troubleshooting.md",
        "docs/operation-plans.md",
        "docs/recipes.md",
        "docs/reference-workflows.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "SUPPORT.md",
        "CHANGELOG.md",
    ] {
        assert!(
            readme.contains(snippet),
            "expected README to mention {snippet}"
        );
    }
}

#[test]
fn troubleshooting_and_fixture_guides_preserve_their_public_contracts() {
    let troubleshooting =
        std::fs::read_to_string("docs/troubleshooting.md").expect("read troubleshooting guide");
    for check_id in [
        "host.kvm.device",
        "host.kvm.permissions",
        "host.android_sdk.root",
        "host.tool.emulator",
        "host.tool.adb",
        "host.tool.aapt",
        "host.tool.apkanalyzer",
        "host.tool.scrcpy",
        "host.avds",
        "docker.daemon",
        "docker.gpu_passthrough",
    ] {
        assert!(
            troubleshooting.contains(check_id),
            "troubleshooting guide must document doctor ID {check_id}"
        );
    }
    assert!(troubleshooting.contains("rustdroid --json doctor"));
    assert!(troubleshooting.contains("rustdroid setup --distro ubuntu"));

    let fixtures =
        std::fs::read_to_string("docs/fixture-testing.md").expect("read fixture testing guide");
    for fixture in [
        "launch-success.apk",
        "missing-launcher.apk",
        "x86_64-native.apk",
        "arm64-native.apk",
        "split-base.apk",
        "split-config.en.apk",
    ] {
        assert!(
            fixtures.contains(fixture),
            "fixture guide must document {fixture}"
        );
    }
    assert!(fixtures.contains("do not replace it with an application APK"));
}

#[test]
fn roadmap_distinguishes_delivery_from_external_evidence() {
    let roadmap = std::fs::read_to_string("ROADMAP.md").expect("read roadmap");

    for snippet in [
        "## Delivery status -- 2026-08-26",
        "v0.3.1",
        "Do not add a reliability badge until four consecutive scheduled runs pass.",
        "External adoption and public maturity",
        "success measures, not build artifacts",
        "prepared social preview only after explicit approval",
    ] {
        assert!(
            roadmap.contains(snippet),
            "roadmap delivery status must contain {snippet}"
        );
    }
}
