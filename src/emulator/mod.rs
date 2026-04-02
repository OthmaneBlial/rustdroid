use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Result};
use serde::Serialize;

use crate::{
    adb::{AdbClient, ApkMetadata},
    cli::{BenchArgs, InstallArgs, LaunchArgs, LogsArgs, OpenArgs, RunArgs, StartArgs, StopArgs},
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
    pub apk_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunSummary {
    pub runtime_backend: String,
    pub container_name: String,
    pub adb_serial: String,
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
    pub apk_paths: Vec<String>,
}

#[derive(Debug, Clone)]
struct InstallOutcome {
    metadata: ApkMetadata,
}

impl EmulatorOrchestrator {
    pub fn new(config: RuntimeConfig, runtime: Runtime) -> Self {
        let adb = AdbClient::new(
            config.adb_serial.clone(),
            config.disable_animations,
            config.optimize_android_runtime,
            config.device_width_px,
            config.device_height_px,
            config.device_density_dpi,
            config.compile_installed_package,
            config.disable_preinstalled_packages,
            config.disable_google_play_services,
        );
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
        self.require_apks(&args.apks)?;
        self.start_device(true, false).await?;
        self.install_uploaded_apks(&args.apks, args.replace).await?;
        Ok(())
    }

    pub async fn launch(&self, args: LaunchArgs) -> Result<()> {
        self.start_device(true, true).await?;

        if let Some(apk) = args.apk.as_ref() {
            self.require_apk(apk)?;
            let remote_paths = self.upload_apks(&[apk.clone()]).await?;
            let metadata = self.inspect_uploaded_apks(&remote_paths).await?;
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

        bail!("launch requires either an APK path or --package <name>");
    }

    pub async fn run(&self, args: RunArgs) -> Result<()> {
        self.require_apks(&args.apks)?;
        let total_started = Instant::now();

        eprintln!("==> starting emulator on {}", self.runtime_backend_name());
        let boot_started = Instant::now();
        self.start_device(true, true).await?;
        let boot_duration_ms = boot_started.elapsed().as_millis();

        eprintln!("==> installing package set");
        let install_started = Instant::now();
        let install = self.install_uploaded_apks(&args.apks, args.replace).await?;
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
        let (crash_summary, anr_summary) = stream_result
            .as_ref()
            .err()
            .map(|error| parse_failure_summary(&error.to_string()))
            .unwrap_or((None, None));

        let summary = RunSummary {
            runtime_backend: self.runtime_backend_name().to_owned(),
            container_name: self.config.container_name.clone(),
            adb_serial: self.config.adb_serial.clone(),
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
            apk_paths: args
                .apks
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        };

        if let Some(artifacts_dir) = args.artifacts_dir.as_ref() {
            write_run_artifacts(artifacts_dir, &summary)?;
        }

        print_run_summary(&summary);

        if !args.keep_alive {
            eprintln!("==> stopping runtime because --keep-alive=false");
            self.runtime.stop(&self.config, 15).await?;
        }

        stream_result
    }

    pub async fn bench(&self, args: BenchArgs, json: bool) -> Result<()> {
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
            apk_paths: args
                .apk
                .as_ref()
                .map(|path| vec![path.display().to_string()])
                .unwrap_or_default(),
        };

        if let Some(apk) = args.apk.as_ref() {
            self.require_apk(apk)?;

            eprintln!("==> install benchmark");
            let install_started = Instant::now();
            let install = self
                .install_uploaded_apks(&[apk.clone()], args.replace)
                .await?;
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

    async fn install_uploaded_apks(
        &self,
        apk_paths: &[PathBuf],
        replace: bool,
    ) -> Result<InstallOutcome> {
        let remote_paths = self.upload_apks(apk_paths).await?;
        let metadata = self.inspect_uploaded_apks(&remote_paths).await?;
        print_apk_notes(
            &metadata,
            self.runtime_backend_name(),
            !self.config.emulator_enable_gps,
        );
        self.adb
            .install_apks(&self.runtime, &self.config, &remote_paths, replace)
            .await?;
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

    fn require_apk(&self, apk_path: &Path) -> Result<()> {
        anyhow::ensure!(apk_path.is_file(), "APK not found: {}", apk_path.display());
        Ok(())
    }

    fn require_apks(&self, apk_paths: &[PathBuf]) -> Result<()> {
        anyhow::ensure!(!apk_paths.is_empty(), "at least one APK path is required");
        for apk_path in apk_paths {
            self.require_apk(apk_path)?;
        }
        Ok(())
    }

    fn runtime_backend_name(&self) -> &'static str {
        if self.config.uses_host_runtime() {
            "host"
        } else {
            "docker"
        }
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

fn print_run_summary(summary: &RunSummary) {
    println!(
        "summary: backend={} package={} boot_ms={} install_ms={} launch_ms={} total_ms={} kept_alive={}",
        summary.runtime_backend,
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

fn write_run_artifacts(artifacts_dir: &Path, summary: &RunSummary) -> Result<()> {
    fs::create_dir_all(artifacts_dir)?;
    let summary_path = artifacts_dir.join("run-summary.json");
    let summary_json = serde_json::to_string_pretty(summary)?;
    fs::write(summary_path, summary_json)?;
    Ok(())
}
