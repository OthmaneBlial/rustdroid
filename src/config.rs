use std::{fs, path::Path, thread};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use toml::Value;

use crate::cli::{BootMode, Cli, RuntimeBackend, UiBackend};
use crate::profiles::apply_named_profile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    pub runtime_backend: RuntimeBackend,
    pub image: String,
    pub container_name: String,
    pub device: String,
    pub adb_serial: String,
    pub adb_connect_port: u16,
    pub boot_timeout_secs: u64,
    pub poll_interval_secs: u64,
    pub boot_mode: BootMode,
    pub headless: bool,
    pub no_skin: bool,
    pub emulator_additional_args: String,
    pub emulator_cpu_cores: u16,
    pub emulator_ram_mb: u64,
    pub emulator_vm_heap_mb: u64,
    pub emulator_gpu_mode: String,
    pub disable_animations: bool,
    pub optimize_android_runtime: bool,
    pub device_width_px: u16,
    pub device_height_px: u16,
    pub device_density_dpi: u16,
    pub compile_installed_package: bool,
    pub disable_preinstalled_packages: bool,
    pub disable_google_play_services: bool,
    pub ui_backend: UiBackend,
    pub scrcpy_max_fps: u16,
    pub scrcpy_max_size: u16,
    pub scrcpy_video_bit_rate: String,
    pub emulator_enable_audio: bool,
    pub emulator_enable_battery: bool,
    pub emulator_enable_gps: bool,
    pub emulator_enable_motion_sensors: bool,
    pub emulator_enable_environment_sensors: bool,
    pub vnc_port: u16,
    pub web_vnc_port: u16,
    pub host_avd_name: Option<String>,
    pub host_emulator_binary: String,
    pub host_emulator_port: u16,
    pub docker_gpu_passthrough: bool,
    pub remote_apk_dir: String,
    pub remote_apk_name: String,
    pub logcat_filters: Vec<String>,
    pub artifacts_dir: Option<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        let emulator_cpu_cores = default_emulator_cpu_cores();
        let emulator_ram_mb = default_emulator_ram_mb();
        let emulator_vm_heap_mb = default_emulator_vm_heap_mb(emulator_ram_mb);

        Self {
            runtime_backend: RuntimeBackend::Docker,
            image: "budtmo/docker-android:emulator_14.0".to_owned(),
            container_name: "rustdroid-emulator".to_owned(),
            device: "Nexus 5".to_owned(),
            adb_serial: "emulator-5554".to_owned(),
            adb_connect_port: 5555,
            boot_timeout_secs: 360,
            poll_interval_secs: 2,
            boot_mode: BootMode::Warm,
            headless: true,
            no_skin: true,
            emulator_additional_args: "-no-audio -no-boot-anim -no-snapshot -no-snapshot-save -no-metrics -camera-back none -camera-front none -skip-adb-auth".to_owned(),
            emulator_cpu_cores,
            emulator_ram_mb,
            emulator_vm_heap_mb,
            emulator_gpu_mode: "swiftshader_indirect".to_owned(),
            disable_animations: true,
            optimize_android_runtime: true,
            device_width_px: 720,
            device_height_px: 1280,
            device_density_dpi: 280,
            compile_installed_package: true,
            disable_preinstalled_packages: true,
            disable_google_play_services: false,
            ui_backend: UiBackend::Scrcpy,
            scrcpy_max_fps: 30,
            scrcpy_max_size: 720,
            scrcpy_video_bit_rate: "4M".to_owned(),
            emulator_enable_audio: false,
            emulator_enable_battery: false,
            emulator_enable_gps: false,
            emulator_enable_motion_sensors: false,
            emulator_enable_environment_sensors: false,
            vnc_port: 5900,
            web_vnc_port: 6080,
            host_avd_name: None,
            host_emulator_binary: "emulator".to_owned(),
            host_emulator_port: 5554,
            docker_gpu_passthrough: true,
            remote_apk_dir: "/tmp/rustdroid".to_owned(),
            remote_apk_name: "app.apk".to_owned(),
            logcat_filters: vec!["*:I".to_owned()],
            artifacts_dir: None,
        }
    }
}

