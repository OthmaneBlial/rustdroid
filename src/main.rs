mod adb;
mod apks;
mod cli;
mod config;
mod diagnostics;
mod display;
mod docker;
mod emulator;
mod host;
mod logs;
mod output;
mod profiles;
mod runtime;
mod tooling;

use std::process;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command, ConfigCommand, ProfileCommand};
use config::RuntimeConfig;
use emulator::EmulatorOrchestrator;
use runtime::Runtime;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let command = cli.command.clone();

    if let Err(error) = run(cli, command.clone()).await {
        eprintln!("error: {error}");
        if let Some(hint) = error_hint(&command, &error.to_string()) {
            eprintln!("hint: {hint}");
        }
        process::exit(exit_code_for_command(&command));
    }
}

async fn run(cli: Cli, command: Command) -> Result<()> {
    match command {
        Command::Version => diagnostics::print_version(cli.json)?,
        Command::Completions(args) => diagnostics::print_completions(args.shell),
        Command::Devices(_args) => diagnostics::run_devices(cli.json).await?,
        Command::Doctor(_args) => {
            let config = RuntimeConfig::load(&cli)?;
            diagnostics::run_doctor(&config, cli.json).await?;
        }
        Command::Avds(_args) => {
            let config = RuntimeConfig::load(&cli)?;
            diagnostics::run_avds(&config, cli.json).await?;
        }
        Command::SelfTest(args) => {
            let config = RuntimeConfig::load(&cli)?;
            diagnostics::run_self_test(&config, &args, cli.json).await?;
        }
        Command::Bench(args) => {
            let config = RuntimeConfig::load(&cli)?;
            let runtime = Runtime::connect(&config)?;
            let orchestrator = EmulatorOrchestrator::new(config, runtime);
            orchestrator.bench(args, cli.json).await?;
        }
        Command::Profile(args) => match args.command {
            ProfileCommand::List => tooling::list_profiles(cli.json)?,
            ProfileCommand::Use(args) => {
                tooling::use_profile(&cli.config, &args.name, args.force, cli.json)?
            }
        },
        Command::Config(args) => match args.command {
            ConfigCommand::Init(args) => tooling::init_config(&cli.config, &args, cli.json)?,
        },
        Command::Clean(args) => tooling::clean(args.dry_run, cli.json).await?,
        Command::Stop(args) if args.all => tooling::stop_all(args.timeout_secs).await?,
        command => {
            let config = RuntimeConfig::load(&cli)?;
            let runtime = Runtime::connect(&config)?;
            let orchestrator = EmulatorOrchestrator::new(config, runtime);

            match command {
                Command::Start(args) => orchestrator.start(args).await?,
                Command::Open(args) => orchestrator.open(args).await?,
                Command::Install(args) => orchestrator.install(args).await?,
                Command::Launch(args) => orchestrator.launch(args).await?,
                Command::Uninstall(args) => orchestrator.uninstall(args).await?,
                Command::ClearData(args) => orchestrator.clear_data(args).await?,
                Command::Run(args) => orchestrator.run(args).await?,
                Command::Watch(args) => orchestrator.watch(args).await?,
                Command::Logs(args) => orchestrator.logs(args).await?,
                Command::Stop(args) => orchestrator.stop(args).await?,
                _ => unreachable!("non-runtime commands are handled above"),
            }
        }
    }
    Ok(())
}

fn exit_code_for_command(command: &Command) -> i32 {
    match command {
        Command::Doctor(_) => 10,
        Command::SelfTest(_) => 11,
        Command::Bench(_) => 12,
        Command::Config(_) => 13,
        Command::Profile(_) => 14,
        Command::Clean(_) => 15,
        Command::Devices(_) | Command::Avds(_) => 16,
        _ => 1,
    }
}

