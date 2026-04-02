use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use bollard::{
    container::{
        Config, CreateContainerOptions, InspectContainerOptions, LogOutput, RemoveContainerOptions,
        StartContainerOptions, StopContainerOptions, UploadToContainerOptions,
    },
    errors::Error as BollardError,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    image::CreateImageOptions,
    models::{DeviceMapping, HostConfig, PortBinding, PortMap},
    Docker,
};
use bytes::Bytes;
use futures_util::StreamExt;
use tar::Builder;

use crate::config::RuntimeConfig;

const MANAGED_LABEL_KEY: &str = "io.rustdroid.managed";
const MANAGED_LABEL_VALUE: &str = "true";
const MANAGED_CONFIG_HASH_LABEL_KEY: &str = "io.rustdroid.config-hash";
const BASE_SUPERVISORD_PATH: &str =
    "/home/androidusr/docker-android/mixins/configs/process/supervisord-base.conf";
const SCREEN_SUPERVISORD_PATH: &str =
    "/home/androidusr/docker-android/mixins/configs/process/supervisord-screen.conf";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
}

#[derive(Debug, Clone)]
pub struct DockerRuntime {
    docker: Docker,
}

#[derive(Debug, Clone, PartialEq)]
struct PreparedContainerConfig {
    env: Vec<String>,
    binds: Option<Vec<String>>,
    exposed_ports: Option<std::collections::HashMap<String, std::collections::HashMap<(), ()>>>,
    port_bindings: Option<PortMap>,
    config_hash: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OverrideConfigMount {
    host_path: PathBuf,
    container_path: String,
}

impl DockerRuntime {
    pub fn connect() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    pub fn client(&self) -> &Docker {
        &self.docker
    }

    pub async fn ping(&self) -> Result<()> {
        self.docker.ping().await?;
        Ok(())
    }

