use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use tokio::{process::Command, time::sleep};

use crate::{config::RuntimeConfig, docker::ExecOutcome};

#[derive(Debug, Clone)]
pub struct HostRuntime;

impl HostRuntime {
    pub fn connect() -> Result<Self> {
        Ok(Self)
    }

    pub async fn ping(&self) -> Result<()> {
        ensure_tool_available("adb")?;
        ensure_tool_available("emulator")?;
        Ok(())
    }

    pub async fn ensure_started(&self, config: &RuntimeConfig) -> Result<()> {
        validate_host_config(config)?;

        let state = HostStatePaths::new(config);
        fs::create_dir_all(&state.dir)?;
        let config_hash = host_config_hash(config);

        if let Some(pid) = read_pid(&state.pid_path)? {
            if process_alive(pid) {
                if read_trimmed(&state.config_hash_path)?.as_deref() == Some(config_hash.as_str()) {
                    eprintln!("reusing managed host emulator {}", config.adb_serial);
                    return Ok(());
                }

                eprintln!(
                    "restarting managed host emulator {} to apply config changes",
                    config.adb_serial
                );
                self.stop(config, 15).await?;
            } else {
                cleanup_state_files(&state)?;
            }
        }

        if adb_device_reachable(&config.adb_serial).await? {
            eprintln!(
                "reusing existing unmanaged host emulator {}",
                config.adb_serial
            );
            return Ok(());
        }

        let avd_name = resolve_avd_name(config).await?;
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&state.log_path)
            .with_context(|| format!("failed to open {}", state.log_path.display()))?;
        let stderr_log_file = log_file
            .try_clone()
            .with_context(|| format!("failed to clone {}", state.log_path.display()))?;
        let emulator_binary = resolve_host_tool(&config.host_emulator_binary)?;
        let mut command = StdCommand::new(&emulator_binary);

        command.args(build_launch_args(config, &avd_name));
        command.stdin(Stdio::null());
        command.stdout(Stdio::from(log_file));
        command.stderr(Stdio::from(stderr_log_file));

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;

            command.process_group(0);
        }

        let child = command.spawn().with_context(|| {
            format!(
                "failed to launch host emulator '{}' using {}",
                avd_name,
                emulator_binary.display()
            )
        })?;

        fs::write(&state.pid_path, child.id().to_string())
            .with_context(|| format!("failed to write {}", state.pid_path.display()))?;
        fs::write(&state.config_hash_path, config_hash)
            .with_context(|| format!("failed to write {}", state.config_hash_path.display()))?;
        fs::write(&state.avd_name_path, avd_name.as_bytes())
            .with_context(|| format!("failed to write {}", state.avd_name_path.display()))?;

        eprintln!(
            "launching host emulator '{}' on {} (logs: {})",
            avd_name,
            config.adb_serial,
            state.log_path.display()
        );
        Ok(())
    }

    pub async fn stop(&self, config: &RuntimeConfig, timeout_secs: u64) -> Result<()> {
        let state = HostStatePaths::new(config);
        let Some(pid) = read_pid(&state.pid_path)? else {
            if adb_device_reachable(&config.adb_serial).await? {
                eprintln!(
                    "host emulator {} is not managed by rustdroid; leaving it running",
                    config.adb_serial
                );
            }
            cleanup_state_files(&state)?;
            return Ok(());
        };

        if !process_alive(pid) {
            cleanup_state_files(&state)?;
            return Ok(());
        }

        let _ = self
            .exec(
                config,
                vec![
                    "adb".to_owned(),
                    "-s".to_owned(),
                    config.adb_serial.clone(),
                    "emu".to_owned(),
                    "kill".to_owned(),
                ],
            )
            .await;

        wait_for_process_exit(pid, timeout_secs).await;

        if process_alive(pid) {
            terminate_process(pid, "TERM")?;
            wait_for_process_exit(pid, 3).await;
        }

        if process_alive(pid) {
            terminate_process(pid, "KILL")?;
            wait_for_process_exit(pid, 2).await;
        }

        cleanup_state_files(&state)?;
        Ok(())
    }

    pub async fn exec(&self, _config: &RuntimeConfig, command: Vec<String>) -> Result<ExecOutcome> {
        let Some(program) = command.first() else {
            bail!("cannot execute an empty host command");
        };
        let program_path = resolve_host_tool(program)?;
        let output = Command::new(&program_path)
            .args(command.iter().skip(1))
            .output()
            .await
            .with_context(|| {
                format!(
                    "failed to run host command {:?} using {}",
                    command,
                    program_path.display()
                )
            })?;

        Ok(ExecOutcome {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(1).into(),
        })
    }

    pub async fn upload_file(
        &self,
        _config: &RuntimeConfig,
        local_path: &Path,
        _remote_dir: &str,
        _remote_name: &str,
    ) -> Result<String> {
        let canonical = fs::canonicalize(local_path)
            .with_context(|| format!("failed to resolve {}", local_path.display()))?;
        Ok(canonical.display().to_string())
    }

    pub fn log_path(&self, config: &RuntimeConfig) -> PathBuf {
        HostStatePaths::new(config).log_path
    }
}

