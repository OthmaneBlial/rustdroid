use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use tokio::time::sleep;

use crate::{
    config::RuntimeConfig,
    host::{managed_log_path, managed_process_running},
    runtime::Runtime,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApkMetadata {
    pub package_name: String,
    pub launchable_activity: Option<String>,
    pub native_abis: Vec<String>,
}

impl ApkMetadata {
    pub fn uses_arm_translation_on_x86_emulator(&self) -> bool {
        !self.native_abis.is_empty()
            && self.native_abis.iter().all(|abi| !abi.starts_with("x86"))
            && self.native_abis.iter().any(|abi| abi.starts_with("arm"))
    }
}

const PREINSTALLED_PACKAGES_TO_DISABLE: &[&str] = &[
    "com.android.vending",
    "com.google.android.googlequicksearchbox",
    "com.android.chrome",
    "com.google.android.apps.wellbeing",
    "com.google.android.gm",
    "com.google.android.dialer",
    "com.google.android.contacts",
    "com.google.android.apps.messaging",
    "com.google.android.calendar",
    "com.google.android.apps.photos",
];

const GOOGLE_PLAY_PACKAGES_TO_DISABLE: &[&str] = &[
    "com.google.android.gms",
    "com.google.android.gms.supervision",
];

#[derive(Debug, Clone)]
pub struct AdbClient {
    serial: String,
    disable_animations: bool,
    optimize_android_runtime: bool,
    device_width_px: u16,
    device_height_px: u16,
    device_density_dpi: u16,
    compile_installed_package: bool,
    disable_preinstalled_packages: bool,
    disable_google_play_services: bool,
}

impl AdbClient {
    pub fn new(
        serial: impl Into<String>,
        disable_animations: bool,
        optimize_android_runtime: bool,
        device_width_px: u16,
        device_height_px: u16,
        device_density_dpi: u16,
        compile_installed_package: bool,
        disable_preinstalled_packages: bool,
        disable_google_play_services: bool,
    ) -> Self {
        Self {
            serial: serial.into(),
            disable_animations,
            optimize_android_runtime,
            device_width_px,
            device_height_px,
            device_density_dpi,
            compile_installed_package,
            disable_preinstalled_packages,
            disable_google_play_services,
        }
    }

    pub async fn wait_for_boot(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);

        loop {
            let outcome = runtime
                .exec(
                    config,
                    self.adb_command([
                        "shell",
                        "sh",
                        "-lc",
                        "echo sys=$(getprop sys.boot_completed); echo dev=$(getprop dev.bootcomplete); echo anim=$(getprop init.svc.bootanim)",
                    ]),
                )
                .await?;

            let current_stdout = outcome.stdout.trim().to_owned();
            let current_stderr = outcome.stderr.trim().to_owned();
            let mut sys_boot_completed = "";
            let mut dev_bootcomplete = "";
            let mut bootanim_status = "";

            for part in current_stdout.split_whitespace() {
                if let Some(value) = part.strip_prefix("sys=") {
                    sys_boot_completed = value;
                } else if let Some(value) = part.strip_prefix("dev=") {
                    dev_bootcomplete = value;
                } else if let Some(value) = part.strip_prefix("anim=") {
                    bootanim_status = value;
                }
            }

            if outcome.exit_code == 0
                && (sys_boot_completed == "1"
                    || dev_bootcomplete == "1"
                    || bootanim_status.eq_ignore_ascii_case("stopped"))
            {
                let package_manager = runtime
                    .exec(config, self.adb_command(["shell", "pm", "path", "android"]))
                    .await?;
                let storage_manager = runtime
                    .exec(
                        config,
                        self.adb_command(["shell", "service", "check", "mount"]),
                    )
                    .await?;
                let window_manager = runtime
                    .exec(
                        config,
                        self.adb_command(["shell", "service", "check", "window"]),
                    )
                    .await?;
                let network_routes = runtime
                    .exec(config, self.adb_command(["shell", "ip", "route"]))
                    .await?;
                let gateway_ping = runtime
                    .exec(
                        config,
                        self.adb_command(["shell", "ping", "-c", "1", "-W", "1", "10.0.2.2"]),
                    )
                    .await?;

                let services_ready = package_manager.exit_code == 0
                    && package_manager.stdout.contains("package:")
                    && storage_manager.exit_code == 0
                    && storage_manager.stdout.contains("Service mount: found")
                    && window_manager.exit_code == 0
                    && window_manager.stdout.contains("Service window: found");
                let network_ready =
                    !network_routes.stdout.trim().is_empty() && gateway_ping.exit_code == 0;

                if services_ready && network_ready {
                    return Ok(());
                }

                if services_ready {
                    let _ = runtime
                        .exec(config, self.adb_command(["shell", "svc", "wifi", "enable"]))
                        .await;
                    let _ = runtime
                        .exec(config, self.adb_command(["shell", "svc", "data", "enable"]))
                        .await;
                }
            }

            if config.uses_host_runtime() {
                if let Some(false) = managed_process_running(config)? {
                    let log_tail = tail_file(&managed_log_path(config), 20);
                    bail!(
                        "host emulator exited before adb became ready{}",
                        log_tail
                            .as_deref()
                            .map(|tail| format!(" (recent log tail='{}')", tail))
                            .unwrap_or_default()
                    );
                }
            }

            if Instant::now() >= deadline {
                bail!(
                    "emulator boot timed out after {}s (stdout='{}', stderr='{}')",
                    timeout_secs,
                    current_stdout,
                    current_stderr
                );
            }

            sleep(Duration::from_secs(poll_interval_secs)).await;
        }
    }

    pub async fn install_apks(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        remote_paths: &[String],
        replace: bool,
    ) -> Result<()> {
        anyhow::ensure!(
            !remote_paths.is_empty(),
            "install requires at least one APK path"
        );

        let mut command = if remote_paths.len() == 1 {
            self.adb_command(["install"])
        } else {
            self.adb_command(["install-multiple"])
        };
        if replace {
            command.push("-r".to_owned());
        }
        command.push("-t".to_owned());
        command.extend(remote_paths.iter().cloned());

        let mut last_outcome = None;
        for attempt in 0..3 {
            self.stabilize_device(runtime, config).await?;
            let outcome = runtime.exec(config, command.clone()).await?;
            if outcome.exit_code == 0 {
                return Ok(());
            }

            let combined = format!("{}\n{}", outcome.stdout, outcome.stderr);
            let transient = combined.contains("Can't find service: package")
                || combined.contains("PackageManagerInternal.freeStorage")
                || combined.contains("StorageManagerService.allocateBytes")
                || combined.contains("NullPointerException");
            last_outcome = Some(outcome);

            if !transient || attempt == 2 {
                break;
            }

            sleep(Duration::from_secs(5)).await;
        }

        let outcome = last_outcome.expect("install outcome should be recorded");
        ensure_command_success(
            "install APK",
            &outcome.stdout,
            &outcome.stderr,
            outcome.exit_code,
        )
    }

    pub async fn compile_package(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        package_name: &str,
    ) -> Result<()> {
        if !self.compile_installed_package {
            return Ok(());
        }

        let outcome = runtime
            .exec(
                config,
                self.adb_command([
                    "shell",
                    "cmd",
                    "package",
                    "compile",
                    "-m",
                    "speed",
                    "-f",
                    package_name,
                ]),
            )
            .await?;

        ensure_command_success(
            "compile installed package",
            &outcome.stdout,
            &outcome.stderr,
            outcome.exit_code,
        )
    }

    pub async fn inspect_apk(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        remote_path: &str,
    ) -> Result<ApkMetadata> {
        let badging = runtime
            .exec(
                config,
                vec![
                    "aapt".to_owned(),
                    "dump".to_owned(),
                    "badging".to_owned(),
                    remote_path.to_owned(),
                ],
            )
            .await?;

        if badging.exit_code == 0 {
            return parse_badging(&badging.stdout);
        }

        let fallback = runtime
            .exec(
                config,
                vec![
                    "apkanalyzer".to_owned(),
                    "manifest".to_owned(),
                    "application-id".to_owned(),
                    remote_path.to_owned(),
                ],
            )
            .await?;

        if fallback.exit_code == 0 {
            let package_name = fallback.stdout.trim();
            if !package_name.is_empty() {
                return Ok(ApkMetadata {
                    package_name: package_name.to_owned(),
                    launchable_activity: None,
                    native_abis: Vec::new(),
                });
            }
        }

        bail!(
            "failed to inspect APK metadata (aapt stderr='{}', apkanalyzer stderr='{}')",
            badging.stderr.trim(),
            fallback.stderr.trim()
        )
    }

    pub async fn launch_app(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        metadata: &ApkMetadata,
    ) -> Result<()> {
        self.launch_package(
            runtime,
            config,
            &metadata.package_name,
            metadata.launchable_activity.as_deref(),
        )
        .await
    }

    pub async fn launch_package(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        package_name: &str,
        activity: Option<&str>,
    ) -> Result<()> {
        self.stabilize_device(runtime, config).await?;
        self.force_stop_package(runtime, config, package_name)
            .await?;

        if let Some(activity) = activity {
            let outcome = runtime
                .exec(
                    config,
                    self.adb_command([
                        "shell",
                        "am",
                        "start",
                        "-W",
                        "-S",
                        "-n",
                        &format!("{package_name}/{activity}"),
                    ]),
                )
                .await?;

            let combined = format!("{}\n{}", outcome.stdout, outcome.stderr);
            if outcome.exit_code == 0 && !looks_like_failed_activity_launch(&combined) {
                return self
                    .wait_for_package_foreground(runtime, config, package_name)
                    .await;
            }
        }

        let outcome = runtime
            .exec(
                config,
                self.adb_command([
                    "shell",
                    "monkey",
                    "-p",
                    package_name,
                    "-c",
                    "android.intent.category.LAUNCHER",
                    "1",
                ]),
            )
            .await?;

        ensure_command_success(
            "launch app",
            &outcome.stdout,
            &outcome.stderr,
            outcome.exit_code,
        )?;
        self.wait_for_package_foreground(runtime, config, package_name)
            .await
    }

    fn adb_command<const N: usize>(&self, command: [&str; N]) -> Vec<String> {
        let mut args = vec!["adb".to_owned(), "-s".to_owned(), self.serial.clone()];
        args.extend(command.into_iter().map(str::to_owned));
        args
    }

    pub async fn stabilize_device(&self, runtime: &Runtime, config: &RuntimeConfig) -> Result<()> {
        for attempt in 0..6 {
            let focus = self.current_focus(runtime, config).await?;
            let lowercase = focus.to_ascii_lowercase();

            if lowercase.contains("application not responding: com.android.systemui")
                || lowercase.contains("not responding: com.android.systemui")
                || lowercase.contains("system ui isn't responding")
            {
                if attempt >= 3 {
                    let _ = self.restart_system_ui(runtime, config).await;
                }
                self.choose_wait_on_anr_dialog(runtime, config).await?;
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            if lowercase.contains("application not responding:")
                || lowercase.contains("keeps stopping")
                || lowercase.contains("isn't responding")
            {
                self.choose_wait_on_anr_dialog(runtime, config).await?;
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            break;
        }

        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "input", "keyevent", "KEYCODE_HOME"]),
            )
            .await;
        self.configure_runtime_performance(runtime, config).await?;
        Ok(())
    }

    async fn current_focus(&self, runtime: &Runtime, config: &RuntimeConfig) -> Result<String> {
        let outcome = runtime
            .exec(
                config,
                vec![
                    "sh".to_owned(),
                    "-lc".to_owned(),
                    format!(
                        "adb -s {} shell dumpsys window | grep -i mCurrentFocus || true",
                        self.serial
                    ),
                ],
            )
            .await?;
        Ok(outcome.stdout)
    }

    async fn force_stop_package(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        package_name: &str,
    ) -> Result<()> {
        let outcome = runtime
            .exec(
                config,
                self.adb_command(["shell", "am", "force-stop", package_name]),
            )
            .await?;
        ensure_command_success(
            "force-stop package",
            &outcome.stdout,
            &outcome.stderr,
            outcome.exit_code,
        )
    }

    async fn disable_package_for_user(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        package_name: &str,
    ) {
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "pm", "disable-user", "--user", "0", package_name]),
            )
            .await;
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "am", "force-stop", package_name]),
            )
            .await;
    }

    async fn configure_runtime_performance(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
    ) -> Result<()> {
        if self.disable_animations {
            let commands = [
                [
                    "shell",
                    "settings",
                    "put",
                    "global",
                    "window_animation_scale",
                    "0",
                ],
                [
                    "shell",
                    "settings",
                    "put",
                    "global",
                    "transition_animation_scale",
                    "0",
                ],
                [
                    "shell",
                    "settings",
                    "put",
                    "global",
                    "animator_duration_scale",
                    "0",
                ],
            ];

            for command in commands {
                let _ = runtime.exec(config, self.adb_command(command)).await;
            }
        }

        if !self.optimize_android_runtime {
            return Ok(());
        }

        let runtime_commands = [
            [
                "shell",
                "settings",
                "put",
                "global",
                "package_verifier_enable",
                "0",
            ],
            [
                "shell",
                "settings",
                "put",
                "global",
                "verifier_verify_adb_installs",
                "0",
            ],
            [
                "shell",
                "settings",
                "put",
                "global",
                "device_provisioned",
                "1",
            ],
            [
                "shell",
                "settings",
                "put",
                "secure",
                "user_setup_complete",
                "1",
            ],
            [
                "shell",
                "settings",
                "put",
                "global",
                "captive_portal_mode",
                "0",
            ],
            [
                "shell",
                "settings",
                "put",
                "global",
                "wifi_scan_always_enabled",
                "0",
            ],
            [
                "shell",
                "settings",
                "put",
                "global",
                "ble_scan_always_enabled",
                "0",
            ],
            ["shell", "settings", "put", "secure", "location_mode", "0"],
        ];

        for command in runtime_commands {
            let _ = runtime.exec(config, self.adb_command(command)).await;
        }

        let size = format!("{}x{}", self.device_width_px, self.device_height_px);
        let density = self.device_density_dpi.to_string();
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "wm", "size", size.as_str()]),
            )
            .await;
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "wm", "density", density.as_str()]),
            )
            .await;

        let stay_on_commands = [
            [
                "shell",
                "settings",
                "put",
                "global",
                "stay_on_while_plugged_in",
                "7",
            ],
            [
                "shell",
                "settings",
                "put",
                "system",
                "screen_off_timeout",
                "2147483647",
            ],
        ];

        for command in stay_on_commands {
            let _ = runtime.exec(config, self.adb_command(command)).await;
        }

        if self.disable_preinstalled_packages {
            for package_name in PREINSTALLED_PACKAGES_TO_DISABLE {
                self.disable_package_for_user(runtime, config, package_name)
                    .await;
            }
        }

        if self.disable_google_play_services {
            for package_name in GOOGLE_PLAY_PACKAGES_TO_DISABLE {
                self.disable_package_for_user(runtime, config, package_name)
                    .await;
            }
        }

        self.unlock_device(runtime, config).await?;
        sleep(Duration::from_millis(750)).await;
        Ok(())
    }

    async fn package_pid(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        package_name: &str,
    ) -> Result<Option<String>> {
        let outcome = runtime
            .exec(config, self.adb_command(["shell", "pidof", package_name]))
            .await?;
        let pid = outcome.stdout.trim();
        if pid.is_empty() {
            Ok(None)
        } else {
            Ok(Some(pid.to_owned()))
        }
    }

    async fn unlock_device(&self, runtime: &Runtime, config: &RuntimeConfig) -> Result<()> {
        let center_x = (self.device_width_px / 2).max(1).to_string();
        let start_y = (self
            .device_height_px
            .saturating_sub(self.device_height_px / 8))
        .max(1)
        .to_string();
        let end_y = (self.device_height_px / 4).max(1).to_string();

        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "input", "keyevent", "KEYCODE_WAKEUP"]),
            )
            .await;
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "wm", "dismiss-keyguard"]),
            )
            .await;
        let _ = runtime
            .exec(
                config,
                self.adb_command([
                    "shell",
                    "input",
                    "swipe",
                    center_x.as_str(),
                    start_y.as_str(),
                    center_x.as_str(),
                    end_y.as_str(),
                    "200",
                ]),
            )
            .await;
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "input", "keyevent", "82"]),
            )
            .await;
        Ok(())
    }

    async fn wait_for_package_foreground(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
        package_name: &str,
    ) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(45);
        let package_name_lower = package_name.to_ascii_lowercase();

        loop {
            let focus = self.current_focus(runtime, config).await?;
            let lowercase = focus.to_ascii_lowercase();

            if lowercase.contains(&package_name_lower) {
                return Ok(());
            }

            if lowercase.contains("application not responding:")
                || lowercase.contains("keeps stopping")
                || lowercase.contains("isn't responding")
            {
                self.choose_wait_on_anr_dialog(runtime, config).await?;
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            if lowercase.contains("notificationshade") || lowercase.contains("keyguard") {
                self.unlock_device(runtime, config).await?;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            if focus.trim().is_empty()
                && self
                    .package_pid(runtime, config, package_name)
                    .await?
                    .is_some()
            {
                self.unlock_device(runtime, config).await?;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            if Instant::now() >= deadline {
                bail!(
                    "launch timed out before '{}' reached the foreground (last focus='{}')",
                    package_name,
                    focus.trim()
                );
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn choose_wait_on_anr_dialog(
        &self,
        runtime: &Runtime,
        config: &RuntimeConfig,
    ) -> Result<()> {
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "input", "keyevent", "KEYCODE_DPAD_DOWN"]),
            )
            .await;
        let _ = runtime
            .exec(
                config,
                self.adb_command(["shell", "input", "keyevent", "KEYCODE_ENTER"]),
            )
            .await;
        Ok(())
    }

    async fn restart_system_ui(&self, runtime: &Runtime, config: &RuntimeConfig) -> Result<()> {
        let _ = runtime
            .exec(
                config,
                self.adb_command([
                    "shell",
                    "su",
                    "root",
                    "sh",
                    "-c",
                    "kill $(pidof com.android.systemui) || true",
                ]),
            )
            .await;
        Ok(())
    }
}

fn tail_file(path: &Path, line_count: usize) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let lines: Vec<_> = contents.lines().rev().take(line_count).collect();
    if lines.is_empty() {
        return None;
    }

    Some(
        lines
            .into_iter()
            .rev()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join(" | "),
    )
}

