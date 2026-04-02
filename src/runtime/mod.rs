use std::path::Path;

use anyhow::Result;

use crate::{
    config::RuntimeConfig,
    docker::{DockerRuntime, ExecOutcome},
    host::HostRuntime,
};

#[derive(Debug, Clone)]
pub enum Runtime {
    Docker(DockerRuntime),
    Host(HostRuntime),
}

impl Runtime {
    pub fn connect(config: &RuntimeConfig) -> Result<Self> {
        match config.runtime_backend {
            crate::cli::RuntimeBackend::Docker => Ok(Self::Docker(DockerRuntime::connect()?)),
            crate::cli::RuntimeBackend::Host => Ok(Self::Host(HostRuntime::connect()?)),
        }
    }

    pub async fn ping(&self) -> Result<()> {
        match self {
            Self::Docker(runtime) => runtime.ping().await,
            Self::Host(runtime) => runtime.ping().await,
        }
    }

    pub async fn ensure_started(&self, config: &RuntimeConfig) -> Result<()> {
        match self {
            Self::Docker(runtime) => runtime.ensure_started(config).await,
            Self::Host(runtime) => runtime.ensure_started(config).await,
        }
    }

    pub async fn stop(&self, config: &RuntimeConfig, timeout_secs: u64) -> Result<()> {
        match self {
            Self::Docker(runtime) => runtime.stop(&config.container_name, timeout_secs).await,
            Self::Host(runtime) => runtime.stop(config, timeout_secs).await,
        }
    }

    pub async fn exec(&self, config: &RuntimeConfig, command: Vec<String>) -> Result<ExecOutcome> {
        match self {
            Self::Docker(runtime) => runtime.exec(&config.container_name, command).await,
            Self::Host(runtime) => runtime.exec(config, command).await,
        }
    }

    pub async fn upload_file(
        &self,
        config: &RuntimeConfig,
        local_path: &Path,
        remote_dir: &str,
        remote_name: &str,
    ) -> Result<String> {
        match self {
            Self::Docker(runtime) => {
                runtime
                    .upload_file(&config.container_name, local_path, remote_dir, remote_name)
                    .await
            }
            Self::Host(runtime) => {
                runtime
                    .upload_file(config, local_path, remote_dir, remote_name)
                    .await
            }
        }
    }
}
