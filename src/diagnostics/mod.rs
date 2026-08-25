use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
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
    cli::{
        BackendScope, Cli, Command as CliCommand, CompletionShell, RuntimeBackend, SelfTestArgs,
        SetupArgs, SetupDistro,
    },
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
    id: &'static str,
    name: String,
    required: bool,
    state: CheckState,
    summary: String,
    hint: Option<String>,
    remediation: Vec<String>,
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
    schema_version: u8,
    selected_backend: String,
    checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, Serialize)]
struct SetupStep {
    title: String,
    reason: String,
    commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SetupPlan {
    schema_version: u8,
    distro: String,
    detected_from: String,
    changes_applied: bool,
    steps: Vec<SetupStep>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RuntimePlan {
    schema_version: u8,
    dry_run: bool,
    command: String,
    runtime_backend: String,
    profile: Option<String>,
    adb_serial: String,
    host_avd_name: Option<String>,
    inputs: Vec<String>,
    state_effects: Vec<String>,
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

pub fn print_setup(args: &SetupArgs, json: bool) -> Result<()> {
    let plan = setup_plan(args.distro);

    if json {
        return print_json(&plan);
    }

    println!("RustDroid setup plan ({})", plan.distro);
    println!("No changes have been applied. Review each command before running it.");
    for (index, step) in plan.steps.iter().enumerate() {
        println!("{}. {} — {}", index + 1, step.title, step.reason);
        for command in &step.commands {
            println!("   {command}");
        }
    }
    for note in &plan.notes {
        println!("note: {note}");
    }

    Ok(())
}

pub fn print_runtime_plan(config: &RuntimeConfig, command: &CliCommand, json: bool) -> Result<()> {
    let plan = runtime_plan(config, command);

    if json {
        return print_json(&plan);
    }

    println!("RustDroid dry-run: {}", plan.command);
    println!(
        "backend={} profile={} serial={} avd={}",
        plan.runtime_backend,
        plan.profile.as_deref().unwrap_or("custom"),
        plan.adb_serial,
        plan.host_avd_name.as_deref().unwrap_or("not selected"),
    );
    if !plan.inputs.is_empty() {
        println!("inputs: {}", plan.inputs.join(", "));
    }
    for effect in plan.state_effects {
        println!("- {effect}");
    }
    println!("No emulator, container, APK, process, or file was changed.");
    Ok(())
}

fn runtime_plan(config: &RuntimeConfig, command: &CliCommand) -> RuntimePlan {
    let (command_name, inputs, state_effects) = match command {
        CliCommand::Bench(args) => (
            "bench",
            args.apk
                .as_ref()
                .map(|path| input_labels(std::slice::from_ref(path)))
                .unwrap_or_default(),
            vec![
                "check runtime readiness".to_owned(),
                "start or reuse an emulator".to_owned(),
                "measure boot and optional install/launch stages".to_owned(),
            ],
        ),
        CliCommand::FastLocal(args) => (
            "fast-local",
            args.apk
                .as_ref()
                .map(|path| input_labels(std::slice::from_ref(path)))
                .unwrap_or_else(|| vec!["app-debug.apk".to_owned()]),
            vec![
                "check Docker runtime readiness".to_owned(),
                "start or reuse an emulator".to_owned(),
                "install and launch the APK".to_owned(),
            ],
        ),
        CliCommand::Start(_) | CliCommand::Open(_) => (
            if matches!(command, CliCommand::Open(_)) {
                "open"
            } else {
                "start"
            },
            Vec::new(),
            vec![
                "check runtime readiness".to_owned(),
                "start or reuse an emulator".to_owned(),
                "record managed runtime state when RustDroid starts it".to_owned(),
            ],
        ),
        CliCommand::Install(args) => (
            "install",
            input_labels(&args.apks),
            vec![
                "start or reuse an emulator".to_owned(),
                "upload and inspect the APK input".to_owned(),
                "install or replace the package".to_owned(),
            ],
        ),
        CliCommand::Launch(args) => (
            "launch",
            args.input
                .as_ref()
                .map(|path| input_labels(std::slice::from_ref(path)))
                .unwrap_or_else(|| args.package.clone().into_iter().collect()),
            vec![
                "start or reuse an emulator".to_owned(),
                "resolve package and launch activity".to_owned(),
                "bring the package to the foreground".to_owned(),
            ],
        ),
        CliCommand::Uninstall(args) => (
            "uninstall",
            args.input
                .as_ref()
                .map(|path| input_labels(std::slice::from_ref(path)))
                .unwrap_or_else(|| args.package.clone().into_iter().collect()),
            vec![
                "start or reuse an emulator".to_owned(),
                "remove the selected package from the emulator".to_owned(),
            ],
        ),
        CliCommand::ClearData(args) => (
            "clear-data",
            args.input
                .as_ref()
                .map(|path| input_labels(std::slice::from_ref(path)))
                .unwrap_or_else(|| args.package.clone().into_iter().collect()),
            vec![
                "start or reuse an emulator".to_owned(),
                "clear application data on the emulator".to_owned(),
            ],
        ),
        CliCommand::Run(args) => (
            "run",
            input_labels(&args.apks),
            vec![
                "check runtime readiness".to_owned(),
                "start or reuse an emulator".to_owned(),
                "upload, inspect, and install the APK input".to_owned(),
                "launch the resolved activity and collect logs".to_owned(),
                "write a receipt when an artifacts directory is configured".to_owned(),
                if args.keep_alive {
                    "leave a RustDroid-managed runtime running".to_owned()
                } else {
                    "stop the managed runtime after the receipt".to_owned()
                },
            ],
        ),
        CliCommand::Watch(args) => (
            "watch",
            input_labels(std::slice::from_ref(&args.path)),
            vec![
                "watch the path for an APK, .apks, or .xapk change".to_owned(),
                "start or reuse an emulator for each detected build".to_owned(),
                "install and launch each selected input".to_owned(),
            ],
        ),
        CliCommand::Logs(_) => (
            "logs",
            Vec::new(),
            vec![
                "start or reuse an emulator".to_owned(),
                "stream the requested log source".to_owned(),
            ],
        ),
        CliCommand::Stop(_) => (
            "stop",
            Vec::new(),
            vec![
                "find RustDroid-managed runtime state".to_owned(),
                "stop managed emulator or container processes".to_owned(),
            ],
        ),
        _ => unreachable!("runtime plans are only requested for runtime commands"),
    };

    RuntimePlan {
        schema_version: 1,
        dry_run: true,
        command: command_name.to_owned(),
        runtime_backend: format_backend(config.runtime_backend).to_owned(),
        profile: if matches!(command, CliCommand::FastLocal(_)) {
            Some("fast-local".to_owned())
        } else {
            config.active_profile.clone()
        },
        adb_serial: config.adb_serial.clone(),
        host_avd_name: config.host_avd_name.clone(),
        inputs,
        state_effects,
    }
}

fn input_labels(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("input")
                .to_owned()
        })
        .collect()
}

fn setup_plan(requested: SetupDistro) -> SetupPlan {
    let (detected, detected_from) = detected_setup_distro();
    let distro = match requested {
        SetupDistro::Auto => detected,
        explicit => Some(explicit),
    };

    let Some(distro) = distro else {
        return SetupPlan {
            schema_version: 1,
            distro: "unsupported".to_owned(),
            detected_from,
            changes_applied: false,
            steps: Vec::new(),
            notes: vec![
                "Automatic setup plans are available for Ubuntu/Debian and Fedora Linux only."
                    .to_owned(),
                "Use --distro ubuntu, --distro debian, or --distro fedora to preview a supported plan."
                    .to_owned(),
            ],
        };
    };

    let (label, packages, install_command, scrcpy_command) = match distro {
        SetupDistro::Ubuntu => (
            "ubuntu",
            "openjdk-17-jdk unzip wget curl qemu-kvm",
            "sudo apt-get install --yes openjdk-17-jdk unzip wget curl qemu-kvm",
            "sudo apt-get install --yes scrcpy",
        ),
        SetupDistro::Debian => (
            "debian",
            "openjdk-17-jdk unzip wget curl qemu-kvm",
            "sudo apt-get install --yes openjdk-17-jdk unzip wget curl qemu-kvm",
            "sudo apt-get install --yes scrcpy",
        ),
        SetupDistro::Fedora => (
            "fedora",
            "java-17-openjdk-devel unzip wget curl qemu-kvm",
            "sudo dnf install --assumeyes java-17-openjdk-devel unzip wget curl qemu-kvm",
            "sudo dnf install --assumeyes scrcpy",
        ),
        SetupDistro::Auto => unreachable!("auto is resolved before building a setup plan"),
    };

    let update_command = match distro {
        SetupDistro::Ubuntu | SetupDistro::Debian => "sudo apt-get update",
        SetupDistro::Fedora => "sudo dnf makecache",
        SetupDistro::Auto => unreachable!("auto is resolved before building a setup plan"),
    };

    SetupPlan {
        schema_version: 1,
        distro: label.to_owned(),
        detected_from,
        changes_applied: false,
        steps: vec![
            setup_step(
                "Install system prerequisites",
                format!("Installs Java, archive tools, and KVM support ({packages})."),
                vec![update_command, install_command],
            ),
            setup_step(
                "Grant KVM access",
                "Adds the current user to the KVM group; start a new login session afterwards.",
                vec!["sudo usermod -aG kvm \"$USER\"", "newgrp kvm"],
            ),
            setup_step(
                "Install Android command-line tools",
                "Download the current Linux command-line tools archive from developer.android.com, then unpack it under $ANDROID_SDK_ROOT/cmdline-tools/latest.",
                vec![
                    "export ANDROID_SDK_ROOT=\"${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}\"",
                    "mkdir -p \"$ANDROID_SDK_ROOT/cmdline-tools/latest\"",
                    "# https://developer.android.com/studio#command-tools",
                ],
            ),
            setup_step(
                "Expose Android tools in this shell",
                "Makes sdkmanager, adb, and emulator discoverable; add these exports to your shell profile after verifying them.",
                vec![
                    "export ANDROID_SDK_ROOT=\"${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}\"",
                    "export PATH=\"$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/emulator:$PATH\"",
                ],
            ),
            setup_step(
                "Install the reproducible Android SDK set",
                "Uses API 35 and an x86_64 Google APIs image, the documented RustDroid host-fixture baseline.",
                vec![
                    "sdkmanager --licenses",
                    "sdkmanager \"platform-tools\" \"emulator\" \"build-tools;35.0.0\" \"platforms;android-35\" \"system-images;android-35;google_apis;x86_64\"",
                ],
            ),
            setup_step(
                "Create the documented AVD",
                "Creates the test_avd used by the demo and CI commands.",
                vec![
                    "echo no | avdmanager create avd --force --name test_avd --package \"system-images;android-35;google_apis;x86_64\" --device pixel_5",
                    "emulator -list-avds",
                ],
            ),
            setup_step(
                "Optionally install the native emulator UI",
                "scrcpy is optional; RustDroid can remain headless without it.",
                vec![scrcpy_command],
            ),
            setup_step(
                "Verify before running an APK",
                "Checks the selected host backend and then exercises the checked-in fixture.",
                vec![
                    "rustdroid --runtime-backend host doctor",
                    "rustdroid --profile host-fast --host-avd-name test_avd run tests/fixtures/apks/launch-success.apk --duration-secs 2 --keep-alive false --artifacts-dir artifacts/rustdroid-demo",
                ],
            ),
        ],
        notes: vec![
            "This command never executes the plan, writes config, accepts Android licenses, or runs sudo."
                .to_owned(),
            "Use `rustdroid --json setup` when a provisioning script needs the same reviewable plan."
                .to_owned(),
        ],
    }
}

fn setup_step(
    title: impl Into<String>,
    reason: impl Into<String>,
    commands: Vec<impl Into<String>>,
) -> SetupStep {
    SetupStep {
        title: title.into(),
        reason: reason.into(),
        commands: commands.into_iter().map(Into::into).collect(),
    }
}

fn detected_setup_distro() -> (Option<SetupDistro>, String) {
    if std::env::consts::OS != "linux" {
        return (None, format!("current platform: {}", std::env::consts::OS));
    }

    let Ok(raw) = fs::read_to_string("/etc/os-release") else {
        return (None, "could not read /etc/os-release".to_owned());
    };
    let values = parse_os_release(&raw);
    let id = values.get("ID").map(String::as_str).unwrap_or_default();
    let id_like = values
        .get("ID_LIKE")
        .map(String::as_str)
        .unwrap_or_default();
    let distro = match id {
        "ubuntu" => Some(SetupDistro::Ubuntu),
        "debian" => Some(SetupDistro::Debian),
        "fedora" => Some(SetupDistro::Fedora),
        _ if id_like.split_whitespace().any(|value| value == "debian") => Some(SetupDistro::Debian),
        _ if id_like.split_whitespace().any(|value| value == "fedora") => Some(SetupDistro::Fedora),
        _ => None,
    };

    (distro, format!("/etc/os-release ID={id}"))
}

fn parse_os_release(raw: &str) -> BTreeMap<String, String> {
    raw.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            (
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            )
        })
        .collect()
}