fn ensure_command_success(action: &str, stdout: &str, stderr: &str, exit_code: i64) -> Result<()> {
    if exit_code == 0 {
        return Ok(());
    }

    bail!(
        "{} failed with exit code {} (stdout='{}', stderr='{}')",
        action,
        exit_code,
        stdout.trim(),
        stderr.trim()
    )
}

fn parse_badging(output: &str) -> Result<ApkMetadata> {
    let mut package_name = None;
    let mut launchable_activity = None;
    let mut native_abis = Vec::new();

    for line in output.lines() {
        if package_name.is_none() && line.starts_with("package: ") {
            package_name = quoted_field(line, "name");
        }

        if launchable_activity.is_none() && line.starts_with("launchable-activity: ") {
            launchable_activity = quoted_field(line, "name");
        }

        if native_abis.is_empty() && line.starts_with("native-code:") {
            native_abis = quoted_values(line);
        }
    }

    let package_name =
        package_name.ok_or_else(|| anyhow::anyhow!("missing package name in APK"))?;
    Ok(ApkMetadata {
        package_name,
        launchable_activity,
        native_abis,
    })
}

fn quoted_field(line: &str, field: &str) -> Option<String> {
    let needle = format!("{field}='");
    let start = line.find(&needle)? + needle.len();
    let remainder = &line[start..];
    let end = remainder.find('\'')?;
    Some(remainder[..end].to_owned())
}