impl RuntimeConfig {
    pub fn load(cli: &Cli) -> Result<Self> {
        let mut config = Self::from_path(&cli.config)?;
        apply_env_overrides(&mut config)?;
        let default_image = Self::default().image;
        let serial_explicit = cli.adb_serial.is_some();
        let host_port_explicit = cli.host_emulator_port.is_some();

        if let Some(runtime_backend) = cli.runtime_backend {
            config.runtime_backend = runtime_backend;
        }

        if let Some(image) = &cli.image {
            config.image = image.clone();
        }
        if let Some(container_name) = &cli.container_name {
            config.container_name = container_name.clone();
        }
        if let Some(device) = &cli.device {
            config.device = device.clone();
        }
        if let Some(adb_serial) = &cli.adb_serial {
            config.adb_serial = adb_serial.clone();
        }
        if let Some(adb_connect_port) = cli.adb_connect_port {
            config.adb_connect_port = adb_connect_port;
        }
        if let Some(timeout) = cli.boot_timeout_secs {
            config.boot_timeout_secs = timeout;
        }
        if let Some(interval) = cli.poll_interval_secs {
            config.poll_interval_secs = interval;
        }
        if let Some(boot_mode) = cli.boot_mode {
            config.boot_mode = boot_mode;
        }
        if let Some(headless) = cli.headless {
            config.headless = headless;
        }
        if cli.fast_local {
            if cli.image.is_none() && config.image == default_image {
                config.image = "budtmo/docker-android:emulator_12.0".to_owned();
            }
            config.disable_google_play_services = true;
            config.device_width_px = 540;
            config.device_height_px = 960;
            config.device_density_dpi = 220;
            config.scrcpy_max_fps = 24;
            config.scrcpy_max_size = 540;
            config.scrcpy_video_bit_rate = "2M".to_owned();
        }
        if let Some(no_skin) = cli.no_skin {
            config.no_skin = no_skin;
        }
        if let Some(args) = &cli.emulator_additional_args {
            config.emulator_additional_args = args.clone();
        }
        if let Some(cpu_cores) = cli.emulator_cpu_cores {
            config.emulator_cpu_cores = cpu_cores;
        }
        if let Some(ram_mb) = cli.emulator_ram_mb {
            config.emulator_ram_mb = ram_mb;
        }
        if let Some(vm_heap_mb) = cli.emulator_vm_heap_mb {
            config.emulator_vm_heap_mb = vm_heap_mb;
        }
        if let Some(gpu_mode) = &cli.emulator_gpu_mode {
            config.emulator_gpu_mode = gpu_mode.clone();
        }
        if let Some(disable_animations) = cli.disable_animations {
            config.disable_animations = disable_animations;
        }
        if let Some(optimize_android_runtime) = cli.optimize_android_runtime {
            config.optimize_android_runtime = optimize_android_runtime;
        }
        if let Some(device_width_px) = cli.device_width_px {
            config.device_width_px = device_width_px;
        }
        if let Some(device_height_px) = cli.device_height_px {
            config.device_height_px = device_height_px;
        }
        if let Some(device_density_dpi) = cli.device_density_dpi {
            config.device_density_dpi = device_density_dpi;
        }
        if let Some(compile_installed_package) = cli.compile_installed_package {
            config.compile_installed_package = compile_installed_package;
        }
        if let Some(disable_preinstalled_packages) = cli.disable_preinstalled_packages {
            config.disable_preinstalled_packages = disable_preinstalled_packages;
        }
        if let Some(disable_google_play_services) = cli.disable_google_play_services {
            config.disable_google_play_services = disable_google_play_services;
        }
        if let Some(ui_backend) = cli.ui_backend {
            config.ui_backend = ui_backend;
        }
        if let Some(scrcpy_max_fps) = cli.scrcpy_max_fps {
            config.scrcpy_max_fps = scrcpy_max_fps;
        }
        if let Some(scrcpy_max_size) = cli.scrcpy_max_size {
            config.scrcpy_max_size = scrcpy_max_size;
        }
        if let Some(scrcpy_video_bit_rate) = &cli.scrcpy_video_bit_rate {
            config.scrcpy_video_bit_rate = scrcpy_video_bit_rate.clone();
        }
        if let Some(enable_audio) = cli.emulator_enable_audio {
            config.emulator_enable_audio = enable_audio;
        }
        if let Some(enable_battery) = cli.emulator_enable_battery {
            config.emulator_enable_battery = enable_battery;
        }
        if let Some(enable_gps) = cli.emulator_enable_gps {
            config.emulator_enable_gps = enable_gps;
        }
        if let Some(enable_motion_sensors) = cli.emulator_enable_motion_sensors {
            config.emulator_enable_motion_sensors = enable_motion_sensors;
        }
        if let Some(enable_environment_sensors) = cli.emulator_enable_environment_sensors {
            config.emulator_enable_environment_sensors = enable_environment_sensors;
        }
        if let Some(vnc_port) = cli.vnc_port {
            config.vnc_port = vnc_port;
        }
        if let Some(web_vnc_port) = cli.web_vnc_port {
            config.web_vnc_port = web_vnc_port;
        }
        if let Some(host_avd_name) = &cli.host_avd_name {
            config.host_avd_name = Some(host_avd_name.clone());
        }
        if let Some(host_emulator_binary) = &cli.host_emulator_binary {
            config.host_emulator_binary = host_emulator_binary.clone();
        }
        if let Some(host_emulator_port) = cli.host_emulator_port {
            config.host_emulator_port = host_emulator_port;
        }
        if let Some(docker_gpu_passthrough) = cli.docker_gpu_passthrough {
            config.docker_gpu_passthrough = docker_gpu_passthrough;
        }
        if config.runtime_backend == RuntimeBackend::Host {
            if let Some(port) = host_emulator_port_from_serial(&config.adb_serial) {
                if !host_port_explicit {
                    config.host_emulator_port = port;
                }
            }

            if host_port_explicit || !serial_explicit {
                config.adb_serial = format!("emulator-{}", config.host_emulator_port);
            }

            if cli.emulator_gpu_mode.is_none()
                && config.emulator_gpu_mode == Self::default().emulator_gpu_mode
            {
                config.emulator_gpu_mode = if config.effective_emulator_headless() {
                    "auto-no-window".to_owned()
                } else {
                    "host".to_owned()
                };
            }
        }
        if cli.container_name.is_none() && config.container_name == Self::default().container_name {
            config.container_name = default_container_name(&config);
        }

        Ok(config)
    }

