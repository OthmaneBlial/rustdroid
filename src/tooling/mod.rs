use std::{fs, io::ErrorKind, path::Path, process::Command as StdCommand, thread, time::Duration};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::{
    cli::{ConfigInitArgs, RuntimeBackend},
    config::RuntimeConfig,
    docker::DockerRuntime,
    output::print_json,
    profiles::{apply_named_profile, built_in_profiles},
};

#[derive(Debug, Clone, Serialize)]
struct ConfigWriteReport {
    path: String,
    profile: Option<String>,
    runtime_backend: RuntimeBackend,
}

#[derive(Debug, Clone, Serialize)]
struct CleanReport {
    dry_run: bool,
    removed_docker_containers: usize,
    terminated_host_emulators: usize,
    terminated_scrcpy_sessions: usize,
    removed_temp_state: bool,
    warnings: Vec<String>,
}

pub fn list_profiles(json: bool) -> Result<()> {
    let profiles = built_in_profiles();
    if json {
        return print_json(&profiles);
    }

    for profile in profiles {
        println!("{}  {}", profile.name, profile.description);
        println!(
            "  backend={} ui={:?} headless={} ram={}MB gpu={}",
            format_backend(profile.config.runtime_backend),
            profile.config.ui_backend,
            profile.config.headless,
            profile.config.emulator_ram_mb,
            profile.config.emulator_gpu_mode
        );
    }

    Ok(())
}

pub fn init_config(config_path: &Path, args: &ConfigInitArgs, json: bool) -> Result<()> {
    if config_path.exists() && !args.force {
        bail!(
            "config file already exists at {}; pass --force to overwrite it",
            config_path.display()
        );
    }

    let mut config = RuntimeConfig::default();
    if let Some(profile) = args.profile.as_deref() {
        apply_named_profile(&mut config, profile)?;
    }

    write_config(config_path, &config)?;
    let report = ConfigWriteReport {
        path: config_path.display().to_string(),
        profile: args.profile.clone(),
        runtime_backend: config.runtime_backend,
    };

    if json {
        return print_json(&report);
    }

    println!("wrote {}", report.path);
    if let Some(profile) = report.profile {
        println!("profile: {profile}");
    }
    println!("backend: {}", format_backend(report.runtime_backend));
    Ok(())
}

pub fn use_profile(config_path: &Path, profile_name: &str, force: bool, json: bool) -> Result<()> {
    let mut config = if config_path.exists() && !force {
        RuntimeConfig::from_path(config_path)?
    } else {
        RuntimeConfig::default()
    };

    apply_named_profile(&mut config, profile_name)?;
    write_config(config_path, &config)?;

    let report = ConfigWriteReport {
        path: config_path.display().to_string(),
        profile: Some(profile_name.to_owned()),
        runtime_backend: config.runtime_backend,
    };

    if json {
        return print_json(&report);
    }

    println!("updated {}", report.path);
    println!("profile: {}", profile_name);
    println!("backend: {}", format_backend(report.runtime_backend));
    Ok(())
}

pub async fn clean(dry_run: bool, json: bool) -> Result<()> {
    let mut warnings = Vec::new();
    let mut removed_docker_containers = 0;

    match DockerRuntime::connect() {
        Ok(runtime) => match runtime.list_managed_container_names().await {
            Ok(container_names) => {
                if !dry_run {
                    for name in &container_names {
                        runtime.remove_container_force(name).await?;
                    }
                }
                removed_docker_containers = container_names.len();
            }
            Err(error) => warnings.push(format!("failed to inspect Docker containers: {error}")),
        },
        Err(error) => warnings.push(format!("Docker client unavailable during clean: {error}")),
    }

    let state_root = std::env::temp_dir().join("rustdroid");
    let terminated_host_emulators =
        terminate_pid_files(&state_root.join("host"), "host emulator", dry_run)?;
    let terminated_scrcpy_sessions =
        terminate_pid_files(&state_root.join("scrcpy"), "scrcpy", dry_run)?;

    let removed_temp_state = if dry_run {
        state_root.exists()
    } else {
        match fs::remove_dir_all(&state_root) {
            Ok(()) => true,
            Err(error) if error.kind() == ErrorKind::NotFound => false,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to remove {}", state_root.display()))
            }
        }
    };

    let report = CleanReport {
        dry_run,
        removed_docker_containers,
        terminated_host_emulators,
        terminated_scrcpy_sessions,
        removed_temp_state,
        warnings,
    };

    if json {
        return print_json(&report);
    }

    println!(
        "removed {} Docker container(s), terminated {} host emulator(s), terminated {} scrcpy session(s)",
        report.removed_docker_containers,
        report.terminated_host_emulators,
        report.terminated_scrcpy_sessions
    );
    if report.dry_run {
        println!("dry-run: no containers or processes were actually removed");
    }
    if report.removed_temp_state {
        println!("removed {}", state_root.display());
    }
    for warning in report.warnings {
        println!("warning: {warning}");
    }

    Ok(())
}

pub async fn stop_all(timeout_secs: u64) -> Result<()> {
    if let Ok(runtime) = DockerRuntime::connect() {
        if let Ok(container_names) = runtime.list_managed_container_names().await {
            for name in &container_names {
                let _ = runtime.stop(name, timeout_secs).await;
            }
        }
    }

    let state_root = std::env::temp_dir().join("rustdroid");
    let _ = terminate_pid_files(&state_root.join("host"), "host emulator", false)?;
    let _ = terminate_pid_files(&state_root.join("scrcpy"), "scrcpy", false)?;
    let _ = fs::remove_dir_all(&state_root);
    Ok(())
}

fn write_config(path: &Path, config: &RuntimeConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let contents = toml::to_string_pretty(config)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn terminate_pid_files(root: &Path, label: &str, dry_run: bool) -> Result<usize> {
    let mut terminated = 0;

    if !root.exists() {
        return Ok(terminated);
    }

    for pid_path in walk_pid_files(root)? {
        let raw_pid = match fs::read_to_string(&pid_path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", pid_path.display()))
            }
        };

        let Some(pid) = raw_pid.trim().parse::<u32>().ok() else {
            continue;
        };

        if !process_alive(pid) {
            continue;
        }

        if !dry_run {
            send_signal(pid, "TERM")
                .with_context(|| format!("failed to stop {label} process {pid}"))?;
            thread::sleep(Duration::from_millis(500));
            if process_alive(pid) {
                send_signal(pid, "KILL")
                    .with_context(|| format!("failed to kill {label} process {pid}"))?;
            }
        }
        terminated += 1;
    }

    Ok(terminated)
}

fn walk_pid_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            paths.extend(walk_pid_files(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "pid") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn send_signal(pid: u32, signal: &str) -> Result<()> {
    let status = StdCommand::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .status()
        .with_context(|| format!("failed to send {signal} to process {pid}"))?;

    if status.success() {
        Ok(())
    } else {
        bail!("kill {signal} {pid} exited with status {status}")
    }
}

fn format_backend(backend: RuntimeBackend) -> &'static str {
    match backend {
        RuntimeBackend::Docker => "docker",
        RuntimeBackend::Host => "host",
    }
}
