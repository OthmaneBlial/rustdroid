mod common;

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