    pub async fn ensure_started(&self, config: &RuntimeConfig) -> Result<()> {
        self.ensure_image(&config.image).await?;

        match self
            .docker
            .inspect_container(&config.container_name, None::<InspectContainerOptions>)
            .await
        {
            Ok(existing) => {
                let managed = existing
                    .config
                    .as_ref()
                    .and_then(|container_config| container_config.labels.as_ref())
                    .and_then(|labels| labels.get(MANAGED_LABEL_KEY))
                    .is_some_and(|value| value == MANAGED_LABEL_VALUE);

                if !managed {
                    bail!(
                        "container '{}' already exists and is not managed by rustdroid",
                        config.container_name
                    );
                }

                if !container_matches_config(&existing, config)? {
                    eprintln!(
                        "recreating container {} to apply config changes",
                        config.container_name
                    );
                    self.docker
                        .remove_container(
                            &config.container_name,
                            Some(RemoveContainerOptions {
                                force: true,
                                ..Default::default()
                            }),
                        )
                        .await
                        .with_context(|| {
                            format!("failed to remove container '{}'", config.container_name)
                        })?;
                    self.create_managed_container(config).await?;
                    eprintln!("starting container {}", config.container_name);
                    self.docker
                        .start_container(
                            &config.container_name,
                            None::<StartContainerOptions<String>>,
                        )
                        .await
                        .with_context(|| {
                            format!("failed to start container '{}'", config.container_name)
                        })?;
                    print_visual_access(config);
                    return Ok(());
                }

                let running = existing
                    .state
                    .as_ref()
                    .and_then(|state| state.running)
                    .unwrap_or(false);

                if !running {
                    eprintln!(
                        "recreating stopped container {} for a clean emulator boot",
                        config.container_name
                    );
                    self.docker
                        .remove_container(
                            &config.container_name,
                            Some(RemoveContainerOptions {
                                force: true,
                                ..Default::default()
                            }),
                        )
                        .await
                        .with_context(|| {
                            format!("failed to remove container '{}'", config.container_name)
                        })?;
                    self.create_managed_container(config).await?;
                    eprintln!("starting container {}", config.container_name);
                    self.docker
                        .start_container(
                            &config.container_name,
                            None::<StartContainerOptions<String>>,
                        )
                        .await
                        .with_context(|| {
                            format!("failed to start container '{}'", config.container_name)
                        })?;
                    print_visual_access(config);
                }
            }
            Err(error) if is_not_found(&error) => {
                eprintln!("creating container {}", config.container_name);
                self.create_managed_container(config).await?;
                eprintln!("starting container {}", config.container_name);
                self.docker
                    .start_container(
                        &config.container_name,
                        None::<StartContainerOptions<String>>,
                    )
                    .await
                    .with_context(|| {
                        format!("failed to start container '{}'", config.container_name)
                    })?;
                print_visual_access(config);
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    pub async fn stop(&self, container_name: &str, timeout_secs: u64) -> Result<()> {
        match self
            .docker
            .stop_container(
                container_name,
                Some(StopContainerOptions {
                    t: timeout_secs as i64,
                }),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(error) if is_not_found(&error) => Ok(()),
            Err(error) => {
                Err(error).with_context(|| format!("failed to stop container '{container_name}'"))
            }
        }
    }

    pub async fn exec(&self, container_name: &str, command: Vec<String>) -> Result<ExecOutcome> {
        let exec = self
            .docker
            .create_exec(
                container_name,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(command.clone()),
                    ..Default::default()
                },
            )
            .await
            .with_context(|| {
                format!(
                    "failed to create exec in '{}' for command {:?}",
                    container_name, command
                )
            })?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        match self
            .docker
            .start_exec(&exec.id, None::<StartExecOptions>)
            .await
            .with_context(|| format!("failed to start exec in '{container_name}'"))?
        {
            StartExecResults::Attached { mut output, .. } => {
                while let Some(chunk) = output.next().await {
                    match chunk? {
                        LogOutput::StdOut { message } | LogOutput::Console { message } => {
                            stdout.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::StdErr { message } => {
                            stderr.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::StdIn { .. } => {}
                    }
                }
            }
            StartExecResults::Detached => {
                bail!("unexpected detached exec result for command {:?}", command);
            }
        }

        let inspect = self
            .docker
            .inspect_exec(&exec.id)
            .await
            .with_context(|| format!("failed to inspect exec in '{container_name}'"))?;

        Ok(ExecOutcome {
            stdout,
            stderr,
            exit_code: inspect.exit_code.unwrap_or_default(),
        })
    }

    pub async fn upload_file(
        &self,
        container_name: &str,
        local_path: &Path,
        remote_dir: &str,
        remote_name: &str,
    ) -> Result<String> {
        self.exec(
            container_name,
            vec!["mkdir".to_owned(), "-p".to_owned(), remote_dir.to_owned()],
        )
        .await?;

        let tarball = make_tar_archive(local_path, remote_name)?;
        self.docker
            .upload_to_container(
                container_name,
                Some(UploadToContainerOptions {
                    path: remote_dir.to_owned(),
                    no_overwrite_dir_non_dir: "true".to_owned(),
                }),
                Bytes::from(tarball),
            )
            .await
            .with_context(|| {
                format!(
                    "failed to upload '{}' into container '{}'",
                    local_path.display(),
                    container_name
                )
            })?;

        Ok(format!("{remote_dir}/{remote_name}"))
    }

    async fn ensure_image(&self, image: &str) -> Result<()> {
        match self.docker.inspect_image(image).await {
            Ok(_) => Ok(()),
            Err(error) if is_not_found(&error) => {
                eprintln!("pulling image {image}");
                let mut stream = self.docker.create_image(
                    Some(CreateImageOptions {
                        from_image: image,
                        ..Default::default()
                    }),
                    None,
                    None,
                );

                while let Some(progress) = stream.next().await {
                    let progress =
                        progress.with_context(|| format!("failed to pull image '{image}'"))?;
                    let id = progress.id.unwrap_or_default();
                    let status = progress.status.unwrap_or_default();
                    let detail = progress.progress.unwrap_or_default();
                    if id.is_empty() {
                        let message = format!("{status} {detail}");
                        eprintln!("{}", message.trim());
                    } else {
                        let message = format!("{id}: {status} {detail}");
                        eprintln!("{}", message.trim());
                    }
                }
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    async fn create_managed_container(&self, config: &RuntimeConfig) -> Result<()> {
        let prepared = prepare_container_config(config)?;
        self.docker
            .create_container(
                Some(CreateContainerOptions {
                    name: config.container_name.clone(),
                    platform: None,
                }),
                Config {
                    image: Some(config.image.clone()),
                    env: Some(prepared.env.clone()),
                    exposed_ports: prepared.exposed_ports.clone(),
                    labels: Some(std::collections::HashMap::from([
                        (MANAGED_LABEL_KEY.to_owned(), MANAGED_LABEL_VALUE.to_owned()),
                        (
                            MANAGED_CONFIG_HASH_LABEL_KEY.to_owned(),
                            prepared.config_hash.clone(),
                        ),
                    ])),
                    host_config: Some(HostConfig {
                        init: Some(true),
                        binds: prepared.binds.clone(),
                        port_bindings: prepared.port_bindings.clone(),
                        shm_size: Some(recommended_shm_size_bytes(config)),
                        devices: Some(runtime_devices(config)),
                        group_add: runtime_group_add(config),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .with_context(|| {
                format!(
                    "failed to create managed container '{}'",
                    config.container_name
                )
            })?;

        Ok(())
    }
}

fn is_not_found(error: &BollardError) -> bool {
    matches!(
        error,
        BollardError::DockerResponseServerError {
            status_code: 404,
            ..
        }
    )
}

fn make_tar_archive(local_path: &Path, remote_name: &str) -> Result<Vec<u8>> {
    let mut archive = Vec::new();
    {
        let mut builder = Builder::new(&mut archive);
        builder
            .append_path_with_name(local_path, remote_name)
            .with_context(|| format!("failed to archive '{}'", local_path.display()))?;
        builder.finish()?;
    }
    Ok(archive)
}

fn port_configuration(
    config: &RuntimeConfig,
) -> (
    Option<std::collections::HashMap<String, std::collections::HashMap<(), ()>>>,
    Option<PortMap>,
) {
    if config.headless {
        return (None, None);
    }

    let mut exposed_ports = std::collections::HashMap::new();
    let mut port_bindings = PortMap::new();

    if config.uses_scrcpy_ui() {
        exposed_ports.insert("5555/tcp".to_owned(), std::collections::HashMap::new());
        port_bindings.insert(
            "5555/tcp".to_owned(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_owned()),
                host_port: Some(config.adb_connect_port.to_string()),
            }]),
        );
    }

    if config.uses_vnc_ui() {
        exposed_ports.insert("5900/tcp".to_owned(), std::collections::HashMap::new());
        port_bindings.insert(
            "5900/tcp".to_owned(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_owned()),
                host_port: Some(config.vnc_port.to_string()),
            }]),
        );
    }

    if config.uses_web_ui() {
        exposed_ports.insert("6080/tcp".to_owned(), std::collections::HashMap::new());
        port_bindings.insert(
            "6080/tcp".to_owned(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_owned()),
                host_port: Some(config.web_vnc_port.to_string()),
            }]),
        );
    }

    (Some(exposed_ports), Some(port_bindings))
}

fn print_visual_access(config: &RuntimeConfig) {
    if !config.headless {
        if config.uses_scrcpy_ui() {
            eprintln!(
                "local desktop via scrcpy on 127.0.0.1:{}",
                config.adb_connect_port
            );
        }
        if config.uses_vnc_ui() {
            eprintln!("legacy VNC available on 127.0.0.1:{}", config.vnc_port);
        }
        if config.uses_web_ui() {
            eprintln!(
                "browser UI available at http://127.0.0.1:{}?autoconnect=true",
                config.web_vnc_port
            );
        }
    }
}

fn base_env(config: &RuntimeConfig) -> Vec<String> {
    vec![
        "DEVICE_TYPE=emulator".to_owned(),
        "APPIUM=false".to_owned(),
        "USER_BEHAVIOR_ANALYTICS=false".to_owned(),
        "WEB_LOG=false".to_owned(),
        format!("WEB_VNC={}", !config.headless && config.uses_web_ui()),
        "VNC_PORT=5900".to_owned(),
        "WEB_VNC_PORT=6080".to_owned(),
        format!("EMULATOR_DEVICE={}", config.device),
        format!("EMULATOR_HEADLESS={}", config.effective_emulator_headless()),
        format!("EMULATOR_NO_SKIN={}", config.no_skin),
        format!(
            "EMULATOR_ADDITIONAL_ARGS={}",
            config.effective_emulator_additional_args()
        ),
    ]
}

fn prepare_container_config(config: &RuntimeConfig) -> Result<PreparedContainerConfig> {
    let (exposed_ports, port_bindings) = port_configuration(config);
    let mut env = base_env(config);
    let override_mount = prepare_override_mount(config)?;
    env.push(format!(
        "EMULATOR_CONFIG_PATH={}",
        override_mount.container_path
    ));

    let mut bind_specs = vec![format!(
        "{}:{}:ro",
        override_mount.host_path.display(),
        override_mount.container_path
    )];
    bind_specs.extend(prepare_supervisor_bindings(config)?);
    let binds = Some(bind_specs);
    let config_hash = container_config_hash(config, &env, binds.as_ref(), &port_bindings);

    Ok(PreparedContainerConfig {
        env,
        binds,
        exposed_ports,
        port_bindings,
        config_hash,
    })
}

fn prepare_override_mount(config: &RuntimeConfig) -> Result<OverrideConfigMount> {
    let contents = config.emulator_override_config();
    let override_hash = stable_hash_hex(&contents);
    let sanitized_name = sanitize_name(&config.container_name);
    let host_dir = std::env::temp_dir()
        .join("rustdroid")
        .join("emulator-config");
    fs::create_dir_all(&host_dir)?;

    let file_name = format!("{sanitized_name}-{override_hash}.ini");
    let host_path = host_dir.join(file_name);
    fs::write(&host_path, contents)?;

    Ok(OverrideConfigMount {
        host_path,
        container_path: format!("/tmp/rustdroid-{sanitized_name}-{override_hash}.ini"),
    })
}

fn prepare_supervisor_bindings(config: &RuntimeConfig) -> Result<Vec<String>> {
    let host_dir = std::env::temp_dir().join("rustdroid").join("supervisord");
    fs::create_dir_all(&host_dir)?;

    let mut binds = Vec::new();

    let base_config = rustdroid_supervisord_base_config();
    let base_hash = stable_hash_hex(&base_config);
    let base_host_path = host_dir.join(format!("base-{base_hash}.conf"));
    fs::write(&base_host_path, base_config)?;
    binds.push(format!(
        "{}:{}:ro",
        base_host_path.display(),
        BASE_SUPERVISORD_PATH
    ));

    if config.uses_screen_stack() {
        let screen_config =
            rustdroid_supervisord_screen_config(config.uses_web_ui(), config.uses_vnc_ui());
        let screen_hash = stable_hash_hex(&screen_config);
        let screen_host_path = host_dir.join(format!("screen-{screen_hash}.conf"));
        fs::write(&screen_host_path, screen_config)?;
        binds.push(format!(
            "{}:{}:ro",
            screen_host_path.display(),
            SCREEN_SUPERVISORD_PATH
        ));
    }

    Ok(binds)
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

fn stable_hash_hex(value: &impl Hash) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn rustdroid_supervisord_base_config() -> String {
    r#"[supervisord]
nodaemon=true
logfile=%(ENV_LOG_PATH)s/supervisord-base.log
childlogdir=%(ENV_LOG_PATH)s

[program:device]
command=/usr/local/bin/docker-android start device
autostart=true
autorestart=false
startretries=0
stdout_logfile=%(ENV_LOG_PATH)s/device.stdout.log
redirect_stderr=true
priority=1
stopsignal=TERM
stopwaitsecs=10
"#
    .to_owned()
}

fn rustdroid_supervisord_screen_config(include_web_vnc: bool, include_vnc_server: bool) -> String {
    let mut config = r#"[supervisord]
nodaemon=true
logfile=%(ENV_LOG_PATH)s/supervisord-extend.log
childlogdir=%(ENV_LOG_PATH)s

[program:d_screen]
command=/usr/local/bin/docker-android start display_screen
autostart=true
autorestart=true
stdout_logfile=%(ENV_LOG_PATH)s/display_screen.stdout.log
stderr_logfile=%(ENV_LOG_PATH)s/display_screen.stderr.log
redirect_stderr=false
priority=1

[program:d_wm]
command=/usr/local/bin/docker-android start display_wm
autostart=true
autorestart=true
stdout_logfile=%(ENV_LOG_PATH)s/display_wm.stdout.log
stderr_logfile=%(ENV_LOG_PATH)s/display_wm.stderr.log
redirect_stderr=false
priority=2
"#
    .to_owned();

    if include_vnc_server || include_web_vnc {
        config.push_str(
            r#"
[program:vnc_server]
command=/usr/local/bin/docker-android start vnc_server
autostart=true
autorestart=true
stdout_logfile=%(ENV_LOG_PATH)s/vnc_server.stdout.log
stderr_logfile=%(ENV_LOG_PATH)s/vnc_server.stderr.log
redirect_stderr=false
priority=3
"#,
        );
    }

    if include_web_vnc {
        config.push_str(
            r#"
[program:vnc_web]
command=/usr/local/bin/docker-android start vnc_web
autostart=true
autorestart=true
stdout_logfile=%(ENV_LOG_PATH)s/vnc_web.stdout.log
stderr_logfile=%(ENV_LOG_PATH)s/vnc_web.stderr.log
redirect_stderr=false
priority=3
"#,
        );
    }

    config
}

fn container_config_hash(
    config: &RuntimeConfig,
    env: &[String],
    binds: Option<&Vec<String>>,
    port_bindings: &Option<PortMap>,
) -> String {
    let mut hasher = DefaultHasher::new();
    config.image.hash(&mut hasher);
    config.container_name.hash(&mut hasher);
    env.hash(&mut hasher);
    binds.hash(&mut hasher);
    hash_port_bindings(&mut hasher, port_bindings);
    format!("{:016x}", hasher.finish())
}

fn recommended_shm_size_bytes(config: &RuntimeConfig) -> i64 {
    let shm_mb = (config.emulator_ram_mb / 2).clamp(1024, 2048);
    (shm_mb * 1024 * 1024) as i64
}

fn runtime_devices(config: &RuntimeConfig) -> Vec<DeviceMapping> {
    let mut devices = vec![DeviceMapping {
        path_on_host: Some("/dev/kvm".to_owned()),
        path_in_container: Some("/dev/kvm".to_owned()),
        cgroup_permissions: Some("rwm".to_owned()),
    }];

    if config.docker_gpu_passthrough {
        devices.extend(gpu_device_paths().into_iter().map(|path| DeviceMapping {
            path_on_host: Some(path.clone()),
            path_in_container: Some(path),
            cgroup_permissions: Some("rwm".to_owned()),
        }));
    }

    devices
}

fn runtime_group_add(config: &RuntimeConfig) -> Option<Vec<String>> {
    #[cfg(unix)]
    {
        use std::collections::BTreeSet;
        use std::os::unix::fs::MetadataExt;

        let mut groups = BTreeSet::new();

        for path in std::iter::once("/dev/kvm".to_owned()).chain(
            if config.docker_gpu_passthrough {
                gpu_device_paths().into_iter()
            } else {
                Vec::new().into_iter()
            },
        ) {
            if let Ok(metadata) = fs::metadata(&path) {
                groups.insert(metadata.gid().to_string());
            }
        }

        if groups.is_empty() {
            None
        } else {
            Some(groups.into_iter().collect())
        }
    }

    #[cfg(not(unix))]
    {
        let _ = config;
        None
    }
}

fn gpu_device_paths() -> Vec<String> {
    #[cfg(unix)]
    use std::os::unix::fs::FileTypeExt;

    let Ok(entries) = fs::read_dir("/dev/dri") else {
        return Vec::new();
    };

    let mut paths: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            fs::metadata(path)
                .map(|metadata| metadata.file_type().is_char_device())
                .unwrap_or(false)
        })
        .filter_map(|path| path.to_str().map(str::to_owned))
        .collect();
    paths.sort();
    paths
}

fn hash_port_bindings(hasher: &mut DefaultHasher, port_bindings: &Option<PortMap>) {
    let Some(port_bindings) = port_bindings else {
        0_u8.hash(hasher);
        return;
    };

    let mut entries: Vec<_> = port_bindings.iter().collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));

