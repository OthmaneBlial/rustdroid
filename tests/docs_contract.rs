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
        "docs/performance-notes/v0.3.1.md",
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
        "examples/apps/README.md",
        "examples/apps/gradle-android/scripts/build-debug-apk.sh",
        "examples/apps/flutter/scripts/build-debug-apk.sh",
        "examples/apps/expo-prebuild/scripts/build-debug-apk.sh",
        ".github/workflows/reference-stack-fixtures.yml",
    ] {
        assert!(Path::new(path).is_file(), "missing required doc: {path}");
    }
}

#[test]
fn readme_links_to_the_main_guides() {
    let readme = std::fs::read_to_string("README.md").expect("read README");

    for snippet in [
        "actions/workflows/ci.yml/badge.svg?branch=main",
        "img.shields.io/github/v/release/OthmaneBlial/rustdroid",
        "img.shields.io/github/license/OthmaneBlial/rustdroid",
        "Rust-stable",
        "watch build/outputs/apk/debug",
        "assets/rustdroid-proof.svg",
        "assets/rustdroid-demo.gif",
        "https://othmaneblial.github.io/rustdroid/",
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
        "examples/apps/README.md",
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
fn static_site_ships_local_docs_and_subpath_safe_assets() {
    let index = std::fs::read_to_string("site/index.html").expect("read static site home");
    let docs = std::fs::read_to_string("site/docs.html").expect("read static site docs");
    let docs_script =
        std::fs::read_to_string("site/docs.js").expect("read static site docs script");

    for snippet in [
        "docs.html",
        "assets/rustdroid-proof.svg",
        "assets/rustdroid-demo.gif",
        "APK path in.",
    ] {
        assert!(index.contains(snippet), "site home must preserve {snippet}");
    }

    for snippet in ["RustDroid documentation", "doc-navigation", "docs.js"] {
        assert!(docs.contains(snippet), "site docs must preserve {snippet}");
    }

    for snippet in [
        "first-install.md",
        "ci-examples.md",
        "assets/${cleanUrl.slice",
    ] {
        assert!(
            docs_script.contains(snippet),
            "site docs reader must preserve {snippet}"
        );
    }

    for document in [
        "site/docs/first-install.md",
        "site/docs/ci-examples.md",
        "site/docs/receipt-schema-v1.md",
        "site/assets/rustdroid-proof.svg",
        "site/assets/rustdroid-demo.gif",
    ] {
        assert!(
            std::path::Path::new(document).is_file(),
            "site must ship {document}"
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
        "First manual host proof",
        "second manual host proof",
        "third manual host proof",
        "32905475830",
        "32907975602/attempts/2",
        "32915494009",
        "32911691564",
        "32913034524",
    ] {
        assert!(
            roadmap.contains(snippet),
            "roadmap delivery status must contain {snippet}"
        );
    }
}

#[test]
fn v031_release_notes_publish_reproducible_receipt_timings() {
    let release =
        std::fs::read_to_string("docs/releases/v0.3.1.md").expect("read v0.3.1 release notes");
    let notes = std::fs::read_to_string("docs/performance-notes/v0.3.1.md")
        .expect("read v0.3.1 performance notes");

    for snippet in [
        "Measured receipt path",
        "mean ± sample standard deviation",
        "Gradle Android",
        "Flutter",
        "Expo prebuild",
        "Boot",
        "Install",
        "Launch",
        "Total",
    ] {
        assert!(
            release.contains(snippet),
            "v0.3.1 release notes must publish {snippet}"
        );
    }

    for snippet in [
        "32911691564",
        "32913034524",
        "32914348192",
        "ubuntu-22.04",
        "API 35",
        "n = 3",
        "No Docker comparison",
    ] {
        assert!(
            notes.contains(snippet),
            "v0.3.1 performance notes must preserve {snippet}"
        );
    }
}

#[test]
fn support_matrix_only_marks_linked_successful_contracts_as_verified() {
    let matrix =
        std::fs::read_to_string("docs/support-matrix.json").expect("read generated support matrix");

    for (workflow, run_id) in [
        ("host-integration-runtime", "32907975602/attempts/2"),
        ("fresh-machine-contract", "32905479252"),
        ("action-contract", "32907241889"),
    ] {
        assert!(
            matrix.contains(workflow),
            "support matrix must name the verified {workflow} contract"
        );
        assert!(
            matrix.contains(run_id),
            "support matrix must link the successful {workflow} run"
        );
    }

    assert_eq!(
        matrix
            .matches("\"verification_state\": \"verified\"")
            .count(),
        3,
        "the matrix must mark only its three linked contract combinations as verified"
    );
}

#[test]
fn executable_stack_fixtures_preserve_their_documented_build_contracts() {
    let guide = std::fs::read_to_string("docs/reference-workflows.md")
        .expect("read reference-workflows guide");
    let workflow = std::fs::read_to_string(".github/workflows/reference-stack-fixtures.yml")
        .expect("read reference stack fixtures workflow");

    for (fixture, apk_path) in [
        (
            "examples/apps/gradle-android",
            "app/build/outputs/apk/debug/app-debug.apk",
        ),
        (
            "examples/apps/flutter",
            "build/app/outputs/flutter-apk/app-debug.apk",
        ),
        (
            "examples/apps/expo-prebuild",
            "android/app/build/outputs/apk/debug/app-debug.apk",
        ),
    ] {
        let script = format!("{fixture}/scripts/build-debug-apk.sh");
        assert!(
            Path::new(&script).is_file(),
            "missing executable fixture script {script}"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::metadata(&script)
                .unwrap_or_else(|error| panic!("read {script} metadata: {error}"))
                .permissions();
            assert_ne!(
                permissions.mode() & 0o111,
                0,
                "fixture script {script} must be executable"
            );
        }
        assert!(
            guide.contains(fixture) && guide.contains(apk_path),
            "reference-workflows guide must document {fixture} and its APK path"
        );
        assert!(
            workflow.contains(fixture) && workflow.contains(apk_path),
            "fixture workflow must build {fixture} and check its APK path"
        );
    }

    for snippet in [
        "workflow_dispatch:",
        "cron: \"0 10 1 * *\"",
        "api-level: 35",
        "OthmaneBlial/rustdroid@964ed16d32d4fa12b52dea21b95484a7b96e9854",
        "Create the immutable RustDroid receipt",
    ] {
        assert!(
            workflow.contains(snippet),
            "reference stack fixture workflow must contain {snippet}"
        );
    }

    for run_id in ["32911691564", "32913034524"] {
        assert!(
            guide.contains(run_id),
            "reference-workflows guide must link the documented stack proof {run_id}"
        );
    }

    for current_action in [
        "actions/checkout@v6",
        "actions/setup-java@v5",
        "actions/setup-node@v6",
        "actions/upload-artifact@v7",
    ] {
        assert!(
            guide.contains(current_action),
            "reference-workflows guide must document the runtime-migrated {current_action}"
        );
    }
}