    pub fn effective_emulator_additional_args(&self) -> String {
        let mut parts: Vec<String> = self
            .emulator_additional_args
            .split_whitespace()
            .map(str::to_owned)
            .collect();

        if self.effective_emulator_headless() {
            if !parts.iter().any(|part| part == "-no-window") {
                parts.push("-no-window".to_owned());
            }
        } else {
            parts.retain(|part| part != "-no-window");
        }

        parts.join(" ")
    }

    pub fn emulator_override_config(&self) -> String {
        let audio = bool_flag(self.emulator_enable_audio);
        let battery = bool_flag(self.emulator_enable_battery);
        let gps = bool_flag(self.emulator_enable_gps);
        let motion_sensors = bool_flag(self.emulator_enable_motion_sensors);
        let environment_sensors = bool_flag(self.emulator_enable_environment_sensors);

        format!(
            "hw.cpu.ncore = {cpu_cores}\n\
hw.ramSize = {ram_mb}\n\
vm.heapSize = {vm_heap_mb}M\n\
hw.gpu.enabled = yes\n\
hw.gpu.mode = {gpu_mode}\n\
hw.audioInput = {audio}\n\
hw.audioOutput = {audio}\n\
hw.battery = {battery}\n\
hw.gps = {gps}\n\
hw.accelerometer = {motion_sensors}\n\
hw.accelerometer_uncalibrated = {motion_sensors}\n\
hw.gyroscope = {motion_sensors}\n\
hw.sensor.roll = {motion_sensors}\n\
hw.sensors.orientation = {motion_sensors}\n\
hw.sensors.gyroscope_uncalibrated = {motion_sensors}\n\
hw.sensors.light = {environment_sensors}\n\
hw.sensors.proximity = {environment_sensors}\n\
hw.sensors.pressure = {environment_sensors}\n\
hw.sensors.humidity = {environment_sensors}\n\
hw.sensors.temperature = {environment_sensors}\n\
hw.sensors.magnetic_field = {environment_sensors}\n\
hw.sensors.magnetic_field_uncalibrated = {environment_sensors}\n\
hw.sensors.heading = {environment_sensors}\n\
hw.camera.back = none\n\
hw.camera.front = none\n\
showDeviceFrame = no\n",
            cpu_cores = self.emulator_cpu_cores,
            ram_mb = self.emulator_ram_mb,
            vm_heap_mb = self.emulator_vm_heap_mb,
            gpu_mode = self.emulator_gpu_mode,
            audio = audio,
            battery = battery,
            gps = gps,
            motion_sensors = motion_sensors,
            environment_sensors = environment_sensors
        )
    }

