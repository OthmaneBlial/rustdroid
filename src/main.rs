mod adb;
mod cli;
mod config;
mod diagnostics;
mod display;
mod docker;
mod emulator;
mod host;
mod logs;
mod output;
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
                Command::Run(args) => orchestrator.run(args).await?,
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