pub async fn run_doctor(config: &RuntimeConfig, json: bool) -> Result<()> {
    let checks = collect_doctor_checks(config).await;
    let report = DoctorReport {
        schema_version: 1,
        selected_backend: format_backend(config.runtime_backend).to_owned(),
        checks,
    };

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
    let host_required = matches!(config.runtime_backend, RuntimeBackend::Host);
    let docker_required = matches!(config.runtime_backend, RuntimeBackend::Docker);
    let mut checks = vec![
        check_kvm_device(host_required),
        check_kvm_permissions(host_required),
        check_gpu_passthrough(),
    ];
    checks.push(check_docker(docker_required).await);
    checks.push(check_android_sdk_root(host_required));

    for program in ["emulator", "adb", "aapt", "apkanalyzer", "scrcpy"] {
        checks.push(check_host_tool(program, host_required));
    }

    checks.push(check_host_avds(&config.host_emulator_binary, host_required).await);
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
        let requirement = if check.required {
            "required"
        } else {
            "optional"
        };
        println!(
            "[{status}] {} ({}, {requirement}): {}",
            check.name, check.id, check.summary
        );
        if let Some(hint) = &check.hint {
            println!("  hint: {hint}");
        }
        for command in &check.remediation {
            println!("  fix: {command}");
        }
    }
}