    for (port, bindings) in entries {
        port.hash(hasher);
        match bindings {
            Some(bindings) => {
                for binding in bindings {
                    binding.host_ip.hash(hasher);
                    binding.host_port.hash(hasher);
                }
            }
            None => 0_u8.hash(hasher),
        }
    }
}

fn container_matches_config(
    existing: &bollard::models::ContainerInspectResponse,
    config: &RuntimeConfig,
) -> Result<bool> {
    let Some(container_config) = existing.config.as_ref() else {
        return Ok(false);
    };

    if container_config.image.as_deref() != Some(config.image.as_str()) {
        return Ok(false);
    }

    let prepared = prepare_container_config(config)?;
    let Some(existing_env) = container_config.env.as_ref() else {
        return Ok(false);
    };
    if !prepared
        .env
        .iter()
        .all(|expected| existing_env.iter().any(|actual| actual == expected))
    {
        return Ok(false);
    }

    let existing_port_bindings = existing
        .host_config
        .as_ref()
        .and_then(|host_config| host_config.port_bindings.clone());
    if prepared.port_bindings != existing_port_bindings {
        return Ok(false);
    }

    let existing_binds = existing
        .host_config
        .as_ref()
        .and_then(|host_config| host_config.binds.clone());
    if prepared.binds != existing_binds {
        return Ok(false);
    }

    let existing_hash = container_config
        .labels
        .as_ref()
        .and_then(|labels| labels.get(MANAGED_CONFIG_HASH_LABEL_KEY));

    Ok(existing_hash == Some(&prepared.config_hash))
}