    pub fn uses_scrcpy_ui(&self) -> bool {
        matches!(self.ui_backend, UiBackend::Scrcpy | UiBackend::Both)
    }

    pub fn uses_vnc_ui(&self) -> bool {
        matches!(self.ui_backend, UiBackend::Vnc)
    }

    pub fn uses_web_ui(&self) -> bool {
        matches!(self.ui_backend, UiBackend::Web | UiBackend::Both)
    }

    pub fn uses_screen_stack(&self) -> bool {
        !self.headless && (self.uses_vnc_ui() || self.uses_web_ui())
    }

    pub fn effective_emulator_headless(&self) -> bool {
        self.headless || !self.uses_screen_stack()
    }

    pub fn uses_host_runtime(&self) -> bool {
        self.runtime_backend == RuntimeBackend::Host
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let parsed: Value = toml::from_str(&raw)
            .with_context(|| format!("failed to parse config file {}", path.display()))?;
        let mut config = Self::default();

        if let Some(table) = parsed.as_table() {
            if let Some(profile) = table
                .get("profile")
                .or_else(|| table.get("extends"))
                .and_then(Value::as_str)
            {
                apply_named_profile(&mut config, profile)?;
            }
        }

        let mut merged = Value::try_from(config)
            .with_context(|| format!("failed to merge {}", path.display()))?;

        if let (Some(base_table), Some(file_table)) = (merged.as_table_mut(), parsed.as_table()) {
            for (key, value) in file_table {
                if matches!(key.as_str(), "profile" | "extends") {
                    continue;
                }
                base_table.insert(key.clone(), value.clone());
            }
        }

        merged
            .try_into()
            .with_context(|| format!("failed to build config from {}", path.display()))
    }
}

fn default_emulator_cpu_cores() -> u16 {
    thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(4)
        .clamp(2, 8) as u16
}

fn default_emulator_ram_mb() -> u64 {
    match detect_host_memory_mb() {
        Some(memory_mb) if memory_mb >= 16 * 1024 => 8 * 1024,
        Some(memory_mb) if memory_mb >= 12 * 1024 => 6 * 1024,
        Some(memory_mb) if memory_mb >= 8 * 1024 => 4 * 1024,
        Some(memory_mb) if memory_mb >= 6 * 1024 => 3 * 1024,
        _ => 2 * 1024,
    }
}

