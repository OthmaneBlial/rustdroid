mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    fixtures: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct FixtureEntry {
    name: String,
    path: String,
    package: String,
    launchable_activity: Option<String>,
    native_abis: Vec<String>,
    group: Option<String>,
    split: Option<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_manifest() -> FixtureManifest {
    let manifest_path = repo_root().join("tests/fixtures/manifest.json");
    let content = fs::read_to_string(manifest_path).expect("fixture manifest");
    serde_json::from_str(&content).expect("fixture manifest json")
}

fn aapt_available() -> bool {
    Command::new("aapt")
        .arg("version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn fixture_inventory_is_present_and_tiny() {
    let manifest = load_manifest();

    let split_members = manifest
        .fixtures
        .iter()
        .filter(|fixture| fixture.group.as_deref() == Some("split-locale-en"))
        .count();
    assert_eq!(
        split_members, 2,
        "expected split-locale-en to contain two APKs"
    );

    for fixture in manifest.fixtures {
        let path = repo_root().join(&fixture.path);
        let metadata =
            fs::metadata(&path).unwrap_or_else(|_| panic!("missing fixture {}", fixture.name));
        assert!(
            metadata.is_file(),
            "fixture is not a file: {}",
            fixture.name
        );
        assert!(metadata.len() > 0, "fixture is empty: {}", fixture.name);
        assert!(
            metadata.len() < 128 * 1024,
            "fixture is unexpectedly large: {} ({} bytes)",
            fixture.name,
            metadata.len()
        );
    }
}

#[test]
fn fixture_badging_matches_declared_metadata() {
    if !aapt_available() {
        eprintln!("skipping fixture badging validation because aapt is unavailable");
        return;
    }

    let manifest = load_manifest();
    for fixture in manifest.fixtures {
        let path = repo_root().join(&fixture.path);
        let output = Command::new("aapt")
            .args(["dump", "badging"])
            .arg(&path)
            .output()
            .unwrap_or_else(|_| panic!("failed to inspect fixture {}", fixture.name));

        assert!(
            output.status.success(),
            "aapt failed for {}: {}",
            fixture.name,
            String::from_utf8_lossy(&output.stderr)
        );

        let badging = String::from_utf8(output.stdout).expect("utf8 badging");
        assert!(
            badging.contains(&format!("package: name='{}'", fixture.package)),
            "badging package mismatch for {}",
            fixture.name
        );

        match fixture.launchable_activity.as_deref() {
            Some(activity) => assert!(
                badging.contains(&format!("launchable-activity: name='{}'", activity)),
                "expected launchable activity for {}",
                fixture.name
            ),
            None => assert!(
                !badging.contains("launchable-activity:"),
                "unexpected launchable activity for {}",
                fixture.name
            ),
        }

        for abi in fixture.native_abis {
            assert!(
                badging.contains(&format!("'{}'", abi)),
                "missing native ABI {} for {}",
                abi,
                fixture.name
            );
        }

        if let Some(split) = fixture.split.as_deref() {
            assert!(
                badging.contains(&format!("split='{}'", split)),
                "missing split metadata for {}",
                fixture.name
            );
        }
    }
}
