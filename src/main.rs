mod adb;
mod cli;
mod config;
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
    let config = RuntimeConfig::load(&cli)?;
    let runtime = Runtime::connect(&config)?;
    let orchestrator = EmulatorOrchestrator::new(config, runtime);

    match cli.command {
        Command::Start(args) => orchestrator.start(args).await?,
        Command::Install(args) => orchestrator.install(args).await?,
        Command::Run(args) => orchestrator.run(args).await?,
        Command::Logs(args) => orchestrator.logs(args).await?,
        Command::Stop(args) => orchestrator.stop(args).await?,
    }

    Ok(())
}