fn default_emulator_vm_heap_mb(emulator_ram_mb: u64) -> u64 {
    (emulator_ram_mb / 8).clamp(256, 512)
}

fn detect_host_memory_mb() -> Option<u64> {
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;

    for line in meminfo.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(kib / 1024);
        }
    }

    None
}

fn bool_flag(enabled: bool) -> &'static str {
    if enabled {
        "yes"
    } else {
        "no"
    }
}

fn host_emulator_port_from_serial(serial: &str) -> Option<u16> {
    serial
        .strip_prefix("emulator-")
        .and_then(|value| value.parse::<u16>().ok())
}

fn apply_env_overrides(config: &mut RuntimeConfig) -> Result<()> {
    if let Ok(profile) = std::env::var("RUSTDROID_PROFILE") {
        if !profile.trim().is_empty() {
            apply_named_profile(config, profile.trim())?;
        }
    }

    if let Ok(value) = std::env::var("RUSTDROID_RUNTIME_BACKEND") {
        config.runtime_backend = parse_runtime_backend(&value)?;
    }
    if let Ok(value) = std::env::var("RUSTDROID_BOOT_MODE") {
        config.boot_mode = parse_boot_mode(&value)?;
    }
    if let Ok(value) = std::env::var("RUSTDROID_IMAGE") {
        config.image = value;
    }
    if let Ok(value) = std::env::var("RUSTDROID_CONTAINER_NAME") {
        config.container_name = value;
    }
    if let Ok(value) = std::env::var("RUSTDROID_HOST_AVD_NAME") {
        config.host_avd_name = Some(value);
    }
    if let Ok(value) = std::env::var("RUSTDROID_HOST_EMULATOR_PORT") {
        config.host_emulator_port = value
            .parse::<u16>()
            .with_context(|| format!("invalid RUSTDROID_HOST_EMULATOR_PORT='{value}'"))?;
    }
    if let Ok(value) = std::env::var("RUSTDROID_EMULATOR_GPU_MODE") {
        config.emulator_gpu_mode = value;
    }
    if let Ok(value) = std::env::var("RUSTDROID_UI_BACKEND") {
        config.ui_backend = parse_ui_backend(&value)?;
    }
    if let Ok(value) = std::env::var("RUSTDROID_LOGCAT_FILTERS") {
        config.logcat_filters = value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .collect();
    }
    if let Ok(value) = std::env::var("RUSTDROID_ARTIFACTS_DIR") {
        config.artifacts_dir = if value.trim().is_empty() {
            None
        } else {
            Some(value)
        };
    }

    Ok(())
}

fn parse_runtime_backend(value: &str) -> Result<RuntimeBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        "docker" => Ok(RuntimeBackend::Docker),
        "host" => Ok(RuntimeBackend::Host),
        _ => anyhow::bail!(
            "invalid RUSTDROID_RUNTIME_BACKEND='{}' (expected 'docker' or 'host')",
            value
        ),
    }
}

fn parse_boot_mode(value: &str) -> Result<BootMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "warm" => Ok(BootMode::Warm),
        "cold" => Ok(BootMode::Cold),
        _ => anyhow::bail!(
            "invalid RUSTDROID_BOOT_MODE='{}' (expected 'warm' or 'cold')",
            value
        ),
    }
}

fn parse_ui_backend(value: &str) -> Result<UiBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        "scrcpy" | "native" => Ok(UiBackend::Scrcpy),
        "vnc" => Ok(UiBackend::Vnc),
        "web" => Ok(UiBackend::Web),
        "both" => Ok(UiBackend::Both),
        _ => anyhow::bail!(
            "invalid RUSTDROID_UI_BACKEND='{}' (expected 'scrcpy', 'vnc', 'web', or 'both')",
            value
        ),
    }
}