pub(crate) fn managed_process_running(config: &RuntimeConfig) -> Result<Option<bool>> {
    let state = HostStatePaths::new(config);
    let Some(pid) = read_pid(&state.pid_path)? else {
        return Ok(None);
    };

    Ok(Some(process_alive(pid)))
}

pub(crate) fn managed_log_path(config: &RuntimeConfig) -> PathBuf {
    HostStatePaths::new(config).log_path
}

#[derive(Debug, Clone)]
struct HostStatePaths {
    dir: PathBuf,
    pid_path: PathBuf,
    config_hash_path: PathBuf,
    avd_name_path: PathBuf,
    log_path: PathBuf,
}

impl HostStatePaths {
    fn new(config: &RuntimeConfig) -> Self {
        let dir = std::env::temp_dir()
            .join("rustdroid")
            .join("host")
            .join(sanitize_name(&config.container_name));

        Self {
            pid_path: dir.join("emulator.pid"),
            config_hash_path: dir.join("config.hash"),
            avd_name_path: dir.join("avd.name"),
            log_path: dir.join("emulator.log"),
            dir,
        }
    }
}

fn validate_host_config(config: &RuntimeConfig) -> Result<()> {
    if config.uses_web_ui() || config.uses_vnc_ui() {
        bail!(
            "host runtime only supports scrcpy or headless mode; web/vnc UI still requires the Docker backend"
        );
    }

    Ok(())
}

fn build_launch_args(config: &RuntimeConfig, avd_name: &str) -> Vec<String> {
    let mut args = vec![
        "-avd".to_owned(),
        avd_name.to_owned(),
        "-ports".to_owned(),
        format!(
            "{},{}",
            config.host_emulator_port,
            config.host_emulator_port + 1
        ),
        "-memory".to_owned(),
        config.emulator_ram_mb.to_string(),
        "-cores".to_owned(),
        config.emulator_cpu_cores.to_string(),
        "-accel".to_owned(),
        "auto".to_owned(),
        "-gpu".to_owned(),
        config.emulator_gpu_mode.clone(),
    ];
    args.extend(
        config
            .effective_emulator_additional_args()
            .split_whitespace()
            .map(str::to_owned),
    );
    args
}

pub(crate) async fn resolve_avd_name(config: &RuntimeConfig) -> Result<String> {
    if let Some(avd_name) = config.host_avd_name.as_ref() {
        return Ok(avd_name.clone());
    }

    let avd_name = list_host_avds(&config.host_emulator_binary)
        .await?
        .into_iter()
        .next();

    avd_name.ok_or_else(|| {
        anyhow::anyhow!(
            "no host AVD found; create one in Android Studio or pass --host-avd-name explicitly"
        )
    })
}

