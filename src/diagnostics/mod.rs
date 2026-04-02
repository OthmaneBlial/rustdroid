use std::{
    fs::{self, OpenOptions},
    os::unix::fs::PermissionsExt,
    path::Path,
    time::Instant,
};

use anyhow::{bail, Context, Result};
use clap::CommandFactory;
use clap_complete::{
    generate,
    shells::{Bash, Zsh},
};
use serde::Serialize;
use tokio::process::Command;

use crate::{
    cli::{BackendScope, Cli, CompletionShell, SelfTestArgs},
    config::RuntimeConfig,
    docker::DockerRuntime,
    host::{android_sdk_root, list_host_avds, resolve_host_tool},
    output::print_json,
    runtime::Runtime,
};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckState {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
struct CheckResult {
    name: String,
    state: CheckState,
    summary: String,
    hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DeviceEntry {
    serial: String,
    state: String,
    details: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SelfTestResult {
    backend: String,
    ok: bool,
    duration_ms: u128,
    steps: Vec<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VersionInfo {
    version: String,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorReport {
    checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, Serialize)]
struct DevicesReport {
    devices: Vec<DeviceEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct AvdReport {
    avds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SelfTestReport {
    results: Vec<SelfTestResult>,
}

pub fn print_version(json: bool) -> Result<()> {
    let version = VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    if json {
        return print_json(&version);
    }

    println!("rustdroid {}", version.version);
    Ok(())
}

pub fn print_completions(shell: CompletionShell) {
    let mut command = Cli::command();
    match shell {
        CompletionShell::Bash => generate(Bash, &mut command, "rustdroid", &mut std::io::stdout()),
        CompletionShell::Zsh => generate(Zsh, &mut command, "rustdroid", &mut std::io::stdout()),
    }
}

pub async fn run_doctor(config: &RuntimeConfig, json: bool) -> Result<()> {
    let checks = collect_doctor_checks(config).await;
    let report = DoctorReport { checks };

    if json {
        print_json(&report)?;
    } else {
        print_doctor_checks(&report.checks);
    }

    let failures = report
        .checks
        .iter()
        .filter(|check| check.state == CheckState::Fail)
        .count();
    if failures > 0 {
        bail!("doctor found {failures} failing checks");
    }

    Ok(())
}

pub async fn run_devices(json: bool) -> Result<()> {
    let adb = resolve_host_tool("adb")?;
    let output = Command::new(&adb)
        .args(["devices", "-l"])
        .output()
        .await
        .with_context(|| format!("failed to run {}", adb.display()))?;

    if !output.status.success() {
        bail!(
            "adb devices failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let report = DevicesReport {
        devices: parse_adb_devices(&String::from_utf8_lossy(&output.stdout)),
    };

    if json {
        return print_json(&report);
    }

    if report.devices.is_empty() {
        println!("No adb devices found.");
        return Ok(());
    }

    for device in report.devices {
        if device.details.is_empty() {
            println!("{}  {}", device.serial, device.state);
            continue;
        }

        println!(
            "{}  {}  {}",
            device.serial,
            device.state,
            device.details.join(" ")
        );
    }

    Ok(())
}

pub async fn run_avds(config: &RuntimeConfig, json: bool) -> Result<()> {
    let report = AvdReport {
        avds: list_host_avds(&config.host_emulator_binary).await?,
    };

    if json {
        return print_json(&report);
    }

    if report.avds.is_empty() {
        println!("No Android Virtual Devices found.");
        return Ok(());
    }

    for avd in report.avds {
        println!("{avd}");
    }

    Ok(())
}

pub async fn run_self_test(config: &RuntimeConfig, args: &SelfTestArgs, json: bool) -> Result<()> {
    let mut results = Vec::new();
    for backend in selected_backends(config.runtime_backend, args.backend) {
        results.push(self_test_backend(config, backend, args.full).await);
    }

    let report = SelfTestReport { results };

    if json {
        print_json(&report)?;
    } else {
        print_self_test_results(&report.results);
    }

    let mut failures = 0;
    for result in &report.results {
        if !result.ok {
            failures += 1;
        }
    }

    if failures > 0 {
        bail!("self-test failed for {failures} backend(s)");
    }

    Ok(())
}

fn print_self_test_results(results: &[SelfTestResult]) {
    for result in results {
        if result.ok {
            println!(
                "[PASS] {} self-test completed in {} ms",
                result.backend, result.duration_ms
            );
        } else {
            println!(
                "[FAIL] {} self-test failed in {} ms",
                result.backend, result.duration_ms
            );
        }

        for step in &result.steps {
            println!("  - {step}");
        }

        if let Some(error) = &result.error {
            println!("  - error: {error}");
        }
    }
}

async fn collect_doctor_checks(config: &RuntimeConfig) -> Vec<CheckResult> {
    let mut checks = vec![
        check_kvm_device(),
        check_kvm_permissions(),
        check_gpu_passthrough(),
    ];
    checks.push(check_docker().await);
    checks.push(check_android_sdk_root());

    for program in ["emulator", "adb", "aapt", "apkanalyzer", "scrcpy"] {
        checks.push(check_host_tool(program));
    }

    checks.push(check_host_avds(&config.host_emulator_binary).await);
    checks
}

fn print_doctor_checks(checks: &[CheckResult]) {
    println!("RustDroid doctor");
    for check in checks {
        let status = match check.state {
            CheckState::Pass => "PASS",
            CheckState::Warn => "WARN",
            CheckState::Fail => "FAIL",
        };
        println!("[{status}] {}: {}", check.name, check.summary);
        if let Some(hint) = &check.hint {
            println!("  hint: {hint}");
        }
    }
}

fn check_kvm_device() -> CheckResult {
    match fs::metadata("/dev/kvm") {
        Ok(metadata) => CheckResult {
            name: "kvm".to_owned(),
            state: CheckState::Pass,
            summary: format!(
                "found /dev/kvm (mode {:o})",
                metadata.permissions().mode() & 0o777
            ),
            hint: None,
        },
        Err(_) => CheckResult {
            name: "kvm".to_owned(),
            state: CheckState::Fail,
            summary: "missing /dev/kvm".to_owned(),
            hint: Some("enable KVM or run on a Linux host with hardware virtualization".to_owned()),
        },
    }
}

fn check_kvm_permissions() -> CheckResult {
    if !Path::new("/dev/kvm").exists() {
        return CheckResult {
            name: "kvm_permissions".to_owned(),
            state: CheckState::Warn,
            summary: "skipped because /dev/kvm is missing".to_owned(),
            hint: None,
        };
    }

    match OpenOptions::new().read(true).write(true).open("/dev/kvm") {
        Ok(_) => CheckResult {
            name: "kvm_permissions".to_owned(),
            state: CheckState::Pass,
            summary: "current user can open /dev/kvm".to_owned(),
            hint: None,
        },
        Err(error) => CheckResult {
            name: "kvm_permissions".to_owned(),
            state: CheckState::Fail,
            summary: format!("cannot access /dev/kvm: {error}"),
            hint: Some("add your user to the kvm group or fix /dev/kvm permissions".to_owned()),
        },
    }
}

fn check_gpu_passthrough() -> CheckResult {
    let dri_path = Path::new("/dev/dri");
    if !dri_path.exists() {
        return CheckResult {
            name: "gpu_passthrough".to_owned(),
            state: CheckState::Warn,
            summary: "missing /dev/dri".to_owned(),
            hint: Some("Docker GPU passthrough is limited without /dev/dri".to_owned()),
        };
    }

    let mut entries: Vec<String> = fs::read_dir(dri_path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();

    CheckResult {
        name: "gpu_passthrough".to_owned(),
        state: CheckState::Pass,
        summary: format!("found /dev/dri ({})", entries.join(", ")),
        hint: None,
    }
}

async fn check_docker() -> CheckResult {
    match DockerRuntime::connect() {
        Ok(runtime) => match runtime.ping().await {
            Ok(()) => CheckResult {
                name: "docker".to_owned(),
                state: CheckState::Pass,
                summary: "Docker daemon is reachable".to_owned(),
                hint: None,
            },
            Err(error) => CheckResult {
                name: "docker".to_owned(),
                state: CheckState::Warn,
                summary: format!("Docker is installed but not ready: {error}"),
                hint: Some("start the Docker daemon if you want the Docker backend".to_owned()),
            },
        },
        Err(error) => CheckResult {
            name: "docker".to_owned(),
            state: CheckState::Warn,
            summary: format!("Docker client not available: {error}"),
            hint: Some("install Docker only if you plan to use the Docker backend".to_owned()),
        },
    }
}

fn check_android_sdk_root() -> CheckResult {
    match android_sdk_root() {
        Some(path) => CheckResult {
            name: "android_sdk".to_owned(),
            state: CheckState::Pass,
            summary: format!("found Android SDK at {}", path.display()),
            hint: None,
        },
        None => CheckResult {
            name: "android_sdk".to_owned(),
            state: CheckState::Warn,
            summary: "Android SDK root was not detected".to_owned(),
            hint: Some(
                "set ANDROID_HOME or ANDROID_SDK_ROOT if you want the host backend".to_owned(),
            ),
        },
    }
}

fn check_host_tool(program: &str) -> CheckResult {
    match resolve_host_tool(program) {
        Ok(path) => CheckResult {
            name: program.to_owned(),
            state: CheckState::Pass,
            summary: format!("resolved to {}", path.display()),
            hint: None,
        },
        Err(error) => {
            let state = if program == "scrcpy" {
                CheckState::Warn
            } else {
                CheckState::Fail
            };
            let hint = match program {
                "scrcpy" => Some("install scrcpy if you want the native desktop UI".to_owned()),
                "emulator" | "adb" => Some(
                    "install Android SDK platform-tools and emulator packages, or expose them on PATH"
                        .to_owned(),
                ),
                _ => Some("install Android SDK build-tools or expose them on PATH".to_owned()),
            };

            CheckResult {
                name: program.to_owned(),
                state,
                summary: error.to_string(),
                hint,
            }
        }
    }
}

async fn check_host_avds(emulator_binary: &str) -> CheckResult {
    match list_host_avds(emulator_binary).await {
        Ok(avds) if avds.is_empty() => CheckResult {
            name: "avds".to_owned(),
            state: CheckState::Warn,
            summary: "no Android Virtual Devices found".to_owned(),
            hint: Some("create an AVD in Android Studio to use the host backend".to_owned()),
        },
        Ok(avds) => CheckResult {
            name: "avds".to_owned(),
            state: CheckState::Pass,
            summary: format!("found {} AVD(s): {}", avds.len(), avds.join(", ")),
            hint: None,
        },
        Err(error) => CheckResult {
            name: "avds".to_owned(),
            state: CheckState::Warn,
            summary: error.to_string(),
            hint: Some("create an AVD or fix the host emulator install".to_owned()),
        },
    }
}

fn parse_adb_devices(stdout: &str) -> Vec<DeviceEntry> {
    stdout
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?.to_owned();
            let state = parts.next()?.to_owned();
            let details = parts.map(str::to_owned).collect();
            Some(DeviceEntry {
                serial,
                state,
                details,
            })
        })
        .collect()
}

fn selected_backends(
    current: crate::cli::RuntimeBackend,
    scope: BackendScope,
) -> Vec<crate::cli::RuntimeBackend> {
    use crate::cli::RuntimeBackend;

    match scope {
        BackendScope::Current => vec![current],
        BackendScope::Docker => vec![RuntimeBackend::Docker],
        BackendScope::Host => vec![RuntimeBackend::Host],
        BackendScope::Both => vec![RuntimeBackend::Docker, RuntimeBackend::Host],
    }
}

async fn self_test_backend(
    base_config: &RuntimeConfig,
    backend: crate::cli::RuntimeBackend,
    full: bool,
) -> SelfTestResult {
    let started = Instant::now();
    let mut config = base_config.clone();
    config.runtime_backend = backend;
    config.headless = true;
    config.container_name = format!(
        "{}-self-test-{}",
        base_config.container_name,
        match backend {
            crate::cli::RuntimeBackend::Docker => "docker",
            crate::cli::RuntimeBackend::Host => "host",
        }
    );

    let result = async {
        let mut steps = Vec::new();
        let runtime = Runtime::connect(&config)?;
        runtime.ping().await?;
        steps.push("backend connectivity check passed".to_owned());

        if matches!(backend, crate::cli::RuntimeBackend::Host)
            && list_host_avds(&config.host_emulator_binary)
                .await?
                .is_empty()
        {
            bail!("host backend has no available AVDs");
        }

        if full {
            runtime.ensure_started(&config).await?;
            steps.push("emulator start smoke check passed".to_owned());
            runtime.stop(&config, 15).await?;
            steps.push("emulator stop smoke check passed".to_owned());
        } else {
            steps.push(
                "full emulator boot skipped (use --full to start and stop a test instance)"
                    .to_owned(),
            );
        }

        Ok::<Vec<String>, anyhow::Error>(steps)
    }
    .await;

    let duration_ms = started.elapsed().as_millis();

    match result {
        Ok(steps) => SelfTestResult {
            backend: format_backend(backend).to_owned(),
            ok: true,
            duration_ms,
            steps,
            error: None,
        },
        Err(error) => {
            let _ = cleanup_self_test(&config).await;
            SelfTestResult {
                backend: format_backend(backend).to_owned(),
                ok: false,
                duration_ms,
                steps: Vec::new(),
                error: Some(error.to_string()),
            }
        }
    }
}

async fn cleanup_self_test(config: &RuntimeConfig) -> Result<()> {
    let runtime = Runtime::connect(config)?;
    runtime.stop(config, 5).await
}

fn format_backend(backend: crate::cli::RuntimeBackend) -> &'static str {
    match backend {
        crate::cli::RuntimeBackend::Docker => "docker",
        crate::cli::RuntimeBackend::Host => "host",
    }
}
