use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(long, global = true, default_value = "rustdroid.toml")]
    pub config: PathBuf,

    #[arg(long, global = true)]
    pub image: Option<String>,

    #[arg(long, global = true)]
    pub container_name: Option<String>,

    #[arg(long, global = true)]
    pub device: Option<String>,

    #[arg(long, global = true)]
    pub adb_serial: Option<String>,

    #[arg(long, global = true)]
    pub adb_connect_port: Option<u16>,

    #[arg(long, global = true)]
    pub boot_timeout_secs: Option<u64>,

    #[arg(long, global = true)]
    pub poll_interval_secs: Option<u64>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub headless: Option<bool>,

    #[arg(long, global = true, default_value_t = false)]
    pub fast_local: bool,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub no_skin: Option<bool>,

    #[arg(long, global = true)]
    pub emulator_additional_args: Option<String>,

    #[arg(long, global = true)]
    pub emulator_cpu_cores: Option<u16>,

    #[arg(long, global = true)]
    pub emulator_ram_mb: Option<u64>,

    #[arg(long, global = true)]
    pub emulator_vm_heap_mb: Option<u64>,

    #[arg(long, global = true)]
    pub emulator_gpu_mode: Option<String>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub disable_animations: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub optimize_android_runtime: Option<bool>,

    #[arg(long, global = true)]
    pub device_width_px: Option<u16>,

    #[arg(long, global = true)]
    pub device_height_px: Option<u16>,

    #[arg(long, global = true)]
    pub device_density_dpi: Option<u16>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub compile_installed_package: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub disable_preinstalled_packages: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub disable_google_play_services: Option<bool>,

    #[arg(long, global = true, value_enum)]
    pub ui_backend: Option<UiBackend>,

    #[arg(long, global = true)]
    pub scrcpy_max_fps: Option<u16>,

    #[arg(long, global = true)]
    pub scrcpy_max_size: Option<u16>,

    #[arg(long, global = true)]
    pub scrcpy_video_bit_rate: Option<String>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub emulator_enable_audio: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub emulator_enable_battery: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub emulator_enable_gps: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub emulator_enable_motion_sensors: Option<bool>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub emulator_enable_environment_sensors: Option<bool>,

    #[arg(long, global = true)]
    pub vnc_port: Option<u16>,

    #[arg(long, global = true)]
    pub web_vnc_port: Option<u16>,

    #[arg(long, global = true, value_enum)]
    pub runtime_backend: Option<RuntimeBackend>,

    #[arg(long, global = true)]
    pub host_avd_name: Option<String>,

    #[arg(long, global = true)]
    pub host_emulator_binary: Option<String>,

    #[arg(long, global = true)]
    pub host_emulator_port: Option<u16>,

    #[arg(long, global = true, action = ArgAction::Set)]
    pub docker_gpu_passthrough: Option<bool>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Start(StartArgs),
    Install(InstallArgs),
    Run(RunArgs),
    Logs(LogsArgs),
    Stop(StopArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct StartArgs {
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub wait: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct InstallArgs {
    pub apk: PathBuf,

    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub replace: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    pub apk: PathBuf,

    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub replace: bool,

    #[arg(long)]
    pub duration_secs: Option<u64>,

    #[arg(long, default_value_t = LogSource::Logcat, value_enum)]
    pub log_source: LogSource,
}

#[derive(Debug, Clone, clap::Args)]
pub struct LogsArgs {
    #[arg(long, default_value_t = LogSource::Both, value_enum)]
    pub source: LogSource,

    #[arg(long)]
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct StopArgs {
    #[arg(long, default_value_t = 15)]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialEq, ValueEnum)]
pub enum LogSource {
    Container,
    Logcat,
    #[default]
    Both,
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum UiBackend {
    #[default]
    #[serde(alias = "native")]
    #[value(alias = "native")]
    Scrcpy,
    Vnc,
    Web,
    Both,
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBackend {
    #[default]
    Docker,
    Host,
}
