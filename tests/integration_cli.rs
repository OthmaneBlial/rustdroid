mod common;

use common::{
    assert_output_contains, assert_success, read_to_string, run_command, rustdroid_command,
    TestContext,
};

#[test]
fn version_command_returns_current_version() {
    let context = TestContext::new();
    let output = run_command(rustdroid_command(&context).arg("version"));
    assert_success(&output);
    assert_output_contains(&output, env!("CARGO_PKG_VERSION"));
}

#[test]
fn config_init_writes_isolated_temp_config() {
    let context = TestContext::new();
    let output =
        run_command(rustdroid_command(&context).args(["config", "init", "--profile", "host-fast"]));

    assert_success(&output);
    let contents = read_to_string(&context.config_path);
    assert!(contents.contains("runtime_backend = \"host\""));
    assert!(contents.contains("ui_backend = \"scrcpy\""));
}

#[test]
fn profile_use_updates_existing_config() {
    let context = TestContext::new();
    let init =
        run_command(rustdroid_command(&context).args(["config", "init", "--profile", "host-fast"]));
    assert_success(&init);

    let output = run_command(rustdroid_command(&context).args(["profile", "use", "low-ram"]));

    assert_success(&output);
    let contents = read_to_string(&context.config_path);
    assert!(contents.contains("emulator_ram_mb = 2048"));
    assert!(contents.contains("headless = true"));
}
