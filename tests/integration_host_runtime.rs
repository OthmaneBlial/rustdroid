mod common;

use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

use common::{
    assert_output_contains, assert_success, read_to_string, run_command, rustdroid_command,
    TestContext,
};

const ENABLE_ENV: &str = "RUSTDROID_RUN_HOST_RUNTIME_TESTS";
const HOST_SERIAL_ENV: &str = "RUSTDROID_HOST_TEST_SERIAL";
const FIXTURE_APK: &str = "tests/fixtures/apks/launch-success.apk";
const FIXTURE_PACKAGE: &str = "com.rustdroid.fixture.launch";
const FIXTURE_ACTIVITY: &str = "com.rustdroid.fixture.launch.MainActivity";
const DEFAULT_TEST_EMULATOR_ARGS: &str =
    "-no-audio -no-boot-anim -no-snapshot -no-snapshot-save -no-metrics -camera-back none -camera-front none -skip-adb-auth -read-only";

struct HostRuntimeHarness {
    context: TestContext,
    container_name: String,
    adb_serial: String,
    host_emulator_port: u16,
    artifacts_dir: PathBuf,
}

impl HostRuntimeHarness {
    fn new(label: &str) -> Self {
        let context = TestContext::new();
        let suffix = unique_suffix();
        let container_name = format!("rustdroid-{label}-{suffix}");
        let artifacts_dir = context
            .config_path
            .parent()
            .expect("tempdir root")
            .join("artifacts");

        let adb_serial = configured_host_serial().expect("host serial should be configured");
        let host_emulator_port =
            serial_port(&adb_serial).expect("emulator serial should contain a port");

        Self {
            context,
            container_name,
            adb_serial,
            host_emulator_port,
            artifacts_dir,
        }
    }

    fn command(&self) -> Command {
        let mut command = rustdroid_command(&self.context);
        command
            .arg("--runtime-backend")
            .arg("host")
            .arg("--container-name")
            .arg(&self.container_name)
            .arg("--adb-serial")
            .arg(&self.adb_serial)
            .arg("--host-emulator-port")
            .arg(self.host_emulator_port.to_string())
            .arg("--headless")
            .arg("true")
            .arg("--emulator-gpu-mode")
            .arg("swiftshader_indirect")
            .arg(format!(
                "--emulator-additional-args={DEFAULT_TEST_EMULATOR_ARGS}"
            ));

        command
    }
}

impl Drop for HostRuntimeHarness {
    fn drop(&mut self) {
        let mut command = self.command();
        command.arg("stop").arg("--timeout-secs").arg("5");
        let _ = command.output();
    }
}

fn host_runtime_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn host_runtime_enabled() -> bool {
    std::env::var(ENABLE_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn configured_host_serial() -> Option<String> {
    std::env::var(HOST_SERIAL_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn unique_suffix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time")
        .as_millis();
    format!("{}-{millis}", std::process::id())
}

fn serial_port(serial: &str) -> Option<u16> {
    serial
        .strip_prefix("emulator-")
        .and_then(|value| value.parse::<u16>().ok())
}

fn assert_output_contains_any(output: &Output, needles: &[&str]) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        needles
            .iter()
            .any(|needle| stdout.contains(needle) || stderr.contains(needle)),
        "expected output to contain one of {:?}\nstdout:\n{}\nstderr:\n{}",
        needles,
        stdout,
        stderr
    );
}

fn best_effort_stop_all(harness: &HostRuntimeHarness) {
    let mut command = harness.command();
    command
        .arg("stop")
        .arg("--all")
        .arg("--timeout-secs")
        .arg("5");
    let _ = command.output();
}

fn emit_logcat_marker(serial: &str, marker: &str) {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "log", "-t", "rustdroid", marker])
        .output()
        .expect("adb log command should run");
    assert_success(&output);
}