fn quoted_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remainder = line;

    while let Some(start) = remainder.find('\'') {
        let rest = &remainder[start + 1..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        values.push(rest[..end].to_owned());
        remainder = &rest[end + 1..];
    }

    values
}

fn looks_like_failed_activity_launch(output: &str) -> bool {
    output.contains("Error type")
        || output.contains("Activity class")
        || output.contains("does not exist")
}

#[cfg(test)]
mod tests {
    use super::parse_badging;

    #[test]
    fn parse_badging_extracts_package_and_activity() {
        let metadata = parse_badging(
            "package: name='com.example.app' versionCode='1'\nlaunchable-activity: name='.MainActivity' label='' icon=''\n",
        )
        .expect("metadata should parse");

        assert_eq!(metadata.package_name, "com.example.app");
        assert_eq!(
            metadata.launchable_activity.as_deref(),
            Some(".MainActivity")
        );
        assert!(metadata.native_abis.is_empty());
    }

    #[test]
    fn parse_badging_allows_missing_activity() {
        let metadata =
            parse_badging("package: name='com.example.app' versionCode='1'\n").expect("metadata");

        assert_eq!(metadata.package_name, "com.example.app");
        assert_eq!(metadata.launchable_activity, None);
        assert!(metadata.native_abis.is_empty());
    }

    #[test]
    fn parse_badging_extracts_native_abis() {
        let metadata = parse_badging(
            "package: name='com.example.app' versionCode='1'\nnative-code: 'arm64-v8a' 'armeabi-v7a'\n",
        )
        .expect("metadata");

        assert_eq!(metadata.native_abis, vec!["arm64-v8a", "armeabi-v7a"]);
        assert!(metadata.uses_arm_translation_on_x86_emulator());
    }

    #[test]
    fn parse_badging_detects_x86_ready_apk() {
        let metadata = parse_badging(
            "package: name='com.example.app' versionCode='1'\nlaunchable-activity: name='com.example.app.MainActivity' label='' icon=''\nnative-code: 'x86_64'\n",
        )
        .expect("metadata");

        assert_eq!(metadata.native_abis, vec!["x86_64"]);
        assert!(!metadata.uses_arm_translation_on_x86_emulator());
    }
}