fn default_container_name(config: &RuntimeConfig) -> String {
    if config.headless {
        return "rustdroid-emulator".to_owned();
    }

    match config.ui_backend {
        UiBackend::Scrcpy => "rustdroid-emulator".to_owned(),
        UiBackend::Vnc => "rustdroid-emulator-vnc".to_owned(),
        UiBackend::Web => "rustdroid-emulator-web".to_owned(),
        UiBackend::Both => "rustdroid-emulator-both".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    use clap::Parser;
    use tempfile::tempdir;

    use super::RuntimeConfig;
    use crate::cli::{Cli, RuntimeBackend, UiBackend};

    #[test]
    fn scrcpy_visible_mode_keeps_emulator_headless() {
        let config = RuntimeConfig {
            headless: false,
            ui_backend: UiBackend::Scrcpy,
            scrcpy_max_fps: 30,
            scrcpy_max_size: 720,
            scrcpy_video_bit_rate: "4M".to_owned(),
            ..RuntimeConfig::default()
        };

        assert!(config.effective_emulator_headless());
        assert!(config.uses_scrcpy_ui());
        assert!(!config.uses_screen_stack());
    }

    #[test]
    fn web_mode_requires_screen_stack() {
        let config = RuntimeConfig {
            headless: false,
            ui_backend: UiBackend::Web,
            ..RuntimeConfig::default()
        };

        assert!(!config.effective_emulator_headless());
        assert!(config.uses_screen_stack());
    }

    #[test]
    fn fast_local_prefers_emulator_12_when_image_not_overridden() {
        let _guard = env_lock().lock().expect("env lock should be available");
        clear_env();
        let cli = Cli::parse_from([
            "rustdroid",
            "--config",
            "/tmp/rustdroid-nonexistent.toml",
            "--fast-local",
            "start",
            "--wait",
            "false",
        ]);

        let config = RuntimeConfig::load(&cli).expect("fast local config should load");
        assert_eq!(config.image, "budtmo/docker-android:emulator_12.0");
    }

    #[test]
    fn host_backend_defaults_to_host_gpu_and_serial_from_port() {
        let _guard = env_lock().lock().expect("env lock should be available");
        clear_env();
        let cli = Cli::parse_from([
            "rustdroid",
            "--config",
            "/tmp/rustdroid-nonexistent.toml",
            "--runtime-backend",
            "host",
            "--host-emulator-port",
            "5560",
            "start",
            "--wait",
            "false",
        ]);

        let config = RuntimeConfig::load(&cli).expect("host config should load");
        assert_eq!(config.runtime_backend, RuntimeBackend::Host);
        assert_eq!(config.adb_serial, "emulator-5560");
        assert_eq!(config.emulator_gpu_mode, "auto-no-window");
    }

    #[test]
    fn visible_host_backend_defaults_to_host_gpu_mode() {
        let _guard = env_lock().lock().expect("env lock should be available");
        clear_env();
        let cli = Cli::parse_from([
            "rustdroid",
            "--config",
            "/tmp/rustdroid-nonexistent.toml",
            "--runtime-backend",
            "host",
            "--headless",
            "false",
            "--ui-backend",
            "web",
            "start",
            "--wait",
            "false",
        ]);

        let config = RuntimeConfig::load(&cli).expect("host config should load");
        assert_eq!(config.runtime_backend, RuntimeBackend::Host);
        assert_eq!(config.emulator_gpu_mode, "host");
        assert_eq!(config.adb_serial, "emulator-5554");
    }

    #[test]
    fn headless_args_add_no_window_only_once() {
        let config = RuntimeConfig {
            headless: true,
            emulator_additional_args: "-no-window -no-boot-anim".to_owned(),
            ..RuntimeConfig::default()
        };

        assert_eq!(
            config.effective_emulator_additional_args(),
            "-no-window -no-boot-anim"
        );
    }

    #[test]
    fn invalid_boot_mode_env_override_returns_error() {
        let _guard = env_lock().lock().expect("env lock should be available");
        clear_env();
        std::env::set_var("RUSTDROID_BOOT_MODE", "broken");

        let cli = Cli::parse_from([
            "rustdroid",
            "--config",
            "/tmp/rustdroid-nonexistent.toml",
            "start",
            "--wait",
            "false",
        ]);

        let error = RuntimeConfig::load(&cli).expect_err("invalid boot mode should fail");
        assert!(
            error
                .to_string()
                .contains("invalid RUSTDROID_BOOT_MODE='broken'"),
            "unexpected error: {error}"
        );

        clear_env();
    }

    #[test]
    fn env_profile_and_backend_overrides_are_applied() {
        let _guard = env_lock().lock().expect("env lock should be available");
        clear_env();
        std::env::set_var("RUSTDROID_PROFILE", "browser-demo");
        std::env::set_var("RUSTDROID_RUNTIME_BACKEND", "host");
        std::env::set_var("RUSTDROID_BOOT_MODE", "cold");
        std::env::set_var("RUSTDROID_UI_BACKEND", "scrcpy");
        std::env::set_var("RUSTDROID_EMULATOR_GPU_MODE", "auto-no-window");

        let cli = Cli::parse_from([
            "rustdroid",
            "--config",
            "/tmp/rustdroid-nonexistent.toml",
            "start",
            "--wait",
            "false",
        ]);

        let config = RuntimeConfig::load(&cli).expect("env override config should load");
        assert_eq!(config.runtime_backend, RuntimeBackend::Host);
        assert_eq!(config.boot_mode, crate::cli::BootMode::Cold);
        assert_eq!(config.ui_backend, UiBackend::Scrcpy);
        assert_eq!(config.emulator_gpu_mode, "auto-no-window");

        clear_env();
    }

    #[test]
    fn config_file_can_extend_a_named_profile() {
        let temp_dir = tempdir().expect("temp dir");
        let config_path = temp_dir.path().join("rustdroid.toml");
        fs::write(
            &config_path,
            r#"
profile = "host-fast"
host_avd_name = "team_avd"
artifacts_dir = ".rustdroid/artifacts"
"#,
        )
        .expect("config should be written");

        let config = RuntimeConfig::from_path(&config_path).expect("config should load");
        assert_eq!(config.runtime_backend, RuntimeBackend::Host);
        assert_eq!(config.host_avd_name.as_deref(), Some("team_avd"));
        assert_eq!(
            config.artifacts_dir.as_deref(),
            Some(".rustdroid/artifacts")
        );
        assert_eq!(config.emulator_gpu_mode, "host");
    }

    #[test]
    fn env_artifact_and_logcat_overrides_are_applied() {
        let _guard = env_lock().lock().expect("env lock should be available");
        clear_env();
        std::env::set_var("RUSTDROID_ARTIFACTS_DIR", ".rustdroid/runs");
        std::env::set_var("RUSTDROID_LOGCAT_FILTERS", "*:W,MyApp:I");

        let cli = Cli::parse_from([
            "rustdroid",
            "--config",
            "/tmp/rustdroid-nonexistent.toml",
            "run",
            "app.apk",
        ]);

        let config = RuntimeConfig::load(&cli).expect("env override config should load");
        assert_eq!(config.artifacts_dir.as_deref(), Some(".rustdroid/runs"));
        assert_eq!(config.logcat_filters, vec!["*:W", "MyApp:I"]);

        clear_env();
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clear_env() {
        for key in [
            "RUSTDROID_PROFILE",
            "RUSTDROID_RUNTIME_BACKEND",
            "RUSTDROID_BOOT_MODE",
            "RUSTDROID_UI_BACKEND",
            "RUSTDROID_EMULATOR_GPU_MODE",
            "RUSTDROID_LOGCAT_FILTERS",
            "RUSTDROID_ARTIFACTS_DIR",
        ] {
            std::env::remove_var(key);
        }
    }
}