fn error_hint(command: &Command, message: &str) -> Option<&'static str> {
    let lowercase = message.to_ascii_lowercase();

    if lowercase.contains("apk not found") {
        return Some("Pass a real APK path from your build output.");
    }
    if lowercase.contains("no host avd found") || lowercase.contains("no available avds") {
        return Some("Run `rustdroid avds` or create an AVD in Android Studio.");
    }
    if lowercase.contains("/dev/kvm") {
        return Some("Run `rustdroid doctor` and fix KVM permissions before trying again.");
    }
    if lowercase.contains("docker") {
        return Some(
            "Start Docker or switch to `--runtime-backend host` for the host emulator path.",
        );
    }

    match command {
        Command::Doctor(_) | Command::SelfTest(_) => Some(
            "Review the failing checks above and rerun the command once the environment is fixed.",
        ),
        Command::Config(_) | Command::Profile(_) => {
            Some("Use `--config <path>` to target a different config file if needed.")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{error_hint, exit_code_for_command};
    use crate::cli::{
        BackendScope, BenchArgs, CleanArgs, ClearDataArgs, Command, ConfigArgs, ConfigCommand,
        ConfigInitArgs, DoctorArgs, ProfileArgs, ProfileCommand, ProfileUseArgs, RunArgs,
        SelfTestArgs, StopArgs, UninstallArgs,
    };

    #[test]
    fn exit_codes_stay_stable_for_non_runtime_commands() {
        assert_eq!(
            exit_code_for_command(&Command::Doctor(DoctorArgs::default())),
            10
        );
        assert_eq!(
            exit_code_for_command(&Command::SelfTest(SelfTestArgs {
                backend: BackendScope::Current,
                full: false,
            })),
            11
        );
        assert_eq!(
            exit_code_for_command(&Command::Bench(BenchArgs {
                apk: None,
                replace: true,
            })),
            12
        );
        assert_eq!(
            exit_code_for_command(&Command::Config(ConfigArgs {
                command: ConfigCommand::Init(ConfigInitArgs {
                    profile: None,
                    force: false,
                }),
            })),
            13
        );
        assert_eq!(
            exit_code_for_command(&Command::Profile(ProfileArgs {
                command: ProfileCommand::Use(ProfileUseArgs {
                    name: "fast-local".to_owned(),
                    force: false,
                }),
            })),
            14
        );
        assert_eq!(
            exit_code_for_command(&Command::Clean(CleanArgs { dry_run: true })),
            15
        );
    }

    #[test]
    fn runtime_commands_keep_generic_exit_code() {
        assert_eq!(
            exit_code_for_command(&Command::Run(RunArgs {
                apks: Vec::new(),
                replace: true,
                duration_secs: None,
                log_source: crate::cli::LogSource::Logcat,
                keep_alive: true,
                artifacts_dir: None,
            })),
            1
        );
        assert_eq!(
            exit_code_for_command(&Command::Stop(StopArgs {
                timeout_secs: 15,
                all: false,
            })),
            1
        );
        assert_eq!(
            exit_code_for_command(&Command::Uninstall(UninstallArgs {
                input: None,
                package: Some("com.example.app".to_owned()),
            })),
            1
        );
        assert_eq!(
            exit_code_for_command(&Command::ClearData(ClearDataArgs {
                input: None,
                package: Some("com.example.app".to_owned()),
            })),
            1
        );
    }

    #[test]
    fn error_hint_maps_common_runtime_failures() {
        let run_command = Command::Run(RunArgs {
            apks: Vec::new(),
            replace: true,
            duration_secs: None,
            log_source: crate::cli::LogSource::Logcat,
            keep_alive: true,
            artifacts_dir: None,
        });

        assert_eq!(
            error_hint(&run_command, "APK not found: app.apk"),
            Some("Pass a real APK path from your build output.")
        );
        assert_eq!(
            error_hint(&run_command, "docker daemon is unavailable"),
            Some("Start Docker or switch to `--runtime-backend host` for the host emulator path.")
        );
    }

    #[test]
    fn error_hint_uses_command_specific_guidance() {
        assert_eq!(
            error_hint(
                &Command::Doctor(DoctorArgs::default()),
                "some other environment failure"
            ),
            Some(
                "Review the failing checks above and rerun the command once the environment is fixed."
            )
        );
        assert_eq!(
            error_hint(
                &Command::Profile(ProfileArgs {
                    command: ProfileCommand::List,
                }),
                "unknown profile"
            ),
            Some("Use `--config <path>` to target a different config file if needed.")
        );
    }
}
