use std::path::Path;

use anyhow::Result;

use crate::{
    adb::AdbClient,
    cli::{InstallArgs, LogsArgs, RunArgs, StartArgs, StopArgs},
    config::RuntimeConfig,
    display,
    logs::{self, StreamOptions},
    runtime::Runtime,
};

#[derive(Debug, Clone)]
pub struct EmulatorOrchestrator {
    config: RuntimeConfig,
    runtime: Runtime,
    adb: AdbClient,
}

impl EmulatorOrchestrator {
    pub fn new(config: RuntimeConfig, runtime: Runtime) -> Self {
        let adb = AdbClient::new(
            config.adb_serial.clone(),
            config.disable_animations,
            config.optimize_android_runtime,
            config.device_width_px,
            config.device_height_px,
            config.device_density_dpi,
            config.compile_installed_package,
            config.disable_preinstalled_packages,
            config.disable_google_play_services,
        );
        Self {
            config,
            runtime,
            adb,
        }
    }

    pub async fn start(&self, args: StartArgs) -> Result<()> {
        self.start_device(args.wait, true).await
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

    pub async fn install(&self, args: InstallArgs) -> Result<()> {
        self.require_apk(&args.apk)?;
        self.start_device(true, false).await?;
        self.install_uploaded_apk(&args.apk, args.replace).await?;
        Ok(())
    }

    pub async fn run(&self, args: RunArgs) -> Result<()> {
        self.require_apk(&args.apk)?;
        self.start_device(true, true).await?;
        let metadata = self.install_uploaded_apk(&args.apk, args.replace).await?;
        eprintln!("launching {}", metadata.package_name);
        self.adb
            .launch_app(&self.runtime, &self.config, &metadata)
            .await?;
        logs::stream(
            &self.runtime,
            &self.config,
            StreamOptions {
                source: args.log_source,
                duration_secs: args.duration_secs,
                package_name: Some(metadata.package_name),
            },
        )
        .await
    }

    pub async fn logs(&self, args: LogsArgs) -> Result<()> {
        self.start_device(false, false).await?;
        eprintln!("streaming logs");
        logs::stream(
            &self.runtime,
            &self.config,
            StreamOptions {
                source: args.source,
                duration_secs: args.duration_secs,
                package_name: None,
            },
        )
        .await
    }

    pub async fn stop(&self, args: StopArgs) -> Result<()> {
        self.runtime.stop(&self.config, args.timeout_secs).await
    }

    async fn install_uploaded_apk(
        &self,
        apk_path: &Path,
        replace: bool,
    ) -> Result<crate::adb::ApkMetadata> {
        eprintln!("uploading {}", apk_path.display());
        let remote_path = self
            .runtime
            .upload_file(
                &self.config,
                apk_path,
                &self.config.remote_apk_dir,
                &self.config.remote_apk_name,
            )
            .await?;
        eprintln!("inspecting uploaded APK");
        let metadata = self
            .adb
            .inspect_apk(&self.runtime, &self.config, &remote_path)
            .await?;
        if metadata.uses_arm_translation_on_x86_emulator() {
            eprintln!(
                "warning: APK ships ARM-only native libraries, so the x86_64 emulator must use ARM translation and may stay slower than a native x86/x86_64 build"
            );
        }
        eprintln!("installing {}", metadata.package_name);
        self.adb
            .install_apk(&self.runtime, &self.config, &remote_path, replace)
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
        Ok(metadata)
    }

    fn require_apk(&self, apk_path: &Path) -> Result<()> {
        anyhow::ensure!(apk_path.is_file(), "APK not found: {}", apk_path.display());
        let _ = &self.adb;
        Ok(())
    }
}
