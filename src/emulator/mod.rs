use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Result};
use bollard::container::LogsOptions;
use futures_util::StreamExt;
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
        self.require_apks(&args.apks)?;
        self.start_device(true, false).await?;
        self.install_uploaded_apks(&args.apks, args.replace).await?;
        Ok(())
    }

    pub async fn launch(&self, args: LaunchArgs) -> Result<()> {
        self.start_device(true, true).await?;

        if let Some(apk) = args.apk.as_ref() {
            self.require_apk(apk)?;
            let remote_paths = self.upload_apks(std::slice::from_ref(apk)).await?;
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
            let process_logs = self.collect_process_logs().await?;
            let logcat_dump = self.collect_logcat_dump().await?;
            write_run_artifacts(
                artifacts_dir,
                &summary,
                process_logs.as_deref(),
                logcat_dump.as_deref(),
            )?;
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
                .install_uploaded_apks(std::slice::from_ref(apk), args.replace)
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

fn write_run_artifacts(
    artifacts_dir: &Path,
    summary: &RunSummary,
    process_logs: Option<&str>,
    logcat_dump: Option<&str>,
) -> Result<()> {
    fs::create_dir_all(artifacts_dir)?;
    let summary_path = artifacts_dir.join("run-summary.json");
    let summary_json = serde_json::to_string_pretty(summary)?;
    fs::write(summary_path, summary_json)?;
    fs::write(
        artifacts_dir.join("run-report.html"),
        build_html_report(summary),
    )?;
    if let Some(process_logs) = process_logs {
        fs::write(artifacts_dir.join("emulator-process.log"), process_logs)?;
    }
    if let Some(logcat_dump) = logcat_dump {
        fs::write(artifacts_dir.join("logcat.txt"), logcat_dump)?;
    }
    Ok(())
}

fn build_html_report(summary: &RunSummary) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>RustDroid Run Report</title><style>body{{font-family:system-ui,sans-serif;margin:2rem;background:#f4f1ea;color:#111}}main{{max-width:840px;margin:0 auto;background:#fff;padding:2rem;border-radius:16px;box-shadow:0 20px 60px rgba(0,0,0,.08)}}h1{{margin-top:0}}dl{{display:grid;grid-template-columns:220px 1fr;gap:.75rem 1rem}}dt{{font-weight:700}}dd{{margin:0}}.badge{{display:inline-block;padding:.3rem .6rem;border-radius:999px;background:#111;color:#fff;font-size:.85rem}}.warn{{background:#9b2c2c}}</style></head><body><main><h1>RustDroid Run Report</h1><p><span class=\"badge\">{backend}</span></p><dl><dt>Package</dt><dd>{package}</dd><dt>ADB Serial</dt><dd>{serial}</dd><dt>Boot</dt><dd>{boot} ms</dd><dt>Install</dt><dd>{install} ms</dd><dt>Launch</dt><dd>{launch} ms</dd><dt>Total</dt><dd>{total} ms</dd><dt>ABIs</dt><dd>{abis}</dd><dt>x86 Ready</dt><dd>{x86_ready}</dd><dt>ARM Translation</dt><dd>{arm_translation}</dd><dt>GPS Disabled</dt><dd>{gps_disabled}</dd><dt>Kept Alive</dt><dd>{kept_alive}</dd><dt>Crash</dt><dd>{crash}</dd><dt>ANR</dt><dd>{anr}</dd></dl></main></body></html>",
        backend = summary.runtime_backend,
        package = summary.package_name,
        serial = summary.adb_serial,
        boot = summary.boot_duration_ms,
        install = summary.install_duration_ms,
        launch = summary.launch_duration_ms,
        total = summary.total_duration_ms,
        abis = if summary.native_abis.is_empty() {
            "<none>".to_owned()
        } else {
            summary.native_abis.join(", ")
        },
        x86_ready = summary.x86_ready,
        arm_translation = summary.uses_arm_translation,
        gps_disabled = summary.gps_disabled,
        kept_alive = summary.kept_alive,
        crash = summary.crash_summary.as_deref().unwrap_or("none"),
        anr = summary.anr_summary.as_deref().unwrap_or("none"),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{build_html_report, parse_failure_summary, write_run_artifacts, RunSummary};

    fn sample_summary() -> RunSummary {
        RunSummary {
            runtime_backend: "host".to_owned(),
            container_name: "rustdroid-emulator".to_owned(),
            adb_serial: "emulator-5554".to_owned(),
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
            apk_paths: vec!["app.apk".to_owned()],
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
            Some("process logs"),
            Some("logcat dump"),
        )
        .expect("artifacts should write");

        let summary_json =
            fs::read_to_string(dir.path().join("run-summary.json")).expect("summary json");
        assert!(summary_json.contains("\"package_name\": \"com.example.app\""));
        assert_eq!(
            fs::read_to_string(dir.path().join("emulator-process.log")).expect("process log"),
            "process logs"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("logcat.txt")).expect("logcat"),
            "logcat dump"
        );
        assert!(
            dir.path().join("run-report.html").is_file(),
            "expected html report to be written"
        );
    }

    #[test]
    fn html_report_includes_core_summary_fields() {
        let report = build_html_report(&sample_summary());

        assert!(report.contains("RustDroid Run Report"));
        assert!(report.contains("com.example.app"));
        assert!(report.contains("x86_64"));
        assert!(report.contains("fatal exception"));
        assert!(report.contains("input dispatching timed out"));
    }
}