fn check_kvm_device(required: bool) -> CheckResult {
    match fs::metadata("/dev/kvm") {
        Ok(metadata) => check_result(
            "host.kvm.device",
            "kvm",
            required,
            CheckState::Pass,
            format!(
                "found /dev/kvm (mode {:o})",
                metadata.permissions().mode() & 0o777
            ),
            None,
            &[],
        ),
        Err(_) => check_result(
            "host.kvm.device",
            "kvm",
            required,
            unavailable_state(required),
            "missing /dev/kvm",
            Some("enable KVM or run on a Linux host with hardware virtualization"),
            &["rustdroid setup --distro ubuntu"],
        ),
    }
}

fn check_kvm_permissions(required: bool) -> CheckResult {
    if !Path::new("/dev/kvm").exists() {
        return check_result(
            "host.kvm.permissions",
            "kvm_permissions",
            required,
            CheckState::Warn,
            "skipped because /dev/kvm is missing",
            None,
            &[],
        );
    }

    match OpenOptions::new().read(true).write(true).open("/dev/kvm") {
        Ok(_) => check_result(
            "host.kvm.permissions",
            "kvm_permissions",
            required,
            CheckState::Pass,
            "current user can open /dev/kvm",
            None,
            &[],
        ),
        Err(error) => check_result(
            "host.kvm.permissions",
            "kvm_permissions",
            required,
            unavailable_state(required),
            format!("cannot access /dev/kvm: {error}"),
            Some("add your user to the kvm group or fix /dev/kvm permissions"),
            &["sudo usermod -aG kvm \"$USER\"", "newgrp kvm"],
        ),
    }
}

