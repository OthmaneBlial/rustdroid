use std::path::Path;

#[test]
fn contributor_and_guide_docs_exist() {
    for path in [
        "CONTRIBUTING.md",
        "CHANGELOG.md",
        "docs/1.0-checklist.md",
        "docs/changelog-policy.md",
        "docs/demo.md",
        "docs/configuration.md",
        "docs/first-install.md",
        "docs/host-backend.md",
        "docs/support-scope.md",
        "docs/support-matrix.md",
        "docs/troubleshooting.md",
        "docs/versioning-policy.md",
        "docs/ci-examples.md",
        "docs/fixture-testing.md",
        "docs/github-action.md",
        "docs/release-process.md",
        "docs/receipt-schema-v1.md",
        "docs/github-action.md",
        "docs/quickstart-linux.md",
        "docs/receipt-schema-v1.md",
        ".github/workflows/action-contract.yml",
        "examples/configs/host-fast.rustdroid.toml",
        "examples/configs/headless-ci.rustdroid.toml",
        "examples/configs/low-ram.rustdroid.toml",
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
        "docs/support-scope.md",
        "docs/support-matrix.md",
        "docs/troubleshooting.md",
        "CONTRIBUTING.md",
        "CHANGELOG.md",
    ] {
        assert!(
            readme.contains(snippet),
            "expected README to mention {snippet}"
        );
    }
}
