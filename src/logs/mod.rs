use std::collections::VecDeque;

use anyhow::{anyhow, bail, Context, Result};
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
    task::JoinSet,
    time::{sleep, timeout, Duration},
};

use crate::{cli::LogSource, config::RuntimeConfig, host::resolve_host_tool, runtime::Runtime};

#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub source: LogSource,
    pub duration_secs: Option<u64>,
    pub package_name: Option<String>,
    pub since_start: bool,
    pub verify_liveness: bool,
    pub run_marker: Option<String>,
}

pub async fn stream(
    runtime: &Runtime,
    config: &RuntimeConfig,
    options: StreamOptions,
) -> Result<()> {
    let (crash_tx, mut crash_rx) = mpsc::unbounded_channel::<String>();
    let (ready_tx, ready_rx) = mpsc::unbounded_channel();
    let mut tasks = JoinSet::new();
    // Resolve before starting the observation clock, not inside a detached reader.
    anyhow::ensure!(
        !options.verify_liveness || options.duration_secs != Some(0),
        "application observation duration must be greater than zero"
    );
    let app_pid = if let Some(package) = options.package_name.as_deref() {
        if options.verify_liveness {
            Some(require_app_pid(runtime, config, package).await?)
        } else {
            resolve_app_pid(runtime, config, &config.adb_serial, package).await?
        }
    } else {
        None
    };

    if matches!(options.source, LogSource::Container | LogSource::Both) {
        match runtime {
            Runtime::Docker(docker) => {
                let client = docker.client().clone();
                let container_name = config.container_name.clone();
                let ready_tx = ready_tx.clone();
                tasks.spawn(async move {
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

                    let _ = ready_tx.send(());
                    while let Some(chunk) = output.next().await {
                        let chunk = chunk?;
                        print_prefixed("container", &chunk.to_string());
                    }

                    Ok::<(), anyhow::Error>(())
                });
            }
            Runtime::Host(host) => {
                let log_path = host.log_path(config);
                if log_path.exists() {
                    let since_start = options.since_start;
                    let ready_tx = ready_tx.clone();
                    tasks.spawn(async move {
                        let mut command = Command::new("tail");
                        if since_start {
                            command.args(["-n", "+1", "-F"]);
                        } else {
                            command.args(["-n", "50", "-F"]);
                        }
                        command.arg(&log_path);
                        command.stdout(std::process::Stdio::piped());
                        command.stderr(std::process::Stdio::null());
                        command.kill_on_drop(true);

                        let mut child = command.spawn().map_err(anyhow::Error::from)?;

                        let stdout = child
                            .stdout
                            .take()
                            .ok_or_else(|| anyhow!("tail did not expose stdout"))?;
                        let _ = ready_tx.send(());
                        read_prefixed_lines("host", stdout, None, None, None).await?;
                        Ok::<(), anyhow::Error>(())
                    });
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
                let app_pid = app_pid.clone();
                let ready_tx = ready_tx.clone();
                let marker = options.run_marker.clone();

                tasks.spawn(async move {
                    let mut command = adb_command(
                        &adb_serial,
                        vec!["logcat".to_owned(), "-v".to_owned(), "time".to_owned()],
                    );
                    if let (None, Some(pid)) = (marker.as_ref(), app_pid.as_deref()) {
                        command.push(format!("--pid={pid}"));
                    }
                    command.extend(filters);
                    if marker.is_some() {
                        command.push("RustDroid:I".into());
                    }

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
                            let _ = ready_tx.send(());
                            let mut window = CrashWindow::new(marker);
                            while let Some(chunk) = output.next().await {
                                let chunk = chunk?;
                                let text = chunk.to_string();
                                print_prefixed("logcat", &text);

                                if let Some(reason) = window.feed(&text, package_name.as_deref())? {
                                    let _ = crash_tx.send(reason);
                                }
                            }
                        }
                        StartExecResults::Detached => {
                            return Err(anyhow!("unexpected detached exec when starting logcat"));
                        }
                    }

                    Ok::<(), anyhow::Error>(())
                });
            }
            Runtime::Host(_) => {
                let adb_serial = config.adb_serial.clone();
                let filters = config.logcat_filters.clone();
                let package_name = options.package_name.clone();
                let crash_tx = crash_tx.clone();
                let app_pid = app_pid.clone();
                let ready_tx = ready_tx.clone();
                let marker = options.run_marker.clone();

                tasks.spawn(async move {
                    let adb_binary = resolve_host_tool("adb")?;
                    let mut command = Command::new(adb_binary);
                    command.args(["-s", &adb_serial, "logcat", "-v", "time"]);

                    if let (None, Some(pid)) = (marker.as_ref(), app_pid.as_deref()) {
                        command.arg(format!("--pid={pid}"));
                    }

                    command.args(filters);
                    if marker.is_some() {
                        command.arg("RustDroid:I");
                    }
                    command.stdout(std::process::Stdio::piped());
                    command.stderr(std::process::Stdio::null());
                    command.kill_on_drop(true);

                    let mut child = command.spawn().map_err(anyhow::Error::from)?;
                    let stdout = child
                        .stdout
                        .take()
                        .ok_or_else(|| anyhow!("adb logcat did not expose stdout"))?;
                    let _ = ready_tx.send(());
                    read_prefixed_lines(
                        "logcat",
                        stdout,
                        Some(crash_tx),
                        package_name.as_deref(),
                        marker,
                    )
                    .await?;
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
    }

    anyhow::ensure!(!tasks.is_empty(), "no requested log reader is available");
    if let (true, Some(package), Some(pid)) = (
        options.verify_liveness,
        options.package_name.clone(),
        app_pid.clone(),
    ) {
        let runtime = runtime.clone();
        let config = config.clone();
        let ready_tx = ready_tx.clone();
        tasks.spawn(async move {
            let _ = ready_tx.send(());
            loop {
                sleep(Duration::from_millis(250)).await;
                check_app_pid(&runtime, &config, &package, &pid).await?;
            }
        });
    }
    drop(crash_tx);
    drop(ready_tx);
    let result = observe_tasks(
        &mut tasks,
        ready_rx,
        &mut crash_rx,
        options.duration_secs,
        !options.verify_liveness,
    )
    .await;
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    result?;
    // A final check covers an exit racing the observation deadline.
    if let (true, Some(package), Some(pid)) = (
        options.verify_liveness,
        options.package_name.as_deref(),
        app_pid.as_deref(),
    ) {
        check_app_pid(runtime, config, package, pid).await?;
    }
    if let (Some(marker), Some(package)) = (
        options.run_marker.as_deref(),
        options.package_name.as_deref(),
    ) {
        let dump = timeout(
            Duration::from_secs(5),
            runtime.exec(
                config,
                adb_command(
                    &config.adb_serial,
                    vec![
                        "logcat".into(),
                        "-d".into(),
                        "-v".into(),
                        "time".into(),
                        "*:V".into(),
                    ],
                ),
            ),
        )
        .await
        .context("final log observation timed out")??;
        anyhow::ensure!(dump.exit_code == 0, "final log observation failed");
        if let Some(reason) = scoped_failure(&dump.stdout, marker, package)? {
            bail!("crash detected: {reason}");
        }
    }
    Ok(())
}

pub async fn begin_observation(runtime: &Runtime, config: &RuntimeConfig) -> Result<String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let marker = format!("rustdroid-run-{}-{stamp}", std::process::id());
    let result = timeout(
        Duration::from_secs(5),
        runtime.exec(
            config,
            adb_command(
                &config.adb_serial,
                vec![
                    "shell".into(),
                    "log".into(),
                    "-p".into(),
                    "i".into(),
                    "-t".into(),
                    "RustDroid".into(),
                    marker.clone(),
                ],
            ),
        ),
    )
    .await
    .context("run log marker timed out")??;
    anyhow::ensure!(
        result.exit_code == 0,
        "could not mark the current launch in logcat"
    );
    Ok(marker)
}

fn scoped_failure(logcat: &str, marker: &str, package: &str) -> Result<Option<String>> {
    let mut window = CrashWindow::new(Some(marker.to_owned()));
    let failure = window.feed(&format!("{logcat}\n"), Some(package))?;
    anyhow::ensure!(
        window.started,
        "current-run log marker is missing; observation is incomplete"
    );
    Ok(failure)
}

async fn observe_tasks(
    tasks: &mut JoinSet<Result<()>>,
    mut ready: mpsc::UnboundedReceiver<()>,
    crash: &mut mpsc::UnboundedReceiver<String>,
    duration: Option<u64>,
    allow_interrupt: bool,
) -> Result<()> {
    anyhow::ensure!(!tasks.is_empty(), "no requested log reader is available");
    let ready_deadline = sleep(Duration::from_secs(10));
    tokio::pin!(ready_deadline);
    let mut remaining = tasks.len();
    while remaining > 0 {
        select! {
            biased;
            result = tasks.join_next() => return reader_finished(result),
            Some(reason) = crash.recv() => bail!("crash detected: {reason}"),
            result = ready.recv() => {
                anyhow::ensure!(result.is_some(), "log readers did not become ready");
                remaining -= 1;
            },
            _ = &mut ready_deadline => bail!("log reader startup timed out"),
            _ = tokio::signal::ctrl_c() => return interrupted(allow_interrupt),
        }
    }
    let deadline = async {
        match duration {
            Some(seconds) => sleep(Duration::from_secs(seconds)).await,
            None => std::future::pending::<()>().await,
        }
    };
    select! {
        biased;
        result = tasks.join_next() => reader_finished(result),
        Some(reason) = crash.recv() => Err(anyhow!("crash detected: {reason}")),
        _ = tokio::signal::ctrl_c() => interrupted(allow_interrupt),
        _ = deadline => Ok(()),
    }
}

fn interrupted(allow_interrupt: bool) -> Result<()> {
    anyhow::ensure!(allow_interrupt, "observation interrupted");
    Ok(())
}

fn reader_finished(
    result: Option<std::result::Result<Result<()>, tokio::task::JoinError>>,
) -> Result<()> {
    match result {
        Some(Ok(Err(error))) => Err(anyhow!("observation task failed: {error:#}")),
        Some(Err(error)) => Err(anyhow!("observation task failed: {error}")),
        _ => bail!("requested log reader ended before observation completed"),
    }
}

async fn require_app_pid(
    runtime: &Runtime,
    config: &RuntimeConfig,
    package: &str,
) -> Result<String> {
    timeout(
        Duration::from_secs(12),
        resolve_app_pid(runtime, config, &config.adb_serial, package),
    )
    .await
    .context("application PID discovery timed out")??
    .ok_or_else(|| anyhow!("crash detected: process for {package} died before observation"))
}

async fn check_app_pid(
    runtime: &Runtime,
    config: &RuntimeConfig,
    package: &str,
    expected: &str,
) -> Result<()> {
    let result = timeout(
        Duration::from_secs(3),
        runtime.exec(
            config,
            adb_command(
                &config.adb_serial,
                vec!["shell".into(), "pidof".into(), "-s".into(), package.into()],
            ),
        ),
    )
    .await
    .context("application liveness check timed out")??;
    anyhow::ensure!(
        result.exit_code == 0 && result.stdout.trim() == expected,
        "crash detected: process for {package} died or restarted during observation"
    );
    Ok(())
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
    marker: Option<String>,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut window = CrashWindow::new(marker);
    while let Some(line) = lines.next_line().await? {
        print_prefixed(prefix, &line);
        if let (Some(crash_tx), Some(reason)) = (
            crash_tx.as_ref(),
            window.feed(&format!("{line}\n"), package_name)?,
        ) {
            let _ = crash_tx.send(reason);
        }
    }

    Ok(())
}

#[derive(Default)]
struct CrashWindow {
    pending: String,
    lines: VecDeque<String>,
    marker: Option<String>,
    started: bool,
}

impl CrashWindow {
    fn new(marker: Option<String>) -> Self {
        Self {
            marker,
            ..Self::default()
        }
    }

    fn feed(&mut self, chunk: &str, package: Option<&str>) -> Result<Option<String>> {
        self.pending.push_str(chunk);
        while let Some(end) = self.pending.find('\n') {
            let line = self.pending[..end].to_owned();
            self.pending.drain(..=end);
            if let Some(marker) = self.marker.as_deref() {
                if line.contains("RustDroid") && line.trim_end().ends_with(marker) {
                    self.started = true;
                    self.lines.clear();
                    continue;
                }
                if !self.started {
                    continue;
                }
            }
            self.lines.push_back(line);
            while self.lines.len() > 6 {
                self.lines.pop_front();
            }
            let recent = self
                .lines
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("\n");
            if let Some(reason) = detect_crash(&recent, package) {
                return Ok(Some(reason));
            }
        }
        // Fail closed instead of growing without limit or dropping a crash silently.
        if self.pending.len() > 64 * 1024 {
            bail!("log line exceeded the observation buffer limit");
        }
        Ok(None)
    }
}

fn names_package(line: &str, package: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_' && c != ':')
        .any(|name| name == package || name.starts_with(&format!("{package}:")))
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
            if lowercase.contains("anr in ") && names_package(line, package_name) {
                return Some(format!("anr in {package_name}"));
            }
            if lowercase.contains("has died") && names_package(line, package_name) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn previous_launch_crashes_are_not_current_failures() {
        let logs = "FATAL EXCEPTION: main\nProcess: com.example.app, PID: 1\nI RustDroid: current-launch\nI ActivityManager: healthy\n";
        assert_eq!(
            scoped_failure(logs, "current-launch", "com.example.app").unwrap(),
            None
        );
    }

    #[test]
    fn current_launch_crash_is_detected_after_its_marker() {
        let logs = "I RustDroid: current-launch\nFATAL EXCEPTION: main\nProcess: com.example.app, PID: 2\n";
        assert_eq!(
            scoped_failure(logs, "current-launch", "com.example.app").unwrap(),
            Some("fatal exception in com.example.app".into())
        );
    }

    #[test]
    fn missing_or_old_markers_cannot_validate_observation() {
        assert!(scoped_failure(
            "I RustDroid: old-launch\n",
            "current-launch",
            "com.example.app"
        )
        .is_err());
        assert!(scoped_failure("", "current-launch", "com.example.app").is_err());
    }

    #[test]
    fn current_run_ignores_another_app_crash() {
        let logs =
            "I RustDroid: current-launch\nFATAL EXCEPTION: main\nProcess: other.app, PID: 2\n";
        assert_eq!(
            scoped_failure(logs, "current-launch", "com.example.app").unwrap(),
            None
        );
    }

    #[test]
    fn stopping_interactive_logs_is_distinct_from_incomplete_observation() {
        assert!(interrupted(true).is_ok());
        assert!(interrupted(false).is_err());
    }

    #[test]
    fn docker_chunk_boundaries_do_not_hide_a_crash() {
        let mut window = CrashWindow::default();
        assert!(window
            .feed("E AndroidRuntime: FATAL EXCEP", Some("com.example.app"))
            .unwrap()
            .is_none());
        assert!(window
            .feed("TION: main\nE AndroidRuntime: Pro", Some("com.example.app"))
            .unwrap()
            .is_none());
        assert_eq!(
            window
                .feed("cess: com.example.app, PID: 123\n", Some("com.example.app"))
                .unwrap(),
            Some("fatal exception in com.example.app".into())
        );
    }

    #[test]
    fn similarly_named_packages_do_not_trigger_anr_or_death() {
        for line in [
            "ANR in com.example.application",
            "Process com.example.application has died",
            "ANR in other.com.example.app",
        ] {
            assert!(
                detect_crash(line, Some("com.example.app")).is_none(),
                "{line}"
            );
        }
        assert!(detect_crash("ANR in com.example.app", Some("com.example.app")).is_some());
        assert!(detect_crash(
            "Process com.example.app (pid 123) has died",
            Some("com.example.app")
        )
        .is_some());
    }

    #[test]
    fn fatal_exception_requires_the_target_package() {
        let mut window = CrashWindow::default();
        assert!(window
            .feed(
                "FATAL EXCEPTION: main\nProcess: other.app, PID: 100\n",
                Some("com.example.app")
            )
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn reader_readiness_timeout_cannot_become_a_pass() {
        let mut tasks = JoinSet::new();
        let (_ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(std::future::pending::<Result<()>>());
        let error = timeout(
            Duration::from_secs(12),
            observe_tasks(&mut tasks, ready, &mut crash, Some(1), false),
        )
        .await
        .expect("startup timeout must be bounded")
        .unwrap_err();
        assert!(error.to_string().contains("log reader startup timed out"));
        tasks.abort_all();
    }

    #[tokio::test]
    async fn closed_readiness_channel_cannot_become_a_pass() {
        let mut tasks = JoinSet::new();
        let (ready_tx, ready) = mpsc::unbounded_channel();
        drop(ready_tx);
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(std::future::pending::<Result<()>>());
        let error = observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("log readers did not become ready"));
        tasks.abort_all();
    }

    #[tokio::test]
    async fn reader_start_failure_cannot_become_a_pass() {
        let mut tasks = JoinSet::new();
        let (_ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(async { bail!("adb could not start") });
        let error = observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("adb could not start"));
    }

    #[tokio::test]
    async fn clean_eof_before_deadline_is_a_capture_failure() {
        let mut tasks = JoinSet::new();
        let (ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(async move {
            ready_tx.send(()).unwrap();
            sleep(Duration::from_millis(10)).await;
            Ok(())
        });
        let error = observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("ended before observation"));
    }

    #[tokio::test]
    async fn reader_panic_is_not_hidden() {
        let mut tasks = JoinSet::new();
        let (_ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(async {
            panic!("reader panic");
            #[allow(unreachable_code)]
            Ok(())
        });
        assert!(observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn observation_clock_starts_after_readiness() {
        let mut tasks = JoinSet::new();
        let (ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(async move {
            sleep(Duration::from_millis(150)).await;
            ready_tx.send(()).unwrap();
            std::future::pending::<Result<()>>().await
        });
        let started = Instant::now();
        observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .unwrap();
        assert!(started.elapsed() >= Duration::from_millis(1150));
        tasks.abort_all();
    }

    #[tokio::test]
    async fn runtime_reader_error_preserves_its_reason() {
        let mut tasks = JoinSet::new();
        let (ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(async move {
            ready_tx.send(()).unwrap();
            sleep(Duration::from_millis(10)).await;
            bail!("crash detected: process for example.app died");
        });
        let error = observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("crash detected"));
    }

    #[tokio::test]
    async fn anr_event_interrupts_a_healthy_reader() {
        let mut tasks = JoinSet::new();
        let (ready_tx, ready) = mpsc::unbounded_channel();
        let (crash_tx, mut crash) = mpsc::unbounded_channel();
        tasks.spawn(async move {
            ready_tx.send(()).unwrap();
            sleep(Duration::from_millis(10)).await;
            crash_tx.send("anr in example.app".into()).unwrap();
            std::future::pending::<Result<()>>().await
        });
        let error = observe_tasks(&mut tasks, ready, &mut crash, Some(1), false)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("anr in example.app"));
        tasks.abort_all();
    }

    #[tokio::test]
    async fn absent_readers_cannot_produce_a_pass() {
        let mut tasks = JoinSet::new();
        let (_ready_tx, ready) = mpsc::unbounded_channel();
        let (_crash_tx, mut crash) = mpsc::unbounded_channel();
        assert!(observe_tasks(&mut tasks, ready, &mut crash, Some(0), false)
            .await
            .is_err());
    }
}