#[test]
fn host_runtime_health_commands_work() {
    if !host_runtime_enabled() {
        eprintln!(
            "skipping host runtime integration tests; set {}=1 to enable them",
            ENABLE_ENV
        );
        return;
    }
    if configured_host_serial().is_none() {
        eprintln!(
            "skipping host runtime integration tests; set {} to a running emulator serial",
            HOST_SERIAL_ENV
        );
        return;
    }

    let _guard = host_runtime_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let harness = HostRuntimeHarness::new("health");
    best_effort_stop_all(&harness);

    let doctor_output = run_command(harness.command().arg("--json").arg("doctor"));
    assert_success(&doctor_output);
    let doctor_json: Value =
        serde_json::from_slice(&doctor_output.stdout).expect("doctor json should parse");
    assert!(doctor_json["checks"].as_array().is_some());

    let self_test_output = run_command(
        harness
            .command()
            .arg("--json")
            .arg("self-test")
            .arg("--backend")
            .arg("host")
            .arg("--full"),
    );
    assert_success(&self_test_output);
    let self_test_json: Value =
        serde_json::from_slice(&self_test_output.stdout).expect("self-test json should parse");
    assert_eq!(self_test_json["results"][0]["backend"], "host");
    assert_eq!(self_test_json["results"][0]["ok"], true);
}

#[test]
fn host_runtime_apk_commands_work() {
    if !host_runtime_enabled() {
        eprintln!(
            "skipping host runtime integration tests; set {}=1 to enable them",
            ENABLE_ENV
        );
        return;
    }
    if configured_host_serial().is_none() {
        eprintln!(
            "skipping host runtime integration tests; set {} to a running emulator serial",
            HOST_SERIAL_ENV
        );
        return;
    }

    let _guard = host_runtime_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let harness = HostRuntimeHarness::new("apk-flow");
    best_effort_stop_all(&harness);

    let open_output = run_command(harness.command().arg("open").arg("--wait").arg("false"));
    assert_success(&open_output);
    assert_output_contains_any(
        &open_output,
        &[
            "launching host emulator",
            "reusing existing unmanaged host emulator",
        ],
    );

    let install_output = run_command(harness.command().arg("install").arg(FIXTURE_APK));
    assert_success(&install_output);
    assert_output_contains(&install_output, FIXTURE_PACKAGE);

    let launch_output = run_command(
        harness
            .command()
            .arg("launch")
            .arg("--package")
            .arg(FIXTURE_PACKAGE)
            .arg("--activity")
            .arg(FIXTURE_ACTIVITY),
    );
    assert_success(&launch_output);
    assert_output_contains(&launch_output, "launching com.rustdroid.fixture.launch");

    let clear_data_output = run_command(
        harness
            .command()
            .arg("clear-data")
            .arg("--package")
            .arg(FIXTURE_PACKAGE),
    );
    assert_success(&clear_data_output);
    assert_output_contains(&clear_data_output, "clearing data");

    let log_marker = format!("rustdroid-host-runtime-{}", unique_suffix());
    emit_logcat_marker(&harness.adb_serial, &log_marker);
    let logs_output = run_command(
        harness
            .command()
            .arg("logs")
            .arg("--source")
            .arg("logcat")
            .arg("--duration-secs")
            .arg("2"),
    );
    assert_success(&logs_output);
    assert_output_contains(&logs_output, "[logcat]");
    assert_output_contains(&logs_output, &log_marker);

    let run_output = run_command(
        harness
            .command()
            .arg("run")
            .arg(FIXTURE_APK)
            .arg("--duration-secs")
            .arg("2")
            .arg("--keep-alive")
            .arg("false")
            .arg("--artifacts-dir")
            .arg(&harness.artifacts_dir),
    );
    assert_success(&run_output);
    assert_output_contains(&run_output, "summary: backend=host");
    assert!(
        harness.artifacts_dir.join("run-summary.json").is_file(),
        "expected run summary artifact"
    );
    assert!(
        harness.artifacts_dir.join("run-report.html").is_file(),
        "expected html report artifact"
    );
    assert!(
        read_to_string(&harness.artifacts_dir.join("run-summary.json")).contains(FIXTURE_PACKAGE),
        "run summary should mention the fixture package"
    );

    let uninstall_output = run_command(
        harness
            .command()
            .arg("uninstall")
            .arg("--package")
            .arg(FIXTURE_PACKAGE),
    );
    assert_success(&uninstall_output);
    assert_output_contains(&uninstall_output, "uninstalling");

    let stop_all_output = run_command(
        harness
            .command()
            .arg("stop")
            .arg("--all")
            .arg("--timeout-secs")
            .arg("5"),
    );
    assert_success(&stop_all_output);
}
