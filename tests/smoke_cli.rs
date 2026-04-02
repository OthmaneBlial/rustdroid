mod common;

use common::{assert_output_contains, assert_success, run_command, rustdroid_command, TestContext};

#[test]
fn help_lists_primary_daily_commands() {
    let context = TestContext::new();
    let output = run_command(rustdroid_command(&context).arg("--help"));

    assert_success(&output);
    assert_output_contains(&output, "open");
    assert_output_contains(&output, "launch");
    assert_output_contains(&output, "run");
    assert_output_contains(&output, "doctor");
}

#[test]
fn doctor_json_returns_check_array() {
    let context = TestContext::new();
    let output = run_command(rustdroid_command(&context).args(["--json", "doctor"]));

    assert_success(&output);
    assert_output_contains(&output, "\"checks\"");
}
