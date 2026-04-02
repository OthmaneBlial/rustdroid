use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Fast Android emulator orchestration for local APK testing"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = "rustdroid.toml")]
    pub config: PathBuf,

    #[arg(long, global = true, default_value_t = false)]
    pub json: bool,

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

    #[arg(long, global = true, value_enum)]
    pub boot_mode: Option<BootMode>,

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

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    #[command(about = "Check host and runtime prerequisites")]
    Doctor(DoctorArgs),
    #[command(about = "Run a quick RustDroid backend smoke check")]
    SelfTest(SelfTestArgs),
    #[command(about = "List adb-visible devices")]
    Devices(DevicesArgs),
    #[command(about = "List available host Android Virtual Devices")]
    Avds(AvdsArgs),
    #[command(about = "Print the installed RustDroid version")]
    Version,
    #[command(about = "Generate shell completions")]
    Completions(CompletionsArgs),
    #[command(about = "Measure boot, install, and launch timings")]
    Bench(BenchArgs),
    #[command(about = "Inspect or write named RustDroid profiles")]
    Profile(ProfileArgs),
    #[command(about = "Initialize project config files")]
    Config(ConfigArgs),
    #[command(about = "Remove RustDroid-managed containers and temp state")]
    Clean(CleanArgs),
    #[command(about = "Start the emulator and optionally wait for boot")]
    Start(StartArgs),
    #[command(about = "Open the emulator UI without installing an app")]
    Open(OpenArgs),
    #[command(about = "Install an APK on the active emulator")]
    Install(InstallArgs),
    #[command(about = "Launch an installed app by APK metadata or package name")]
    Launch(LaunchArgs),
    #[command(about = "Start, install, launch, and stream logs for an APK")]
    Run(RunArgs),
    #[command(about = "Stream emulator or logcat logs")]
    Logs(LogsArgs),
    #[command(about = "Stop the active RustDroid runtime")]
    Stop(StopArgs),
}

#[derive(Debug, Clone, clap::Args, Default)]
pub struct DoctorArgs {}

#[derive(Debug, Clone, clap::Args)]
pub struct SelfTestArgs {
    #[arg(long, value_enum, default_value_t = BackendScope::Current)]
    pub backend: BackendScope,

    #[arg(long, default_value_t = false)]
    pub full: bool,
}

#[derive(Debug, Clone, clap::Args, Default)]
pub struct DevicesArgs {}

#[derive(Debug, Clone, clap::Args, Default)]
pub struct AvdsArgs {}

#[derive(Debug, Clone, clap::Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

#[derive(Debug, Clone, clap::Args)]
pub struct BenchArgs {
    pub apk: Option<PathBuf>,

    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub replace: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProfileCommand {
    #[command(about = "List built-in profiles")]
    List,
    #[command(about = "Write a built-in profile into the config file")]
    Use(ProfileUseArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommand,
}

#[derive(Debug, Clone, clap::Args)]
pub struct ProfileUseArgs {
    pub name: String,

    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Create a config file with optional profile defaults")]
    Init(ConfigInitArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Debug, Clone, clap::Args)]
pub struct ConfigInitArgs {
    #[arg(long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Clone, clap::Args, Default)]
pub struct CleanArgs {
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct StartArgs {
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub wait: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct OpenArgs {
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub wait: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct InstallArgs {
    #[arg(required = true)]
    pub apks: Vec<PathBuf>,

    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub replace: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct LaunchArgs {
    pub apk: Option<PathBuf>,

    #[arg(long)]
    pub package: Option<String>,

    #[arg(long)]
    pub activity: Option<String>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    #[arg(required = true)]
    pub apks: Vec<PathBuf>,

    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub replace: bool,

    #[arg(long)]
    pub duration_secs: Option<u64>,

    #[arg(long, default_value_t = LogSource::Logcat, value_enum)]
    pub log_source: LogSource,

    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub keep_alive: bool,

    #[arg(long)]
    pub artifacts_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct LogsArgs {
    #[arg(long, default_value_t = LogSource::Both, value_enum)]
    pub source: LogSource,

    #[arg(long)]
    pub duration_secs: Option<u64>,

    #[arg(long, default_value_t = false)]
    pub since_start: bool,

    #[arg(long)]
    pub package: Option<String>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct StopArgs {
    #[arg(long, default_value_t = 15)]
    pub timeout_secs: u64,

    #[arg(long, default_value_t = false)]
    pub all: bool,
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

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum BackendScope {
    #[default]
    Current,
    Docker,
    Host,
    Both,
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum BootMode {
    #[default]
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum CompletionShell {
    Bash,
    Zsh,
}
