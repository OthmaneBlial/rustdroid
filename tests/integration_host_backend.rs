mod common;

use std::{
    net::TcpListener,
    path::PathBuf,
    process::Command,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

use common::{
    assert_output_contains, assert_success, read_to_string, run_command, rustdroid_command,
    TestContext,
};

const ENABLE_ENV: &str = "RUSTDROID_RUN_HOST_BACKEND_TESTS";
const HOST_SERIAL_ENV: &str = "RUSTDROID_HOST_TEST_SERIAL";
const HOST_AVD_ENV: &str = "RUSTDROID_HOST_TEST_AVD";
const SCRCPY_ENV: &str = "RUSTDROID_RUN_HOST_SCRCPY_TESTS";
const FIXTURE_APK: &str = "tests/fixtures/apks/launch-success.apk";
const FIXTURE_PACKAGE: &str = "com.rustdroid.fixture.launch";
const DEFAULT_TEST_EMULATOR_ARGS: &str =
    "-no-audio -no-boot-anim -no-snapshot -no-snapshot-save -no-metrics -camera-back none -camera-front none -skip-adb-auth -read-only";

struct ManagedHostHarness {
    context: TestContext,
    container_name: String,
    adb_serial: String,
    host_emulator_port: u16,
    host_avd_name: String,
}

impl ManagedHostHarness {
    fn new(label: &str) -> Self {
        let context = TestContext::new();
        let suffix = unique_suffix();
        let container_name = format!("rustdroid-{label}-{suffix}");
        let host_emulator_port = find_free_even_port();

        Self {
            context,
            container_name,
            adb_serial: format!("emulator-{host_emulator_port}"),
            host_emulator_port,
            host_avd_name: configured_host_avd(),
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
            .arg("--host-avd-name")
            .arg(&self.host_avd_name)
            .arg("--headless")
            .arg("true")
            .arg(format!(
                "--emulator-additional-args={DEFAULT_TEST_EMULATOR_ARGS}"
            ));
        command
    }

    fn state_dir(&self) -> PathBuf {
        std::env::temp_dir()
            .join("rustdroid")
            .join("host")
            .join(self.container_name.clone())
    }
}

impl Drop for ManagedHostHarness {
    fn drop(&mut self) {
        let mut command = self.command();
        command.arg("stop").arg("--timeout-secs").arg("5");
        let _ = command.output();
    }
}

struct RunningHostHarness {
    context: TestContext,
    container_name: String,
    adb_serial: String,
    host_emulator_port: u16,
    artifacts_dir: PathBuf,
}

impl RunningHostHarness {
    fn new(label: &str) -> Self {
        let context = TestContext::new();
        let suffix = unique_suffix();
        let container_name = format!("rustdroid-{label}-{suffix}");
        let adb_serial = configured_host_serial().expect("host serial should be configured");
        let host_emulator_port =
            serial_port(&adb_serial).expect("emulator serial should contain a port");
        let artifacts_dir = context
            .config_path
            .parent()
            .expect("tempdir root")
            .join("artifacts");

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
            .arg("true");
        command
    }
}

fn host_backend_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn host_backend_enabled() -> bool {
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

fn configured_host_avd() -> String {
    std::env::var(HOST_AVD_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "test_avd".to_owned())
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

fn find_free_even_port() -> u16 {
    for candidate in (5660..5800).step_by(2) {
        if TcpListener::bind(("127.0.0.1", candidate)).is_ok()
            && TcpListener::bind(("127.0.0.1", candidate + 1)).is_ok()
        {
            return candidate;
        }
    }

    panic!("failed to allocate a free even emulator port pair in the host backend test range");
}

fn best_effort_stop_all(context: &TestContext) {
    let mut command = rustdroid_command(context);
    command
        .arg("stop")
        .arg("--all")
        .arg("--timeout-secs")
        .arg("5");
    let _ = command.output();
}

fn skip_if_backend_disabled() -> bool {
    if !host_backend_enabled() {
        eprintln!(
            "skipping host backend integration tests; set {}=1 to enable them",
            ENABLE_ENV
        );
        return true;
    }

    false
}

#[test]
fn host_backend_detects_avds_and_managed_start_stop() {
    if skip_if_backend_disabled() {
        return;
    }

    let _guard = host_backend_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let harness = ManagedHostHarness::new("host-backend");
    best_effort_stop_all(&harness.context);

    let avds_output = run_command(harness.command().arg("--json").arg("avds"));
    assert_success(&avds_output);
    let avds_json: Value = serde_json::from_slice(&avds_output.stdout).expect("avds json");
    let avds = avds_json["avds"].as_array().expect("avds array");
    assert!(
        avds.iter()
            .any(|entry| entry.as_str() == Some(harness.host_avd_name.as_str())),
        "expected avd list to include {}",
        harness.host_avd_name
    );

    let start_output = run_command(harness.command().arg("start").arg("--wait").arg("false"));
    assert_success(&start_output);
    assert_output_contains(&start_output, "launching host emulator");
    assert!(
        harness.state_dir().join("emulator.pid").is_file(),
        "expected managed host pid file to exist after start"
    );
    assert!(
        harness.state_dir().join("avd.name").is_file(),
        "expected managed host avd marker to exist after start"
    );

    let stop_output = run_command(harness.command().arg("stop").arg("--timeout-secs").arg("5"));
    assert_success(&stop_output);
    assert!(
        !harness.state_dir().join("emulator.pid").exists(),
        "expected host pid file to be removed after stop"
    );
}

#[test]
fn host_backend_run_writes_artifacts_for_running_device() {
    if skip_if_backend_disabled() {
        return;
    }
    let Some(_) = configured_host_serial() else {
        eprintln!(
            "skipping host backend artifact test; set {} to a running emulator serial",
            HOST_SERIAL_ENV
        );
        return;
    };

    let _guard = host_backend_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let harness = RunningHostHarness::new("host-artifacts");

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
        harness.artifacts_dir.join("logcat.txt").is_file(),
        "expected logcat artifact"
    );
    assert!(
        read_to_string(&harness.artifacts_dir.join("run-summary.json")).contains(FIXTURE_PACKAGE),
        "run summary should mention the fixture package"
    );
}

#[test]
fn host_backend_scrcpy_path_is_opt_in() {
    if skip_if_backend_disabled() {
        return;
    }
    let Some(_) = configured_host_serial() else {
        eprintln!(
            "skipping host backend scrcpy test; set {} to a running emulator serial",
            HOST_SERIAL_ENV
        );
        return;
    };
    if std::env::var(SCRCPY_ENV)
        .map(|value| !matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(true)
    {
        eprintln!(
            "skipping host backend scrcpy test; set {}=1 to enable it",
            SCRCPY_ENV
        );
        return;
    }
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipping host backend scrcpy test; no DISPLAY or WAYLAND_DISPLAY is available");
        return;
    }

    let _guard = host_backend_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let harness = RunningHostHarness::new("host-scrcpy");
    let mut command = harness.command();
    command
        .arg("--headless")
        .arg("false")
        .arg("--ui-backend")
        .arg("scrcpy")
        .arg("open")
        .arg("--wait")
        .arg("true");
    let output = run_command(&mut command);
    assert_success(&output);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("scrcpy")
            || String::from_utf8_lossy(&output.stdout).contains("scrcpy"),
        "expected scrcpy-related output"
    );
}