fn check_gpu_passthrough() -> CheckResult {
    let dri_path = Path::new("/dev/dri");
    if !dri_path.exists() {
        return check_result(
            "docker.gpu_passthrough",
            "gpu_passthrough",
            false,
            CheckState::Warn,
            "missing /dev/dri",
            Some("Docker GPU passthrough is limited without /dev/dri"),
            &[],
        );
    }

    let mut entries: Vec<String> = fs::read_dir(dri_path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();

    check_result(
        "docker.gpu_passthrough",
        "gpu_passthrough",
        false,
        CheckState::Pass,
        format!("found /dev/dri ({})", entries.join(", ")),
        None,
        &[],
    )
}

async fn check_docker(required: bool) -> CheckResult {
    match DockerRuntime::connect() {
        Ok(runtime) => match runtime.ping().await {
            Ok(()) => check_result(
                "docker.daemon",
                "docker",
                required,
                CheckState::Pass,
                "Docker daemon is reachable",
                None,
                &[],
            ),
            Err(error) => check_result(
                "docker.daemon",
                "docker",
                required,
                unavailable_state(required),
                format!("Docker is installed but not ready: {error}"),
                Some("start the Docker daemon if you want the Docker backend"),
                &["sudo systemctl enable --now docker"],
            ),
        },
        Err(error) => check_result(
            "docker.daemon",
            "docker",
            required,
            unavailable_state(required),
            format!("Docker client not available: {error}"),
            Some("install Docker only if you plan to use the Docker backend"),
            &["rustdroid setup --distro ubuntu"],
        ),
    }
}

fn check_android_sdk_root(required: bool) -> CheckResult {
    match android_sdk_root() {
        Some(path) => check_result(
            "host.android_sdk.root",
            "android_sdk",
            required,
            CheckState::Pass,
            format!("found Android SDK at {}", path.display()),
            None,
            &[],
        ),
        None => check_result(
            "host.android_sdk.root",
            "android_sdk",
            required,
            unavailable_state(required),
            "Android SDK root was not detected",
            Some("set ANDROID_HOME or ANDROID_SDK_ROOT if you want the host backend"),
            &[
                "export ANDROID_SDK_ROOT=\"$HOME/Android/Sdk\"",
                "rustdroid setup --distro ubuntu",
            ],
        ),
    }
}

fn check_host_tool(program: &str, host_required: bool) -> CheckResult {
    match resolve_host_tool(program) {
        Ok(path) => check_result(
            host_tool_id(program),
            program,
            host_required && program != "scrcpy",
            CheckState::Pass,
            format!("resolved to {}", path.display()),
            None,
            &[],
        ),
        Err(error) => {
            let required = host_required && program != "scrcpy";
            let state = unavailable_state(required);
            let hint = match program {
                "scrcpy" => Some("install scrcpy if you want the native desktop UI".to_owned()),
                "emulator" | "adb" => Some(
                    "install Android SDK platform-tools and emulator packages, or expose them on PATH"
                        .to_owned(),
                ),
                _ => Some("install Android SDK build-tools or expose them on PATH".to_owned()),
            };

            check_result(
                host_tool_id(program),
                program,
                required,
                state,
                error.to_string(),
                hint.as_deref(),
                &["rustdroid setup --distro ubuntu"],
            )
        }
    }
}

async fn check_host_avds(emulator_binary: &str, required: bool) -> CheckResult {
    match list_host_avds(emulator_binary).await {
        Ok(avds) if avds.is_empty() => check_result(
            "host.avds",
            "avds",
            required,
            unavailable_state(required),
            "no Android Virtual Devices found",
            Some("create an AVD in Android Studio to use the host backend"),
            &["rustdroid setup --distro ubuntu", "emulator -list-avds"],
        ),
        Ok(avds) => check_result(
            "host.avds",
            "avds",
            required,
            CheckState::Pass,
            format!("found {} AVD(s): {}", avds.len(), avds.join(", ")),
            None,
            &[],
        ),
        Err(error) => check_result(
            "host.avds",
            "avds",
            required,
            unavailable_state(required),
            error.to_string(),
            Some("create an AVD or fix the host emulator install"),
            &["rustdroid setup --distro ubuntu", "emulator -list-avds"],
        ),
    }
}

fn check_result(
    id: &'static str,
    name: impl Into<String>,
    required: bool,
    state: CheckState,
    summary: impl Into<String>,
    hint: Option<&str>,
    remediation: &[&str],
) -> CheckResult {
    CheckResult {
        id,
        name: name.into(),
        required,
        state,
        summary: summary.into(),
        hint: hint.map(str::to_owned),
        remediation: remediation
            .iter()
            .map(|command| (*command).to_owned())
            .collect(),
    }
}

fn unavailable_state(required: bool) -> CheckState {
    if required {
        CheckState::Fail
    } else {
        CheckState::Warn
    }
}

fn host_tool_id(program: &str) -> &'static str {
    match program {
        "emulator" => "host.tool.emulator",
        "adb" => "host.tool.adb",
        "aapt" => "host.tool.aapt",
        "apkanalyzer" => "host.tool.apkanalyzer",
        "scrcpy" => "host.tool.scrcpy",
        _ => "host.tool.unknown",
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