pub(crate) async fn list_host_avds(emulator_binary: &str) -> Result<Vec<String>> {
    let emulator_binary = resolve_host_tool(emulator_binary)?;
    let output = Command::new(emulator_binary)
        .arg("-list-avds")
        .output()
        .await
        .context("failed to list Android Virtual Devices with the host emulator")?;

    if !output.status.success() {
        bail!(
            "failed to list host AVDs (stderr='{}')",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

async fn adb_device_reachable(serial: &str) -> Result<bool> {
    let adb_binary = resolve_host_tool("adb")?;
    let output = Command::new(adb_binary)
        .args(["-s", serial, "get-state"])
        .output()
        .await
        .with_context(|| format!("failed to query adb state for {serial}"))?;

    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "device")
}

async fn wait_for_process_exit(pid: u32, timeout_secs: u64) {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while process_alive(pid) && Instant::now() < deadline {
        sleep(Duration::from_millis(500)).await;
    }
}

fn terminate_process(pid: u32, signal: &str) -> Result<()> {
    let status = StdCommand::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .status()
        .with_context(|| format!("failed to send {signal} to process {pid}"))?;

    if status.success() {
        Ok(())
    } else {
        bail!("kill {signal} {pid} exited with status {status}");
    }
}

fn host_config_hash(config: &RuntimeConfig) -> String {
    let mut hasher = DefaultHasher::new();
    config.runtime_backend.hash(&mut hasher);
    config.host_avd_name.hash(&mut hasher);
    config.host_emulator_binary.hash(&mut hasher);
    config.host_emulator_port.hash(&mut hasher);
    config.adb_serial.hash(&mut hasher);
    config.emulator_ram_mb.hash(&mut hasher);
    config.emulator_cpu_cores.hash(&mut hasher);
    config.emulator_gpu_mode.hash(&mut hasher);
    config
        .effective_emulator_additional_args()
        .hash(&mut hasher);
    config.headless.hash(&mut hasher);
    config.ui_backend.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn cleanup_state_files(state: &HostStatePaths) -> Result<()> {
    for path in [
        &state.pid_path,
        &state.config_hash_path,
        &state.avd_name_path,
    ] {
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }

    Ok(())
}

fn read_pid(path: &Path) -> Result<Option<u32>> {
    Ok(read_trimmed(path)?.and_then(|value| value.parse::<u32>().ok()))
}

fn read_trimmed(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents.trim().to_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn ensure_tool_available(program: &str) -> Result<()> {
    let _ = resolve_host_tool(program)?;
    Ok(())
}

pub(crate) fn resolve_host_tool(program: &str) -> Result<PathBuf> {
    let program_path = Path::new(program);
    if program_path.components().count() > 1 || program_path.is_absolute() {
        if program_path.exists() {
            return Ok(program_path.to_path_buf());
        }

        bail!("host tool '{}' does not exist", program);
    }

    if let Some(path) = find_in_path(program) {
        return Ok(path);
    }

    let Some(sdk_root) = android_sdk_root() else {
        bail!(
            "required host tool '{}' was not found on PATH and no Android SDK root was detected",
            program
        );
    };

    let candidate = match program {
        "emulator" => Some(sdk_root.join("emulator").join("emulator")),
        "adb" => Some(sdk_root.join("platform-tools").join("adb")),
        "aapt" => find_latest_sdk_tool(&sdk_root.join("build-tools"), "aapt"),
        "apkanalyzer" => find_latest_sdk_tool(&sdk_root.join("cmdline-tools"), "bin/apkanalyzer")
            .or_else(|| find_latest_sdk_tool(&sdk_root.join("tools"), "bin/apkanalyzer")),
        _ => None,
    };

    match candidate.filter(|path| path.exists()) {
        Some(path) => Ok(path),
        None => bail!(
            "required host tool '{}' was not found on PATH or inside {}",
            program,
            sdk_root.display()
        ),
    }
}

fn find_in_path(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(program))
            .find(|path| path.exists())
    })
}

pub(crate) fn android_sdk_root() -> Option<PathBuf> {
    let home_sdk = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Android").join("Sdk"));

    [
        std::env::var_os("ANDROID_HOME"),
        std::env::var_os("ANDROID_SDK_ROOT"),
        Some("/usr/local/android-sdk".into()),
        home_sdk.map(|path| path.into_os_string()),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    .find(|path| path.exists())
}

fn find_latest_sdk_tool(root: &Path, suffix: &str) -> Option<PathBuf> {
    let mut entries: Vec<_> = fs::read_dir(root)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();
    entries.reverse();

    entries
        .into_iter()
        .map(|path| path.join(suffix))
        .find(|path| path.exists())
}
