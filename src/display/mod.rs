use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use tokio::{net::TcpStream, process::Command, time::sleep};

use crate::config::RuntimeConfig;

const SCRCPY_WAIT_TIMEOUT_SECS: u64 = 30;

pub async fn launch_if_needed(config: &RuntimeConfig) -> Result<()> {
    if config.headless || !config.uses_scrcpy_ui() {
        return Ok(());
    }

    let serial = adb_target_serial(config);

    if scrcpy_session_alive(config)? {
        eprintln!("reusing existing scrcpy window for {serial}");
        return Ok(());
    }

    if config.uses_host_runtime() {
        wait_for_host_adb_device(&serial, SCRCPY_WAIT_TIMEOUT_SECS, false).await?;
    } else {
        wait_for_adb_bridge(config.adb_connect_port, SCRCPY_WAIT_TIMEOUT_SECS).await?;
        connect_host_adb(&serial).await?;
        wait_for_host_adb_device(&serial, SCRCPY_WAIT_TIMEOUT_SECS, true).await?;
    }
    spawn_scrcpy(config, &serial).await?;
    Ok(())
}

fn adb_target_serial(config: &RuntimeConfig) -> String {
    if config.uses_host_runtime() {
        config.adb_serial.clone()
    } else {
        format!("127.0.0.1:{}", config.adb_connect_port)
    }
}

async fn wait_for_adb_bridge(port: u16, timeout_secs: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for host ADB bridge on 127.0.0.1:{port}");
        }

        sleep(Duration::from_millis(500)).await;
    }
}

async fn connect_host_adb(serial: &str) -> Result<()> {
    let output = Command::new("adb")
        .arg("connect")
        .arg(serial)
        .output()
        .await
        .with_context(|| format!("failed to run host adb for {serial}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    if output.status.success()
        || combined.contains("connected to")
        || combined.contains("already connected to")
    {
        return Ok(());
    }

    bail!(
        "failed to connect host adb to {serial} (stdout='{}', stderr='{}')",
        stdout.trim(),
        stderr.trim()
    );
}

async fn wait_for_host_adb_device(serial: &str, timeout_secs: u64, reconnect: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        let output = Command::new("adb")
            .args(["-s", serial, "get-state"])
            .output()
            .await
            .with_context(|| format!("failed to query host adb state for {serial}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() && stdout.trim() == "device" {
            return Ok(());
        }

        if Instant::now() >= deadline {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "timed out waiting for host adb device {} (stdout='{}', stderr='{}')",
                serial,
                stdout.trim(),
                stderr.trim()
            );
        }

        if reconnect {
            let _ = Command::new("adb")
                .arg("disconnect")
                .arg(serial)
                .output()
                .await;
            let _ = connect_host_adb(serial).await;
        }
        sleep(Duration::from_millis(500)).await;
    }
}

async fn spawn_scrcpy(config: &RuntimeConfig, serial: &str) -> Result<()> {
    let window_title = format!("RustDroid {}", config.container_name);
    let mut command = StdCommand::new("scrcpy");
    let max_fps = config.scrcpy_max_fps.to_string();
    let max_size = config.scrcpy_max_size.to_string();

    command
        .args(["--serial", serial])
        .args(["--window-title", &window_title])
        .args(["--max-fps", &max_fps])
        .args(["--max-size", &max_size])
        .args(["--video-bit-rate", &config.scrcpy_video_bit_rate])
        .arg("--no-audio")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let child = command.spawn().with_context(|| {
        "failed to launch scrcpy; install scrcpy or switch to --ui-backend web".to_owned()
    })?;

    write_scrcpy_session_pid(config, Some(child.id()))?;
    eprintln!("launching scrcpy window for {serial}");
    Ok(())
}

fn scrcpy_session_alive(config: &RuntimeConfig) -> Result<bool> {
    let pid_path = scrcpy_session_pid_path(config);
    let pid = match fs::read_to_string(&pid_path) {
        Ok(raw) => raw.trim().parse::<u32>().ok(),
        Err(_) => None,
    };

    let Some(pid) = pid else {
        return Ok(false);
    };

    if Path::new(&format!("/proc/{pid}")).exists() {
        return Ok(true);
    }

    let _ = fs::remove_file(pid_path);
    Ok(false)
}

fn write_scrcpy_session_pid(config: &RuntimeConfig, pid: Option<u32>) -> Result<()> {
    let Some(pid) = pid else {
        return Ok(());
    };

    let pid_path = scrcpy_session_pid_path(config);
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(pid_path, pid.to_string())?;
    Ok(())
}

fn scrcpy_session_pid_path(config: &RuntimeConfig) -> PathBuf {
    std::env::temp_dir()
        .join("rustdroid")
        .join("scrcpy")
        .join(format!("{}.pid", sanitize_name(&config.container_name)))
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
