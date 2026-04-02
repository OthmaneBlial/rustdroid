use std::path::Path;

#[test]
fn contributor_and_guide_docs_exist() {
    for path in [
        "CONTRIBUTING.md",
        "docs/first-install.md",
        "docs/host-backend.md",
        "docs/troubleshooting.md",
        "docs/ci-examples.md",
        "docs/fixture-testing.md",
        "docs/release-process.md",
    ] {
        assert!(Path::new(path).is_file(), "missing required doc: {path}");
    }
}

#[test]
fn readme_links_to_the_main_guides() {
    let readme = std::fs::read_to_string("README.md").expect("read README");

    for snippet in [
        "watch build/outputs/apk/debug",
        "docs/first-install.md",
        "docs/host-backend.md",
        "docs/troubleshooting.md",
        "CONTRIBUTING.md",
    ] {
        assert!(
            readme.contains(snippet),
            "expected README to mention {snippet}"
        );
    }
}
