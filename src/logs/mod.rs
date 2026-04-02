use std::collections::VecDeque;

use anyhow::{anyhow, Result};
use bollard::{
    container::LogsOptions,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
};
use futures_util::StreamExt;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    select,
    sync::mpsc,
    time::{sleep, Duration},
};

use crate::{cli::LogSource, config::RuntimeConfig, host::resolve_host_tool, runtime::Runtime};

#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub source: LogSource,
    pub duration_secs: Option<u64>,
    pub package_name: Option<String>,
}

pub async fn stream(
    runtime: &Runtime,
    config: &RuntimeConfig,
    options: StreamOptions,
) -> Result<()> {
    let (crash_tx, mut crash_rx) = mpsc::unbounded_channel::<String>();
    let mut tasks = Vec::new();

    if matches!(options.source, LogSource::Container | LogSource::Both) {
        match runtime {
            Runtime::Docker(docker) => {
                let client = docker.client().clone();
                let container_name = config.container_name.clone();
                tasks.push(tokio::spawn(async move {
                    let mut output = client.logs(
                        &container_name,
                        Some(LogsOptions::<String> {
                            follow: true,
                            stdout: true,
                            stderr: true,
                            since: 0,
                            until: 0,
                            timestamps: true,
                            tail: "50".to_owned(),
                        }),
                    );

                    while let Some(chunk) = output.next().await {
                        let chunk = chunk?;
                        print_prefixed("container", &chunk.to_string());
                    }

                    Ok::<(), anyhow::Error>(())
                }));
            }
            Runtime::Host(host) => {
                let log_path = host.log_path(config);
                if log_path.exists() {
                    tasks.push(tokio::spawn(async move {
                        let mut command = Command::new("tail");
                        command.args(["-n", "50", "-F"]);
                        command.arg(&log_path);
                        command.stdout(std::process::Stdio::piped());
                        command.stderr(std::process::Stdio::null());
                        command.kill_on_drop(true);

                        let mut child = command.spawn().map_err(anyhow::Error::from)?;

                        let stdout = child
                            .stdout
                            .take()
                            .ok_or_else(|| anyhow!("tail did not expose stdout"))?;
                        read_prefixed_lines("host", stdout, None, None).await?;
                        Ok::<(), anyhow::Error>(())
                    }));
                } else {
                    eprintln!(
                        "host emulator process log is unavailable because this emulator is not managed by rustdroid"
                    );
                }
            }
        }
    }

    if matches!(options.source, LogSource::Logcat | LogSource::Both) {
        match runtime {
            Runtime::Docker(docker) => {
                let client = docker.client().clone();
                let container_name = config.container_name.clone();
                let adb_serial = config.adb_serial.clone();
                let filters = config.logcat_filters.clone();
                let package_name = options.package_name.clone();
                let crash_tx = crash_tx.clone();
                let runtime = runtime.clone();
                let config = config.clone();

                tasks.push(tokio::spawn(async move {
                    let mut command = adb_command(
                        &adb_serial,
                        vec!["logcat".to_owned(), "-v".to_owned(), "time".to_owned()],
                    );
                    if let Some(package_name) = package_name.as_deref() {
                        if let Some(pid) =
                            resolve_app_pid(&runtime, &config, &adb_serial, package_name).await?
                        {
                            command.push(format!("--pid={pid}"));
                        }
                    }
                    command.extend(filters);

                    let exec = client
                        .create_exec(
                            &container_name,
                            CreateExecOptions {
                                attach_stdout: Some(true),
                                attach_stderr: Some(true),
                                cmd: Some(command),
                                ..Default::default()
                            },
                        )
                        .await?;

                    let results = client
                        .start_exec(&exec.id, None::<StartExecOptions>)
                        .await?;

                    match results {
                        StartExecResults::Attached { mut output, .. } => {
                            while let Some(chunk) = output.next().await {
                                let chunk = chunk?;
                                let text = chunk.to_string();
                                print_prefixed("logcat", &text);

                                if let Some(reason) = detect_crash(&text, package_name.as_deref()) {
                                    let _ = crash_tx.send(reason);
                                }
                            }
                        }
                        StartExecResults::Detached => {
                            return Err(anyhow!("unexpected detached exec when starting logcat"));
                        }
                    }

                    Ok::<(), anyhow::Error>(())
                }));
            }
            Runtime::Host(_) => {
                let adb_serial = config.adb_serial.clone();
                let filters = config.logcat_filters.clone();
                let package_name = options.package_name.clone();
                let crash_tx = crash_tx.clone();
                let runtime = runtime.clone();
                let config = config.clone();

                tasks.push(tokio::spawn(async move {
                    let adb_binary = resolve_host_tool("adb")?;
                    let mut command = Command::new(adb_binary);
                    command.args(["-s", &adb_serial, "logcat", "-v", "time"]);

                    if let Some(package_name) = package_name.as_deref() {
                        if let Some(pid) =
                            resolve_app_pid(&runtime, &config, &adb_serial, package_name).await?
                        {
                            command.arg(format!("--pid={pid}"));
                        }
                    }

                    command.args(filters);
                    command.stdout(std::process::Stdio::piped());
                    command.stderr(std::process::Stdio::null());
                    command.kill_on_drop(true);

                    let mut child = command.spawn().map_err(anyhow::Error::from)?;
                    let stdout = child
                        .stdout
                        .take()
                        .ok_or_else(|| anyhow!("adb logcat did not expose stdout"))?;
                    read_prefixed_lines("logcat", stdout, Some(crash_tx), package_name.as_deref())
                        .await?;
                    Ok::<(), anyhow::Error>(())
                }));
            }
        }
    }

    drop(crash_tx);

    let result = match options.duration_secs {
        Some(duration_secs) => {
            select! {
                _ = tokio::signal::ctrl_c() => Ok(()),
                Some(reason) = crash_rx.recv() => Err(anyhow!("crash detected: {reason}")),
                _ = sleep(Duration::from_secs(duration_secs)) => Ok(()),
            }
        }
        None => {
            select! {
                _ = tokio::signal::ctrl_c() => Ok(()),
                Some(reason) = crash_rx.recv() => Err(anyhow!("crash detected: {reason}")),
            }
        }
    };

    for task in tasks {
        task.abort();
        let _ = task.await;
    }

    result
}

