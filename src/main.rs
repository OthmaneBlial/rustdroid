mod adb;
mod cli;
mod config;
mod diagnostics;
mod display;
mod docker;
mod emulator;
mod host;
mod logs;
mod runtime;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use config::RuntimeConfig;
use emulator::EmulatorOrchestrator;
use runtime::Runtime;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.clone();

    match command {
        Command::Version => diagnostics::print_version(),
        Command::Completions(args) => diagnostics::print_completions(args.shell),
        Command::Devices(_args) => diagnostics::run_devices().await?,
        Command::Doctor(_args) => {
            let config = RuntimeConfig::load(&cli)?;
            diagnostics::run_doctor(&config).await?;
        }
        Command::Avds(_args) => {
            let config = RuntimeConfig::load(&cli)?;
            diagnostics::run_avds(&config).await?;
        }
        Command::SelfTest(args) => {
            let config = RuntimeConfig::load(&cli)?;
            diagnostics::run_self_test(&config, &args).await?;
        }
        command => {
            let config = RuntimeConfig::load(&cli)?;
            let runtime = Runtime::connect(&config)?;
            let orchestrator = EmulatorOrchestrator::new(config, runtime);

            match command {
                Command::Start(args) => orchestrator.start(args).await?,
                Command::Install(args) => orchestrator.install(args).await?,
                Command::Run(args) => orchestrator.run(args).await?,
                Command::Logs(args) => orchestrator.logs(args).await?,
                Command::Stop(args) => orchestrator.stop(args).await?,
                _ => unreachable!("non-runtime commands are handled above"),
            }
        }
    }

    Ok(())
}
