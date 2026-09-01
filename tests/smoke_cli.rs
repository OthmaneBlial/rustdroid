mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use common::{assert_output_contains, assert_success, run_command, rustdroid_command, TestContext};

#[test]
fn help_lists_primary_daily_commands() {
    let context = TestContext::new();
    let output = run_command(rustdroid_command(&context).arg("--help"));

    assert_success(&output);
    assert_output_contains(&output, "open");
    assert_output_contains(&output, "launch");
    assert_output_contains(&output, "run");
    assert_output_contains(&output, "fast-local");
    assert_output_contains(&output, "doctor");
}

#[test]
fn doctor_json_returns_check_array() {
    let context = TestContext::new();
    let output = run_command(rustdroid_command(&context).args(["--json", "doctor"]));

    assert!(
        matches!(output.status.code(), Some(0 | 10)),
        "expected doctor to return success or the documented doctor failure code, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_output_contains(&output, "\"checks\"");
    assert_output_contains(&output, "\"schema_version\"");
    assert_output_contains(&output, "\"selected_backend\"");
    assert_output_contains(&output, "\"required\"");
    assert_output_contains(&output, "\"remediation\"");
}

#[test]
fn setup_json_is_a_reviewable_non_destructive_plan() {
    let context = TestContext::new();
    let output =
        run_command(rustdroid_command(&context).args(["--json", "setup", "--distro", "ubuntu"]));

    assert_success(&output);
    assert_output_contains(&output, "\"schema_version\"");
    assert_output_contains(&output, "\"changes_applied\": false");
    assert_output_contains(&output, "test_avd");
}

#[test]
fn dry_run_prints_a_runtime_plan_without_needing_an_emulator() {
    let context = TestContext::new();
    let output = run_command(rustdroid_command(&context).args([
        "--json",
        "--dry-run",
        "--runtime-backend",
        "host",
        "run",
        "tests/fixtures/apks/launch-success.apk",
    ]));

    assert_success(&output);
    assert_output_contains(&output, "\"dry_run\": true");
    assert_output_contains(&output, "\"host\"");
    assert_output_contains(&output, "start or reuse an emulator");
}

#[test]
fn missing_input_writes_a_safe_failure_receipt_before_runtime_start() {
    let context = TestContext::new();
    let artifacts_dir = context
        .config_path
        .parent()
        .expect("test config should have a parent")
        .join("failure-artifacts");
    let private_input = context
        .config_path
        .parent()
        .expect("test config should have a parent")
        .join("private-missing.apk");
    let output = run_command(rustdroid_command(&context).args([
        "--runtime-backend",
        "host",
        "run",
        private_input.to_str().expect("UTF-8 fixture path"),
        "--artifacts-dir",
        artifacts_dir.to_str().expect("UTF-8 artifact path"),
        "--keep-alive",
        "false",
    ]));

    assert_eq!(output.status.code(), Some(1));
    assert_output_contains(&output, "status=failed");
    assert_output_contains(&output, "failure_stage=input_preflight");

    let summary = fs::read_to_string(artifacts_dir.join("run-summary.json"))
        .expect("failure summary should exist");
    assert!(summary.contains("\"status\": \"failed\""));
    assert!(summary.contains("\"failure_stage\": \"input_preflight\""));
    assert!(summary.contains("\"failure_classification\": \"input\""));
    assert!(summary.contains("the Android artifact could not be prepared"));
    assert!(!summary.contains(private_input.to_string_lossy().as_ref()));

    let junit =
        fs::read_to_string(artifacts_dir.join("junit.xml")).expect("failure JUnit should exist");
    assert!(junit.contains("failures=\"1\""));
    assert!(junit.contains("<failure type=\"input\""));
    assert!(artifacts_dir.join("run-report.html").is_file());
    assert!(artifacts_dir.join("run-summary.md").is_file());
}

#[test]
fn smoke_matrix_entrypoint_lists_required_cases() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/run-smoke-matrix.sh");

    let output = Command::new("bash")
        .arg(&script)
        .arg("--list")
        .output()
        .expect("smoke matrix script should run");

    assert_success(&output);
    assert_output_contains(&output, "host-fast");
    assert_output_contains(&output, "host-headless");
    assert_output_contains(&output, "split-install");
}