fn adb_command(serial: &str, mut args: Vec<String>) -> Vec<String> {
    let mut command = vec!["adb".to_owned(), "-s".to_owned(), serial.to_owned()];
    command.append(&mut args);
    command
}

async fn read_prefixed_lines<R>(
    prefix: &str,
    reader: R,
    crash_tx: Option<mpsc::UnboundedSender<String>>,
    package_name: Option<&str>,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut recent_lines = VecDeque::with_capacity(6);
    while let Some(line) = lines.next_line().await? {
        print_prefixed(prefix, &line);
        recent_lines.push_back(line);
        while recent_lines.len() > 6 {
            recent_lines.pop_front();
        }

        if let (Some(crash_tx), Some(reason)) = (
            crash_tx.as_ref(),
            detect_crash(
                &recent_lines
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("\n"),
                package_name,
            ),
        ) {
            let _ = crash_tx.send(reason);
        }
    }

    Ok(())
}

fn print_prefixed(prefix: &str, chunk: &str) {
    for line in chunk.lines() {
        println!("[{prefix}] {line}");
    }
}

fn detect_crash(chunk: &str, package_name: Option<&str>) -> Option<String> {
    let lines: Vec<&str> = chunk.lines().collect();

    for (index, line) in lines.iter().enumerate() {
        let lowercase = line.to_ascii_lowercase();

        if lowercase.contains("fatal exception") {
            if let Some(package_name) = package_name {
                if lines[index..]
                    .iter()
                    .take(6)
                    .any(|candidate| candidate.contains(&format!("Process: {package_name},")))
                {
                    return Some(format!("fatal exception in {package_name}"));
                }
            } else {
                return Some("fatal exception".to_owned());
            }
        }

        if let Some(package_name) = package_name {
            if lowercase.contains("anr in ") && line.contains(package_name) {
                return Some(format!("anr in {package_name}"));
            }
            if lowercase.contains("has died") && line.contains(package_name) {
                return Some(format!("process for {package_name} died"));
            }
        }
    }

    None
}

async fn resolve_app_pid(
    runtime: &Runtime,
    config: &RuntimeConfig,
    adb_serial: &str,
    package_name: &str,
) -> Result<Option<String>> {
    for _ in 0..10 {
        let outcome = runtime
            .exec(
                config,
                adb_command(
                    adb_serial,
                    vec![
                        "shell".to_owned(),
                        "pidof".to_owned(),
                        "-s".to_owned(),
                        package_name.to_owned(),
                    ],
                ),
            )
            .await?;

        let pid = outcome.stdout.trim();
        if outcome.exit_code == 0 && !pid.is_empty() {
            return Ok(Some(pid.to_owned()));
        }

        sleep(Duration::from_secs(1)).await;
    }

    Ok(None)
}
