use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Result};
use bollard::container::LogsOptions;
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::time::sleep;

use crate::{
    adb::{AdbClient, ApkMetadata},
    apks::{ObbFile, PreparedApkSet},
    cli::{
        BenchArgs, ClearDataArgs, InstallArgs, LaunchArgs, LogsArgs, OpenArgs, RunArgs, StartArgs,
        StopArgs, UninstallArgs, WatchArgs,
    },
    config::RuntimeConfig,
    display,
    logs::{self, StreamOptions},
    output::print_json,
    runtime::Runtime,
};

#[derive(Debug, Clone)]
pub struct EmulatorOrchestrator {
    config: RuntimeConfig,
    runtime: Runtime,
    adb: AdbClient,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub runtime_backend: String,
    pub container_name: String,
    pub adb_serial: String,
    pub boot_duration_ms: u128,
    pub install_duration_ms: Option<u128>,
    pub launch_duration_ms: Option<u128>,
    pub total_duration_ms: u128,
    pub package_name: Option<String>,
    pub input_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkReceipt {
    schema_version: u8,
    tool_version: String,
    status: String,
    runtime_backend: String,
    profile: Option<String>,
    environment: BenchmarkEnvironment,
    inputs: Vec<ReceiptInput>,
    package_name: Option<String>,
    boot_duration_ms: u128,
    install_duration_ms: Option<u128>,
    launch_duration_ms: Option<u128>,
    total_duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkEnvironment {
    host_os: String,
    host_arch: String,
    host_cpu_cores: Option<usize>,
    runner_image: Option<String>,
    avd_name: Option<String>,
    android_api_level: Option<String>,
    boot_mode: String,
    emulator_cpu_cores: u16,
    emulator_ram_mb: u64,
    emulator_gpu_mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunSummary {
    pub schema_version: u8,
    pub tool_version: String,
    pub status: String,
    pub failure_classification: String,
    pub profile: Option<String>,
    pub runtime_backend: String,
    pub emulator: ReceiptEmulator,
    pub package_name: String,
    pub launchable_activity: Option<String>,
    pub native_abis: Vec<String>,
    pub x86_ready: bool,
    pub uses_arm_translation: bool,
    pub gps_disabled: bool,
    pub boot_duration_ms: u128,
    pub install_duration_ms: u128,
    pub launch_duration_ms: u128,
    pub total_duration_ms: u128,
    pub kept_alive: bool,
    pub crash_summary: Option<String>,
    pub anr_summary: Option<String>,
    pub inputs: Vec<ReceiptInput>,
    pub artifacts: Option<ReceiptArtifacts>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptInput {
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptEmulator {
    pub adb_serial: String,
    pub avd_name: Option<String>,
    pub api_level: Option<String>,
    pub device: String,
    pub headless: bool,
    pub gpu_mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptArtifacts {
    pub json: String,
    pub html: String,
    pub junit: String,
    pub markdown_summary: String,
    pub logcat: Option<String>,
    pub emulator_process_log: Option<String>,
}

#[derive(Debug, Clone)]
struct InstallOutcome {
    metadata: ApkMetadata,
}

#[derive(Debug, Clone, Default)]
struct RunArtifacts {
    process_logs: Option<String>,
    logcat_dump: Option<String>,
    crash_summary: Option<String>,
    anr_summary: Option<String>,
    anr_traces: Option<String>,
    tombstones: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct WatchToken {
    path: PathBuf,
    modified_at_ms: u128,
    size_bytes: u64,
}

impl EmulatorOrchestrator {
    pub fn new(config: RuntimeConfig, runtime: Runtime) -> Self {
        let adb = AdbClient::from_config(&config);
        Self {
            config,
            runtime,
            adb,
        }
    }

    pub async fn start(&self, args: StartArgs) -> Result<()> {
        self.start_device(args.wait, true).await
    }

    pub async fn open(&self, args: OpenArgs) -> Result<()> {
        self.start_device(args.wait, true).await
    }

    pub async fn install(&self, args: InstallArgs) -> Result<()> {
        let prepared = PreparedApkSet::from_inputs(&args.apks)?;
        self.start_device(true, false).await?;
        self.install_prepared_apks(&prepared, args.replace).await?;
        Ok(())
    }

    pub async fn launch(&self, args: LaunchArgs) -> Result<()> {
        self.start_device(true, true).await?;

        if let Some(input) = args.input.as_ref() {
            let prepared = PreparedApkSet::from_inputs(std::slice::from_ref(input))?;
            let mut metadata = self.inspect_prepared_apks(&prepared).await?;
            if let Some(activity) = args.activity.as_ref() {
                metadata.launchable_activity = Some(activity.clone());
            }
            eprintln!(
                "launching {} via APK metadata on {}",
                metadata.package_name,
                self.runtime_backend_name()
            );
            self.adb
                .launch_app(&self.runtime, &self.config, &metadata)
                .await?;
            return Ok(());
        }

        if let Some(package_name) = args.package.as_deref() {
            eprintln!(
                "launching {} via package name on {}",
                package_name,
                self.runtime_backend_name()
            );
            self.adb
                .launch_package(
                    &self.runtime,
                    &self.config,
                    package_name,
                    args.activity.as_deref(),
                )
                .await?;
            return Ok(());
        }

        bail!("launch requires either an APK/archive path or --package <name>");
    }

    pub async fn uninstall(&self, args: UninstallArgs) -> Result<()> {
        self.start_device(true, false).await?;
        let package_name = self
            .resolve_package_name(args.input.as_ref(), args.package)
            .await?;
        eprintln!(
            "uninstalling {} on {}",
            package_name,
            self.runtime_backend_name()
        );
        self.adb
            .uninstall_package(&self.runtime, &self.config, &package_name)
            .await
    }

    pub async fn clear_data(&self, args: ClearDataArgs) -> Result<()> {
        self.start_device(true, false).await?;
        let package_name = self
            .resolve_package_name(args.input.as_ref(), args.package)
            .await?;
        eprintln!(
            "clearing data for {} on {}",
            package_name,
            self.runtime_backend_name()
        );
        self.adb
            .clear_package_data(&self.runtime, &self.config, &package_name)
            .await
    }

    pub async fn run(&self, args: RunArgs) -> Result<()> {
        let prepared = PreparedApkSet::from_inputs(&args.apks)?;
        let inputs = build_receipt_inputs(&args.apks)?;
        let artifacts_dir = args
            .artifacts_dir
            .as_ref()
            .cloned()
            .or_else(|| self.config.artifacts_dir.as_ref().map(PathBuf::from));
        let total_started = Instant::now();

        eprintln!("==> starting emulator on {}", self.runtime_backend_name());
        let boot_started = Instant::now();
        self.start_device(true, true).await?;
        let boot_duration_ms = boot_started.elapsed().as_millis();

        eprintln!("==> installing package set");
        let install_started = Instant::now();
        let install = self.install_prepared_apks(&prepared, args.replace).await?;
        let install_duration_ms = install_started.elapsed().as_millis();

        eprintln!("==> launching {}", install.metadata.package_name);
        let launch_started = Instant::now();
        self.adb
            .launch_app(&self.runtime, &self.config, &install.metadata)
            .await?;
        let launch_duration_ms = launch_started.elapsed().as_millis();

        let stream_result = logs::stream(
            &self.runtime,
            &self.config,
            StreamOptions {
                source: args.log_source,
                duration_secs: args.duration_secs,
                package_name: Some(install.metadata.package_name.clone()),
                since_start: false,
            },
        )
        .await;

        let total_duration_ms = total_started.elapsed().as_millis();
        let (message_crash_summary, message_anr_summary) = stream_result
            .as_ref()
            .err()
            .map(|error| parse_failure_summary(&error.to_string()))
            .unwrap_or((None, None));

        let mut artifacts = RunArtifacts::default();
        if artifacts_dir.is_some() {
            artifacts = self.collect_run_artifacts().await?;
        }

        let crash_summary = message_crash_summary.or_else(|| {
            artifacts
                .logcat_dump
                .as_deref()
                .and_then(extract_logcat_crash_summary)
        });
        let anr_summary = message_anr_summary.or_else(|| {
            artifacts
                .logcat_dump
                .as_deref()
                .and_then(extract_logcat_anr_summary)
        });

        let emulator = ReceiptEmulator {
            adb_serial: self.config.adb_serial.clone(),
            avd_name: self.config.host_avd_name.clone(),
            api_level: self
                .adb
                .get_property(&self.runtime, &self.config, "ro.build.version.sdk")
                .await,
            device: self.config.device.clone(),
            headless: self.config.effective_emulator_headless(),
            gpu_mode: self.config.emulator_gpu_mode.clone(),
        };
        let receipt_artifacts = artifacts_dir.as_ref().map(|_| ReceiptArtifacts {
            json: "run-summary.json".to_owned(),
            html: "run-report.html".to_owned(),
            junit: "junit.xml".to_owned(),
            markdown_summary: "run-summary.md".to_owned(),
            logcat: artifacts
                .logcat_dump
                .as_ref()
                .map(|_| "logcat.txt".to_owned()),
            emulator_process_log: artifacts
                .process_logs
                .as_ref()
                .map(|_| "emulator-process.log".to_owned()),
        });
        let summary = RunSummary {
            schema_version: 1,
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            status: "passed".to_owned(),
            failure_classification: "none".to_owned(),
            profile: self.config.active_profile.clone(),
            runtime_backend: self.runtime_backend_name().to_owned(),
            emulator,
            package_name: install.metadata.package_name.clone(),
            launchable_activity: install.metadata.launchable_activity.clone(),
            native_abis: install.metadata.native_abis.clone(),
            x86_ready: install
                .metadata
                .native_abis
                .iter()
                .any(|abi| abi.starts_with("x86")),
            uses_arm_translation: install.metadata.uses_arm_translation_on_x86_emulator(),
            gps_disabled: !self.config.emulator_enable_gps,
            boot_duration_ms,
            install_duration_ms,
            launch_duration_ms,
            total_duration_ms,
            kept_alive: args.keep_alive,
            crash_summary,
            anr_summary,
            inputs,
            artifacts: receipt_artifacts,
        };

        if let Some(artifacts_dir) = artifacts_dir.as_ref() {
            write_run_artifacts(
                artifacts_dir,
                &summary,
                &RunArtifacts {
                    crash_summary: summary.crash_summary.clone(),
                    anr_summary: summary.anr_summary.clone(),
                    ..artifacts
                },
            )?;
        }

        if let Some(path) = args.junit_path.as_ref() {
            write_optional_report(path, &build_junit_report(&summary))?;
        }
        if let Some(path) = args.markdown_summary_path.as_ref() {
            write_optional_report(path, &build_markdown_summary(&summary))?;
        }

        print_run_summary(&summary);

        if !args.keep_alive {
            eprintln!("==> stopping runtime because --keep-alive=false");
            self.runtime.stop(&self.config, 15).await?;
        }

        stream_result
    }

    pub async fn watch(&self, args: WatchArgs) -> Result<()> {
        let mut last_seen: Option<WatchToken> = None;
        let mut cycles = 0_u32;
        let mut ui_opened = false;

        loop {
            let Some(candidate) = resolve_watch_candidate(&args.path)? else {
                if !args.quiet {
                    eprintln!(
                        "watching {} for .apk, .apks, or .xapk outputs",
                        args.path.display()
                    );
                }
                sleep(std::time::Duration::from_secs(args.poll_interval_secs)).await;
                continue;
            };

            if last_seen.as_ref() == Some(&candidate) {
                sleep(std::time::Duration::from_secs(args.poll_interval_secs)).await;
                continue;
            }

            if args.settle_secs > 0 {
                sleep(std::time::Duration::from_secs(args.settle_secs)).await;
            }

            let Some(stable_candidate) = resolve_watch_candidate(&args.path)? else {
                sleep(std::time::Duration::from_secs(args.poll_interval_secs)).await;
                continue;
            };
            if stable_candidate.path != candidate.path {
                sleep(std::time::Duration::from_secs(args.poll_interval_secs)).await;
                continue;
            }

            cycles += 1;
            if !args.quiet {
                eprintln!(
                    "==> watch cycle {} using {}",
                    cycles,
                    stable_candidate.path.display()
                );
            }

            let prepared =
                PreparedApkSet::from_inputs(std::slice::from_ref(&stable_candidate.path))?;
            self.start_device(true, !ui_opened).await?;
            ui_opened = true;

            let install = self.install_prepared_apks(&prepared, true).await?;
            self.adb
                .launch_app(&self.runtime, &self.config, &install.metadata)
                .await?;

            if let Some(duration_secs) = args.duration_secs {
                logs::stream(
                    &self.runtime,
                    &self.config,
                    StreamOptions {
                        source: args.log_source,
                        duration_secs: Some(duration_secs),
                        package_name: Some(install.metadata.package_name.clone()),
                        since_start: false,
                    },
                )
                .await?;
            }

            if !args.keep_alive {
                self.runtime.stop(&self.config, 15).await?;
                ui_opened = false;
            }

            if !args.quiet {
                eprintln!(
                    "watch cycle {} complete: package={} input={}",
                    cycles,
                    install.metadata.package_name,
                    stable_candidate.path.display()
                );
            }

            last_seen = Some(stable_candidate);
            if args.max_cycles.is_some_and(|limit| cycles >= limit) {
                return Ok(());
            }

            sleep(std::time::Duration::from_secs(args.poll_interval_secs)).await;
        }
    }

    pub async fn bench(&self, args: BenchArgs, json: bool) -> Result<()> {
        let inputs = args
            .apk
            .as_ref()
            .map(|apk| build_receipt_inputs(std::slice::from_ref(apk)))
            .transpose()?
            .unwrap_or_default();
        let total_started = Instant::now();

        eprintln!("==> boot benchmark");
        let boot_started = Instant::now();
        self.start_device(true, false).await?;
        let boot_duration_ms = boot_started.elapsed().as_millis();

        let mut result = BenchResult {
            runtime_backend: self.runtime_backend_name().to_owned(),
            container_name: self.config.container_name.clone(),
            adb_serial: self.config.adb_serial.clone(),
            boot_duration_ms,
            install_duration_ms: None,
            launch_duration_ms: None,
            total_duration_ms: 0,
            package_name: None,
            input_files: args
                .apk
                .as_ref()
                .map(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default(),
        };

        if let Some(apk) = args.apk.as_ref() {
            let prepared = PreparedApkSet::from_inputs(std::slice::from_ref(apk))?;

            eprintln!("==> install benchmark");
            let install_started = Instant::now();
            let install = self.install_prepared_apks(&prepared, args.replace).await?;
            result.install_duration_ms = Some(install_started.elapsed().as_millis());
            result.package_name = Some(install.metadata.package_name.clone());

            eprintln!("==> launch benchmark");
            let launch_started = Instant::now();
            self.adb
                .launch_app(&self.runtime, &self.config, &install.metadata)
                .await?;
            result.launch_duration_ms = Some(launch_started.elapsed().as_millis());
        }

        result.total_duration_ms = total_started.elapsed().as_millis();

        if let Some(artifacts_dir) = args.artifacts_dir.as_ref() {
            let receipt = BenchmarkReceipt {
                schema_version: 1,
                tool_version: env!("CARGO_PKG_VERSION").to_owned(),
                status: "measured".to_owned(),
                runtime_backend: self.runtime_backend_name().to_owned(),
                profile: self.config.active_profile.clone(),
                environment: BenchmarkEnvironment {
                    host_os: std::env::consts::OS.to_owned(),
                    host_arch: std::env::consts::ARCH.to_owned(),
                    host_cpu_cores: std::thread::available_parallelism()
                        .ok()
                        .map(|value| value.get()),
                    runner_image: std::env::var("ImageOS").ok(),
                    avd_name: self.config.host_avd_name.clone(),
                    android_api_level: self
                        .adb
                        .get_property(&self.runtime, &self.config, "ro.build.version.sdk")
                        .await,
                    boot_mode: match self.config.boot_mode {
                        crate::cli::BootMode::Cold => "cold".to_owned(),
                        crate::cli::BootMode::Warm => "warm".to_owned(),
                    },
                    emulator_cpu_cores: self.config.emulator_cpu_cores,
                    emulator_ram_mb: self.config.emulator_ram_mb,
                    emulator_gpu_mode: self.config.emulator_gpu_mode.clone(),
                },
                inputs,
                package_name: result.package_name.clone(),
                boot_duration_ms: result.boot_duration_ms,
                install_duration_ms: result.install_duration_ms,
                launch_duration_ms: result.launch_duration_ms,
                total_duration_ms: result.total_duration_ms,
            };
            write_benchmark_artifacts(artifacts_dir, &receipt)?;
        }

        if json {
            print_json(&result)?;
        } else {
            print_bench_result(&result);
        }

        Ok(())
    }

    pub async fn logs(&self, args: LogsArgs) -> Result<()> {
        self.start_device(false, false).await?;
        eprintln!("streaming logs from {}", self.runtime_backend_name());
        logs::stream(
            &self.runtime,
            &self.config,
            StreamOptions {
                source: args.source,
                duration_secs: args.duration_secs,
                package_name: args.package,
                since_start: args.since_start,
            },
        )
        .await
    }

    pub async fn stop(&self, args: StopArgs) -> Result<()> {
        self.runtime.stop(&self.config, args.timeout_secs).await
    }

    async fn start_device(&self, wait: bool, launch_ui: bool) -> Result<()> {
        self.runtime.ping().await?;
        self.runtime.ensure_started(&self.config).await?;
        if wait {
            eprintln!("waiting for emulator boot completion");
            self.adb
                .wait_for_boot(
                    &self.runtime,
                    &self.config,
                    self.config.boot_timeout_secs,
                    self.config.poll_interval_secs,
                )
                .await?;
            self.adb
                .stabilize_device(&self.runtime, &self.config)
                .await?;
        }
        if launch_ui && wait {
            display::launch_if_needed(&self.config).await?;
        }
        Ok(())
    }

    async fn install_prepared_apks(
        &self,
        prepared: &PreparedApkSet,
        replace: bool,
    ) -> Result<InstallOutcome> {
        let remote_paths = self.upload_apks(&prepared.apk_paths).await?;
        let metadata = self.inspect_uploaded_apks(&remote_paths).await?;
        print_apk_notes(
            &metadata,
            self.runtime_backend_name(),
            !self.config.emulator_enable_gps,
        );
        self.adb
            .install_apks(&self.runtime, &self.config, &remote_paths, replace)
            .await?;
        self.push_obb_files(&metadata, &prepared.obb_files).await?;
        if self.config.compile_installed_package {
            eprintln!("compiling {} for faster relaunches", metadata.package_name);
            if let Err(error) = self
                .adb
                .compile_package(&self.runtime, &self.config, &metadata.package_name)
                .await
            {
                eprintln!(
                    "warning: failed to compile {}: {error}",
                    metadata.package_name
                );
            }
        }

        Ok(InstallOutcome { metadata })
    }

    async fn inspect_prepared_apks(&self, prepared: &PreparedApkSet) -> Result<ApkMetadata> {
        let remote_paths = self.upload_apks(&prepared.apk_paths).await?;
        self.inspect_uploaded_apks(&remote_paths).await
    }

    async fn resolve_package_name(
        &self,
        input: Option<&PathBuf>,
        package_name: Option<String>,
    ) -> Result<String> {
        if let Some(package_name) = package_name {
            return Ok(package_name);
        }

        let input = input.ok_or_else(|| {
            anyhow::anyhow!("command requires either an APK/archive path or --package <name>")
        })?;
        let prepared = PreparedApkSet::from_inputs(std::slice::from_ref(input))?;
        Ok(self.inspect_prepared_apks(&prepared).await?.package_name)
    }

    async fn push_obb_files(&self, metadata: &ApkMetadata, obb_files: &[ObbFile]) -> Result<()> {
        if obb_files.is_empty() {
            return Ok(());
        }

        self.try_enable_shell_obb_access().await;
        let upload_dir = format!("{}/obb", self.config.remote_apk_dir);
        for (index, obb_file) in obb_files.iter().enumerate() {
            let relative_device_path = obb_file.device_relative_path(&metadata.package_name);
            let target_path = format!("/sdcard/Android/obb/{}", relative_device_path.display());
            let parent = relative_device_path
                .parent()
                .map(|path| format!("/sdcard/Android/obb/{}", path.display()))
                .unwrap_or_else(|| "/sdcard/Android/obb".to_owned());

            let uploaded_path = self
                .runtime
                .upload_file(
                    &self.config,
                    &obb_file.local_path,
                    &upload_dir,
                    &format!(
                        "{index}-{}",
                        obb_file
                            .local_path
                            .file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("main.obb")
                    ),
                )
                .await?;

            let mkdir_outcome = self
                .runtime
                .exec(
                    &self.config,
                    vec![
                        "adb".to_owned(),
                        "-s".to_owned(),
                        self.config.adb_serial.clone(),
                        "shell".to_owned(),
                        "mkdir".to_owned(),
                        "-p".to_owned(),
                        parent,
                    ],
                )
                .await?;
            if mkdir_outcome.exit_code != 0 {
                eprintln!(
                    "warning: failed to prepare OBB directory for {} (stderr='{}')",
                    metadata.package_name,
                    mkdir_outcome.stderr.trim()
                );
                continue;
            }

            let push_outcome = self
                .runtime
                .exec(
                    &self.config,
                    vec![
                        "adb".to_owned(),
                        "-s".to_owned(),
                        self.config.adb_serial.clone(),
                        "push".to_owned(),
                        uploaded_path,
                        target_path,
                    ],
                )
                .await?;
            if push_outcome.exit_code != 0 {
                eprintln!(
                    "warning: failed to push OBB for {} (stderr='{}')",
                    metadata.package_name,
                    push_outcome.stderr.trim()
                );
            }
        }

        Ok(())
    }

    async fn try_enable_shell_obb_access(&self) {
        let commands = [
            vec![
                "adb".to_owned(),
                "-s".to_owned(),
                self.config.adb_serial.clone(),
                "shell".to_owned(),
                "cmd".to_owned(),
                "appops".to_owned(),
                "set".to_owned(),
                "com.android.shell".to_owned(),
                "MANAGE_EXTERNAL_STORAGE".to_owned(),
                "allow".to_owned(),
            ],
            vec![
                "adb".to_owned(),
                "-s".to_owned(),
                self.config.adb_serial.clone(),
                "shell".to_owned(),
                "appops".to_owned(),
                "set".to_owned(),
                "--uid".to_owned(),
                "com.android.shell".to_owned(),
                "MANAGE_EXTERNAL_STORAGE".to_owned(),
                "allow".to_owned(),
            ],
        ];

        for command in commands {
            let _ = self.runtime.exec(&self.config, command).await;
        }
    }

    async fn upload_apks(&self, apk_paths: &[PathBuf]) -> Result<Vec<String>> {
        let mut remote_paths = Vec::new();
        for (index, apk_path) in apk_paths.iter().enumerate() {
            eprintln!("uploading {}", apk_path.display());
            let remote_name = remote_name_for_apk(index, apk_path);
            remote_paths.push(
                self.runtime
                    .upload_file(
                        &self.config,
                        apk_path,
                        &self.config.remote_apk_dir,
                        &remote_name,
                    )
                    .await?,
            );
        }
        Ok(remote_paths)
    }

    async fn inspect_uploaded_apks(&self, remote_paths: &[String]) -> Result<ApkMetadata> {
        eprintln!("inspecting uploaded APK set");
        let mut primary: Option<ApkMetadata> = None;
        let mut native_abis = BTreeSet::new();

        for remote_path in remote_paths {
            let metadata = self
                .adb
                .inspect_apk(&self.runtime, &self.config, remote_path)
                .await?;
            native_abis.extend(metadata.native_abis.iter().cloned());

            let should_replace = match primary.as_ref() {
                None => true,
                Some(current) => {
                    current.launchable_activity.is_none() && metadata.launchable_activity.is_some()
                }
            };

            if should_replace {
                primary = Some(metadata);
            }
        }

        let mut metadata =
            primary.ok_or_else(|| anyhow::anyhow!("failed to inspect uploaded APK set"))?;
        metadata.native_abis = native_abis.into_iter().collect();
        Ok(metadata)
    }

    fn runtime_backend_name(&self) -> &'static str {
        if self.config.uses_host_runtime() {
            "host"
        } else {
            "docker"
        }
    }

    async fn collect_process_logs(&self) -> Result<Option<String>> {
        match &self.runtime {
            Runtime::Docker(docker) => {
                let mut stream = docker.client().logs(
                    &self.config.container_name,
                    Some(LogsOptions::<String> {
                        follow: false,
                        stdout: true,
                        stderr: true,
                        since: 0,
                        until: 0,
                        timestamps: true,
                        tail: "all".to_owned(),
                    }),
                );
                let mut output = String::new();
                while let Some(chunk) = stream.next().await {
                    output.push_str(&chunk?.to_string());
                }
                Ok(Some(output))
            }
            Runtime::Host(host) => {
                let log_path = host.log_path(&self.config);
                if !log_path.exists() {
                    return Ok(None);
                }
                Ok(Some(fs::read_to_string(log_path)?))
            }
        }
    }

    async fn collect_logcat_dump(&self) -> Result<Option<String>> {
        let outcome = self
            .runtime
            .exec(
                &self.config,
                vec![
                    "adb".to_owned(),
                    "-s".to_owned(),
                    self.config.adb_serial.clone(),
                    "logcat".to_owned(),
                    "-d".to_owned(),
                    "-v".to_owned(),
                    "time".to_owned(),
                ],
            )
            .await?;

        if outcome.exit_code != 0 {
            return Ok(None);
        }

        Ok(Some(outcome.stdout))
    }

    async fn collect_run_artifacts(&self) -> Result<RunArtifacts> {
        let process_logs = self.collect_process_logs().await?;
        let logcat_dump = self.collect_logcat_dump().await?;
        let anr_traces = self
            .capture_shell_file("if [ -f /data/anr/traces.txt ]; then cat /data/anr/traces.txt; fi")
            .await?;
        let tombstones = self
            .capture_shell_file(
                "if [ -d /data/tombstones ]; then for f in /data/tombstones/tombstone_*; do [ -f \"$f\" ] || continue; echo \"===== $f =====\"; cat \"$f\"; echo; done; fi",
            )
            .await?;

        Ok(RunArtifacts {
            crash_summary: logcat_dump
                .as_deref()
                .and_then(extract_logcat_crash_summary),
            anr_summary: logcat_dump.as_deref().and_then(extract_logcat_anr_summary),
            process_logs,
            logcat_dump,
            anr_traces,
            tombstones,
        })
    }

    async fn capture_shell_file(&self, script: &str) -> Result<Option<String>> {
        let outcome = self
            .runtime
            .exec(
                &self.config,
                vec![
                    "adb".to_owned(),
                    "-s".to_owned(),
                    self.config.adb_serial.clone(),
                    "shell".to_owned(),
                    "sh".to_owned(),
                    "-lc".to_owned(),
                    script.to_owned(),
                ],
            )
            .await?;

        if outcome.exit_code != 0 || outcome.stdout.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(outcome.stdout))
    }
}

fn print_bench_result(result: &BenchResult) {
    println!("runtime: {}", result.runtime_backend);
    println!("target: {}", result.adb_serial);
    println!("boot_ms: {}", result.boot_duration_ms);
    if let Some(install_duration_ms) = result.install_duration_ms {
        println!("install_ms: {}", install_duration_ms);
    }
    if let Some(launch_duration_ms) = result.launch_duration_ms {
        println!("launch_ms: {}", launch_duration_ms);
    }
    if let Some(package_name) = result.package_name.as_deref() {
        println!("package: {package_name}");
    }
    println!("total_ms: {}", result.total_duration_ms);
}

fn build_receipt_inputs(paths: &[PathBuf]) -> Result<Vec<ReceiptInput>> {
    paths.iter().map(|path| receipt_input(path)).collect()
}

fn receipt_input(path: &Path) -> Result<ReceiptInput> {
    let metadata = fs::metadata(path)?;
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(ReceiptInput {
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("input.apk")
            .to_owned(),
        sha256: format!("{:x}", hasher.finalize()),
        size_bytes: metadata.len(),
    })
}

fn write_optional_report(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn print_run_summary(summary: &RunSummary) {
    println!(
        "summary: backend={} status={} package={} boot_ms={} install_ms={} launch_ms={} total_ms={} kept_alive={}",
        summary.runtime_backend,
        summary.status,
        summary.package_name,
        summary.boot_duration_ms,
        summary.install_duration_ms,
        summary.launch_duration_ms,
        summary.total_duration_ms,
        summary.kept_alive
    );
    if let Some(crash_summary) = summary.crash_summary.as_deref() {
        println!("crash_summary: {crash_summary}");
    }
    if let Some(anr_summary) = summary.anr_summary.as_deref() {
        println!("anr_summary: {anr_summary}");
    }
}

fn print_apk_notes(metadata: &ApkMetadata, runtime_backend: &str, gps_disabled: bool) {
    let x86_ready = metadata
        .native_abis
        .iter()
        .any(|abi| abi.starts_with("x86"));
    let activity = metadata
        .launchable_activity
        .as_deref()
        .unwrap_or("<launcher not declared>");
    eprintln!(
        "package={} activity={} abis=[{}] x86_ready={} runtime_backend={} gps_disabled={}",
        metadata.package_name,
        activity,
        metadata.native_abis.join(","),
        x86_ready,
        runtime_backend,
        gps_disabled
    );
    if metadata.uses_arm_translation_on_x86_emulator() {
        eprintln!(
            "warning: APK ships ARM-only native libraries, so the x86_64 emulator must use ARM translation and may stay slower than a native x86/x86_64 build"
        );
    }
}

fn remote_name_for_apk(index: usize, apk_path: &Path) -> String {
    let file_name = apk_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app.apk");
    format!("{index:02}-{file_name}")
}

fn parse_failure_summary(message: &str) -> (Option<String>, Option<String>) {
    let lowercase = message.to_ascii_lowercase();
    if lowercase.contains("anr") {
        return (None, Some(message.to_owned()));
    }
    if lowercase.contains("crash detected") || lowercase.contains("fatal exception") {
        return (Some(message.to_owned()), None);
    }
    (None, None)
}

fn extract_logcat_crash_summary(logcat: &str) -> Option<String> {
    extract_logcat_line(logcat, &["FATAL EXCEPTION", "crash detected"])
}

fn extract_logcat_anr_summary(logcat: &str) -> Option<String> {
    extract_logcat_line(
        logcat,
        &[
            "ANR in",
            "Input dispatching timed out",
            "input dispatching timed out",
        ],
    )
}

fn extract_logcat_line(logcat: &str, needles: &[&str]) -> Option<String> {
    logcat
        .lines()
        .find(|line| needles.iter().any(|needle| line.contains(needle)))
        .map(str::trim)
        .map(str::to_owned)
}

fn write_run_artifacts(
    artifacts_dir: &Path,
    summary: &RunSummary,
    artifacts: &RunArtifacts,
) -> Result<()> {
    fs::create_dir_all(artifacts_dir)?;
    let reports_dir = artifacts_dir.join("reports");
    let logs_dir = artifacts_dir.join("logs");
    let forensics_dir = artifacts_dir.join("forensics");
    fs::create_dir_all(&reports_dir)?;
    fs::create_dir_all(&logs_dir)?;
    fs::create_dir_all(&forensics_dir)?;

    let summary_json = serde_json::to_string_pretty(summary)?;
    let report_html = build_html_report(summary);
    let junit_xml = build_junit_report(summary);
    let markdown_summary = build_markdown_summary(summary);

    for summary_path in [
        artifacts_dir.join("run-summary.json"),
        reports_dir.join("run-summary.json"),
    ] {
        fs::write(summary_path, &summary_json)?;
    }
    for report_path in [
        artifacts_dir.join("run-report.html"),
        reports_dir.join("run-report.html"),
    ] {
        fs::write(report_path, &report_html)?;
    }
    for junit_path in [
        artifacts_dir.join("junit.xml"),
        reports_dir.join("junit.xml"),
    ] {
        fs::write(junit_path, &junit_xml)?;
    }
    for markdown_path in [
        artifacts_dir.join("run-summary.md"),
        reports_dir.join("run-summary.md"),
    ] {
        fs::write(markdown_path, &markdown_summary)?;
    }

    if let Some(process_logs) = artifacts.process_logs.as_deref() {
        for path in [
            artifacts_dir.join("emulator-process.log"),
            logs_dir.join("emulator-process.log"),
        ] {
            fs::write(path, process_logs)?;
        }
    }
    if let Some(logcat_dump) = artifacts.logcat_dump.as_deref() {
        for path in [
            artifacts_dir.join("logcat.txt"),
            logs_dir.join("logcat.txt"),
        ] {
            fs::write(path, logcat_dump)?;
        }
    }
    if let Some(crash_summary) = artifacts.crash_summary.as_deref() {
        fs::write(forensics_dir.join("crash-summary.txt"), crash_summary)?;
    }
    if let Some(anr_summary) = artifacts.anr_summary.as_deref() {
        fs::write(forensics_dir.join("anr-summary.txt"), anr_summary)?;
    }
    if let Some(anr_traces) = artifacts.anr_traces.as_deref() {
        fs::write(forensics_dir.join("anr-traces.txt"), anr_traces)?;
    }
    if let Some(tombstones) = artifacts.tombstones.as_deref() {
        fs::write(forensics_dir.join("tombstones.txt"), tombstones)?;
    }
    Ok(())
}

fn build_junit_report(summary: &RunSummary) -> String {
    let duration_seconds = summary.total_duration_ms as f64 / 1000.0;
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"RustDroid APK receipt\" tests=\"1\" failures=\"0\" errors=\"0\" time=\"{duration_seconds:.3}\">\n  <properties>\n    <property name=\"schema_version\" value=\"{}\"/>\n    <property name=\"tool_version\" value=\"{}\"/>\n    <property name=\"runtime_backend\" value=\"{}\"/>\n    <property name=\"input_sha256\" value=\"{}\"/>\n  </properties>\n  <testcase classname=\"rustdroid.launch\" name=\"{}\" time=\"{duration_seconds:.3}\"/>\n</testsuite>\n",
        summary.schema_version,
        escape_xml(&summary.tool_version),
        escape_xml(&summary.runtime_backend),
        escape_xml(&input_digests(summary)),
        escape_xml(&summary.package_name),
    )
}

fn write_benchmark_artifacts(artifacts_dir: &Path, receipt: &BenchmarkReceipt) -> Result<()> {
    fs::create_dir_all(artifacts_dir)?;
    fs::write(
        artifacts_dir.join("bench-summary.json"),
        serde_json::to_string_pretty(receipt)?,
    )?;
    let input_digests = receipt
        .inputs
        .iter()
        .map(|input| input.sha256.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let markdown = format!(
        "## RustDroid benchmark receipt\n\n| Field | Value |\n| --- | --- |\n| Schema / tool | `{}` / `{}` |\n| Backend / profile | `{}` / `{}` |\n| Host / architecture | `{}` / `{}` |\n| Runner image | `{}` |\n| AVD / Android API | `{}` / `{}` |\n| Boot mode | `{}` |\n| Emulator CPU / RAM / GPU | {} / {} MB / `{}` |\n| Input SHA-256 | `{}` |\n| Boot / install / launch / total | {} ms / {} ms / {} ms / {} ms |\n",
        receipt.schema_version,
        receipt.tool_version,
        receipt.runtime_backend,
        receipt.profile.as_deref().unwrap_or("custom"),
        receipt.environment.host_os,
        receipt.environment.host_arch,
        receipt.environment.runner_image.as_deref().unwrap_or("not reported"),
        receipt.environment.avd_name.as_deref().unwrap_or("not reported"),
        receipt
            .environment
            .android_api_level
            .as_deref()
            .unwrap_or("not reported"),
        receipt.environment.boot_mode,
        receipt.environment.emulator_cpu_cores,
        receipt.environment.emulator_ram_mb,
        receipt.environment.emulator_gpu_mode,
        input_digests,
        receipt.boot_duration_ms,
        receipt.install_duration_ms.unwrap_or_default(),
        receipt.launch_duration_ms.unwrap_or_default(),
        receipt.total_duration_ms,
    );
    fs::write(artifacts_dir.join("bench-summary.md"), markdown)?;
    Ok(())
}

fn build_markdown_summary(summary: &RunSummary) -> String {
    let profile = summary.profile.as_deref().unwrap_or("custom");
    let api_level = summary.emulator.api_level.as_deref().unwrap_or("unknown");
    let activity = summary
        .launchable_activity
        .as_deref()
        .unwrap_or("not declared");
    format!(
        "## RustDroid APK receipt\n\n| Field | Value |\n| --- | --- |\n| Status | `{}` |\n| Package | `{}` |\n| Activity | `{}` |\n| Backend / profile | `{}` / `{}` |\n| Emulator API / serial | `{}` / `{}` |\n| Input SHA-256 | `{}` |\n| Boot / install / launch / total | {} ms / {} ms / {} ms / {} ms |\n| Classification | `{}` |\n\nArtifacts: `run-summary.json`, `run-report.html`, `junit.xml`, `run-summary.md`, plus available logs under `logs/`.\n",
        summary.status,
        summary.package_name,
        activity,
        summary.runtime_backend,
        profile,
        api_level,
        summary.emulator.adb_serial,
        input_digests(summary),
        summary.boot_duration_ms,
        summary.install_duration_ms,
        summary.launch_duration_ms,
        summary.total_duration_ms,
        summary.failure_classification,
    )
}

fn input_digests(summary: &RunSummary) -> String {
    summary
        .inputs
        .iter()
        .map(|input| input.sha256.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn build_html_report(summary: &RunSummary) -> String {
    let profile = summary.profile.as_deref().unwrap_or("custom");
    let avd_name = summary
        .emulator
        .avd_name
        .as_deref()
        .unwrap_or("not reported");
    let api_level = summary
        .emulator
        .api_level
        .as_deref()
        .unwrap_or("not reported");
    let activity = summary
        .launchable_activity
        .as_deref()
        .unwrap_or("not declared");
    let inputs = if summary.inputs.is_empty() {
        "<li>no input metadata recorded</li>".to_owned()
    } else {
        summary
            .inputs
            .iter()
            .map(|input| {
                format!(
                    "<li><code>{}</code> — SHA-256 <code>{}</code> — {} bytes</li>",
                    escape_xml(&input.file_name),
                    escape_xml(&input.sha256),
                    input.size_bytes,
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let artifact_links = summary.artifacts.as_ref().map_or_else(
        || "Artifacts were not requested for this run.".to_owned(),
        |artifacts| {
            format!(
                "<a href=\"{}\">JSON</a> · <a href=\"{}\">HTML</a> · <a href=\"{}\">JUnit</a> · <a href=\"{}\">Markdown</a>",
                escape_xml(&artifacts.json),
                escape_xml(&artifacts.html),
                escape_xml(&artifacts.junit),
                escape_xml(&artifacts.markdown_summary),
            )
        },
    );
    let abis = if summary.native_abis.is_empty() {
        "none".to_owned()
    } else {
        escape_xml(&summary.native_abis.join(", "))
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>RustDroid Run Receipt</title><style>body{{font-family:system-ui,sans-serif;margin:2rem;background:#f4f1ea;color:#111}}main{{max-width:900px;margin:0 auto;background:#fff;padding:2rem;border-radius:16px;box-shadow:0 20px 60px rgba(0,0,0,.08)}}h1{{margin-top:0}}dl{{display:grid;grid-template-columns:220px 1fr;gap:.75rem 1rem}}dt{{font-weight:700}}dd{{margin:0}}.badge{{display:inline-block;padding:.3rem .6rem;border-radius:999px;background:#111;color:#fff;font-size:.85rem}}.panel{{margin-top:1.5rem;padding:1rem 1.25rem;border-radius:12px;background:#f7f3ea}}code{{background:#f1ede4;padding:.1rem .35rem;border-radius:6px}}a{{color:#0b5fff}}</style></head><body><main><h1>RustDroid Run Receipt</h1><p><span class=\"badge\">{status}</span></p><dl><dt>Schema / tool version</dt><dd>{schema_version} / {tool_version}</dd><dt>Backend / profile</dt><dd>{backend} / {profile}</dd><dt>Package / activity</dt><dd>{package} / {activity}</dd><dt>AVD / API</dt><dd>{avd} / {api_level}</dd><dt>ADB serial</dt><dd>{serial}</dd><dt>Headless / GPU</dt><dd>{headless} / {gpu_mode}</dd><dt>Boot</dt><dd>{boot} ms</dd><dt>Install</dt><dd>{install} ms</dd><dt>Launch</dt><dd>{launch} ms</dd><dt>Total</dt><dd>{total} ms</dd><dt>ABIs</dt><dd>{abis}</dd><dt>x86 Ready</dt><dd>{x86_ready}</dd><dt>ARM Translation</dt><dd>{arm_translation}</dd><dt>GPS Disabled</dt><dd>{gps_disabled}</dd><dt>Kept Alive</dt><dd>{kept_alive}</dd><dt>Classification</dt><dd>{classification}</dd><dt>Crash</dt><dd>{crash}</dd><dt>ANR</dt><dd>{anr}</dd></dl><section class=\"panel\"><h2>Input digest</h2><ul>{inputs}</ul><p>Only file names and SHA-256 digests are recorded; local input paths are intentionally excluded.</p></section><section class=\"panel\"><h2>Artifacts</h2><p>{artifact_links}</p><p><code>reports/</code> mirrors the summary files. <code>logs/</code> contains emulator and logcat output. <code>forensics/</code> contains crash, ANR, tombstone, and trace captures when available.</p></section></main></body></html>",
        status = escape_xml(&summary.status),
        schema_version = summary.schema_version,
        tool_version = escape_xml(&summary.tool_version),
        backend = escape_xml(&summary.runtime_backend),
        profile = escape_xml(profile),
        package = escape_xml(&summary.package_name),
        activity = escape_xml(activity),
        avd = escape_xml(avd_name),
        api_level = escape_xml(api_level),
        serial = escape_xml(&summary.emulator.adb_serial),
        headless = summary.emulator.headless,
        gpu_mode = escape_xml(&summary.emulator.gpu_mode),
        boot = summary.boot_duration_ms,
        install = summary.install_duration_ms,
        launch = summary.launch_duration_ms,
        total = summary.total_duration_ms,
        abis = abis,
        x86_ready = summary.x86_ready,
        arm_translation = summary.uses_arm_translation,
        gps_disabled = summary.gps_disabled,
        kept_alive = summary.kept_alive,
        classification = escape_xml(&summary.failure_classification),
        crash = escape_xml(summary.crash_summary.as_deref().unwrap_or("none")),
        anr = escape_xml(summary.anr_summary.as_deref().unwrap_or("none")),
        inputs = inputs,
        artifact_links = artifact_links,
    )
}

fn resolve_watch_candidate(path: &Path) -> Result<Option<WatchToken>> {
    if path.is_file() {
        if !supported_watch_input(path) {
            bail!(
                "watch only supports .apk, .apks, or .xapk files (got '{}')",
                path.display()
            );
        }
        return Ok(Some(watch_token(path)?));
    }

    if !path.exists() {
        bail!("watch path not found: {}", path.display());
    }
    if !path.is_dir() {
        bail!(
            "watch path must be a file or directory (got '{}')",
            path.display()
        );
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let candidate = entry.path();
        if candidate.is_file() && supported_watch_input(&candidate) {
            candidates.push(watch_token(&candidate)?);
        }
    }

    candidates.sort_by(|left, right| {
        right
            .modified_at_ms
            .cmp(&left.modified_at_ms)
            .then(left.path.cmp(&right.path))
    });
    Ok(candidates.into_iter().next())
}

fn watch_token(path: &Path) -> Result<WatchToken> {
    let metadata = fs::metadata(path)?;
    let modified_at_ms = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_millis())
        .unwrap_or_default();

    Ok(WatchToken {
        path: path.to_path_buf(),
        modified_at_ms,
        size_bytes: metadata.len(),
    })
}

fn supported_watch_input(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("apk" | "apks" | "xapk")
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, thread::sleep, time::Duration};

    use tempfile::tempdir;

    use super::{
        build_html_report, extract_logcat_anr_summary, extract_logcat_crash_summary,
        parse_failure_summary, receipt_input, resolve_watch_candidate, write_benchmark_artifacts,
        write_run_artifacts, BenchmarkEnvironment, BenchmarkReceipt, ReceiptArtifacts,
        ReceiptEmulator, ReceiptInput, RunArtifacts, RunSummary,
    };

    fn sample_summary() -> RunSummary {
        RunSummary {
            schema_version: 1,
            tool_version: "0.2.0".to_owned(),
            status: "passed".to_owned(),
            failure_classification: "none".to_owned(),
            profile: Some("host-fast".to_owned()),
            runtime_backend: "host".to_owned(),
            emulator: ReceiptEmulator {
                adb_serial: "emulator-5554".to_owned(),
                avd_name: Some("test_avd".to_owned()),
                api_level: Some("35".to_owned()),
                device: "Pixel 5".to_owned(),
                headless: true,
                gpu_mode: "swiftshader_indirect".to_owned(),
            },
            package_name: "com.example.app".to_owned(),
            launchable_activity: Some("com.example.app.MainActivity".to_owned()),
            native_abis: vec!["x86_64".to_owned()],
            x86_ready: true,
            uses_arm_translation: false,
            gps_disabled: true,
            boot_duration_ms: 1000,
            install_duration_ms: 200,
            launch_duration_ms: 50,
            total_duration_ms: 1400,
            kept_alive: false,
            crash_summary: Some("fatal exception".to_owned()),
            anr_summary: Some("input dispatching timed out".to_owned()),
            inputs: vec![ReceiptInput {
                file_name: "app.apk".to_owned(),
                sha256: "0123456789abcdef".to_owned(),
                size_bytes: 42,
            }],
            artifacts: Some(ReceiptArtifacts {
                json: "run-summary.json".to_owned(),
                html: "run-report.html".to_owned(),
                junit: "junit.xml".to_owned(),
                markdown_summary: "run-summary.md".to_owned(),
                logcat: Some("logcat.txt".to_owned()),
                emulator_process_log: Some("emulator-process.log".to_owned()),
            }),
        }
    }

    fn sample_benchmark_receipt() -> BenchmarkReceipt {
        BenchmarkReceipt {
            schema_version: 1,
            tool_version: "0.2.0".to_owned(),
            status: "measured".to_owned(),
            runtime_backend: "host".to_owned(),
            profile: Some("host-fast".to_owned()),
            environment: BenchmarkEnvironment {
                host_os: "linux".to_owned(),
                host_arch: "x86_64".to_owned(),
                host_cpu_cores: Some(8),
                runner_image: Some("ubuntu22".to_owned()),
                avd_name: Some("test_avd".to_owned()),
                android_api_level: Some("35".to_owned()),
                boot_mode: "cold".to_owned(),
                emulator_cpu_cores: 4,
                emulator_ram_mb: 4096,
                emulator_gpu_mode: "swiftshader_indirect".to_owned(),
            },
            inputs: vec![ReceiptInput {
                file_name: "app.apk".to_owned(),
                sha256: "0123456789abcdef".to_owned(),
                size_bytes: 42,
            }],
            package_name: Some("com.example.app".to_owned()),
            boot_duration_ms: 1000,
            install_duration_ms: Some(200),
            launch_duration_ms: Some(50),
            total_duration_ms: 1400,
        }
    }

    #[test]
    fn failure_summary_classifies_crash_and_anr() {
        let (crash, anr) = parse_failure_summary("Fatal Exception in main thread");
        assert_eq!(crash.as_deref(), Some("Fatal Exception in main thread"));
        assert_eq!(anr, None);

        let (crash, anr) = parse_failure_summary("ANR detected in foreground process");
        assert_eq!(crash, None);
        assert_eq!(anr.as_deref(), Some("ANR detected in foreground process"));
    }

    #[test]
    fn write_run_artifacts_persists_summary_and_logs() {
        let dir = tempdir().expect("tempdir");
        let summary = sample_summary();

        write_run_artifacts(
            dir.path(),
            &summary,
            &RunArtifacts {
                process_logs: Some("process logs".to_owned()),
                logcat_dump: Some("logcat dump".to_owned()),
                crash_summary: Some("fatal exception".to_owned()),
                anr_summary: Some("input dispatching timed out".to_owned()),
                anr_traces: Some("trace data".to_owned()),
                tombstones: Some("tombstone data".to_owned()),
            },
        )
        .expect("artifacts should write");

        let summary_json =
            fs::read_to_string(dir.path().join("run-summary.json")).expect("summary json");
        assert!(summary_json.contains("\"package_name\": \"com.example.app\""));
        assert!(summary_json.contains("\"schema_version\": 1"));
        assert!(summary_json.contains("\"sha256\": \"0123456789abcdef\""));
        assert!(!summary_json.contains("apk_paths"));
        assert_eq!(
            fs::read_to_string(dir.path().join("emulator-process.log")).expect("process log"),
            "process logs"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("logcat.txt")).expect("logcat"),
            "logcat dump"
        );
        assert!(
            dir.path()
                .join("forensics")
                .join("crash-summary.txt")
                .is_file(),
            "expected crash summary forensics file to be written"
        );
        assert!(
            dir.path().join("logs").join("logcat.txt").is_file(),
            "expected nested logcat file to be written"
        );
        assert!(
            dir.path().join("run-report.html").is_file(),
            "expected html report to be written"
        );
        assert!(
            dir.path().join("junit.xml").is_file(),
            "expected JUnit report to be written"
        );
        assert!(
            dir.path().join("run-summary.md").is_file(),
            "expected Markdown summary to be written"
        );
        assert!(
            fs::read_to_string(dir.path().join("junit.xml"))
                .expect("JUnit report")
                .contains("RustDroid APK receipt"),
            "expected JUnit receipt contents"
        );
    }

    #[test]
    fn html_report_includes_core_summary_fields() {
        let report = build_html_report(&sample_summary());

        assert!(report.contains("RustDroid Run Receipt"));
        assert!(report.contains("com.example.app"));
        assert!(report.contains("x86_64"));
        assert!(report.contains("0123456789abcdef"));
        assert!(report.contains("fatal exception"));
        assert!(report.contains("input dispatching timed out"));
        assert!(report.contains("Artifacts"));
    }

    #[test]
    fn html_report_renders_empty_and_untrusted_abi_values_as_text() {
        let mut empty_abis = sample_summary();
        empty_abis.native_abis.clear();
        let empty_report = build_html_report(&empty_abis);
        assert!(empty_report.contains("<dt>ABIs</dt><dd>none</dd>"));
        assert!(!empty_report.contains("<none>"));

        let mut untrusted_abi = sample_summary();
        untrusted_abi.native_abis = vec!["x86_64<script>alert(1)</script>".to_owned()];
        let escaped_report = build_html_report(&untrusted_abi);
        assert!(escaped_report.contains("x86_64&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!escaped_report.contains("x86_64<script>"));
    }

    #[test]
    fn receipt_input_uses_a_digest_without_exposing_parent_paths() {
        let dir = tempdir().expect("tempdir");
        let apk_path = dir.path().join("private-build.apk");
        fs::write(&apk_path, b"abc").expect("fixture input");

        let input = receipt_input(&apk_path).expect("receipt input");

        assert_eq!(input.file_name, "private-build.apk");
        assert_eq!(
            input.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(input.size_bytes, 3);
        assert!(!input.file_name.contains(&dir.path().display().to_string()));
    }

    #[test]
    fn benchmark_artifacts_include_reproducible_environment_fields() {
        let dir = tempdir().expect("tempdir");

        write_benchmark_artifacts(dir.path(), &sample_benchmark_receipt())
            .expect("benchmark artifacts should write");

        let json =
            fs::read_to_string(dir.path().join("bench-summary.json")).expect("benchmark JSON");
        assert!(json.contains("\"boot_mode\": \"cold\""));
        assert!(json.contains("\"sha256\": \"0123456789abcdef\""));
        assert!(dir.path().join("bench-summary.md").is_file());
    }

    #[test]
    fn logcat_extractors_find_crash_and_anr_lines() {
        let logcat = "\
04-01 12:00:00.000 E/AndroidRuntime(123): FATAL EXCEPTION: main\n\
04-01 12:00:01.000 E/ActivityManager(456): ANR in com.example.app";

        assert_eq!(
            extract_logcat_crash_summary(logcat).as_deref(),
            Some("04-01 12:00:00.000 E/AndroidRuntime(123): FATAL EXCEPTION: main")
        );
        assert_eq!(
            extract_logcat_anr_summary(logcat).as_deref(),
            Some("04-01 12:00:01.000 E/ActivityManager(456): ANR in com.example.app")
        );
    }

    #[test]
    fn watch_candidate_uses_direct_file_input() {
        let dir = tempdir().expect("tempdir");
        let apk_path = dir.path().join("app.apk");
        fs::write(&apk_path, b"apk").expect("apk");

        let candidate = resolve_watch_candidate(&apk_path)
            .expect("watch candidate")
            .expect("candidate should exist");
        assert_eq!(candidate.path, apk_path);
    }

    #[test]
    fn watch_candidate_prefers_latest_supported_file_in_directory() {
        let dir = tempdir().expect("tempdir");
        let older = dir.path().join("older.apk");
        let newer = dir.path().join("newer.xapk");
        let ignored = dir.path().join("ignored.txt");

        fs::write(&older, b"older").expect("older");
        sleep(Duration::from_millis(20));
        fs::write(&ignored, b"ignored").expect("ignored");
        sleep(Duration::from_millis(20));
        fs::write(&newer, b"newer").expect("newer");

        let candidate = resolve_watch_candidate(dir.path())
            .expect("watch directory")
            .expect("candidate should exist");
        assert_eq!(candidate.path, newer);
    }
}
