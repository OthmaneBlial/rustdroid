use anyhow::{bail, Result};
use serde::Serialize;

use crate::{
    cli::{RuntimeBackend, UiBackend},
    config::RuntimeConfig,
};

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub config: RuntimeConfig,
}

pub fn built_in_profiles() -> Vec<ProfileInfo> {
    profile_specs()
        .into_iter()
        .map(|(name, description)| {
            let mut config = RuntimeConfig::default();
            apply_named_profile(&mut config, name).expect("built-in profile must be valid");
            ProfileInfo {
                name,
                description,
                config,
            }
        })
        .collect()
}

pub fn apply_named_profile(config: &mut RuntimeConfig, profile_name: &str) -> Result<()> {
    match profile_name {
        "fast-local" => {
            config.runtime_backend = RuntimeBackend::Docker;
            config.image = "budtmo/docker-android:emulator_12.0".to_owned();
            config.headless = false;
            config.ui_backend = UiBackend::Scrcpy;
            config.disable_google_play_services = true;
            config.device_width_px = 540;
            config.device_height_px = 960;
            config.device_density_dpi = 220;
            config.scrcpy_max_fps = 24;
            config.scrcpy_max_size = 540;
            config.scrcpy_video_bit_rate = "2M".to_owned();
        }
        "stable-local" => {
            config.runtime_backend = RuntimeBackend::Docker;
            config.headless = false;
            config.ui_backend = UiBackend::Scrcpy;
            config.emulator_gpu_mode = "auto".to_owned();
        }
        "host-fast" => {
            config.runtime_backend = RuntimeBackend::Host;
            config.headless = false;
            config.ui_backend = UiBackend::Scrcpy;
            config.disable_google_play_services = true;
            config.device_width_px = 540;
            config.device_height_px = 960;
            config.device_density_dpi = 220;
            config.scrcpy_max_fps = 24;
            config.scrcpy_max_size = 540;
            config.scrcpy_video_bit_rate = "2M".to_owned();
            config.emulator_gpu_mode = "host".to_owned();
        }
        "docker-ci" => {
            config.runtime_backend = RuntimeBackend::Docker;
            config.headless = true;
            config.ui_backend = UiBackend::Scrcpy;
            config.disable_google_play_services = true;
            config.boot_timeout_secs = 420;
            config.logcat_filters = vec!["*:W".to_owned()];
        }
        "browser-demo" => {
            config.runtime_backend = RuntimeBackend::Docker;
            config.headless = false;
            config.ui_backend = UiBackend::Web;
            config.emulator_gpu_mode = "auto".to_owned();
        }
        "low-ram" => {
            config.runtime_backend = RuntimeBackend::Docker;
            config.headless = true;
            config.emulator_cpu_cores = 2;
            config.emulator_ram_mb = 2048;
            config.emulator_vm_heap_mb = 256;
            config.device_width_px = 480;
            config.device_height_px = 854;
            config.device_density_dpi = 200;
            config.scrcpy_max_size = 480;
            config.scrcpy_video_bit_rate = "1500K".to_owned();
        }
        _ => bail!(
            "unknown profile '{}'; run `rustdroid profile list` to see the built-in profiles",
            profile_name
        ),
    }

    Ok(())
}

fn profile_specs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "fast-local",
            "Small Docker-first loop tuned for quick local APK checks",
        ),
        (
            "stable-local",
            "Balanced Docker profile for repeatable local work",
        ),
        ("host-fast", "Fastest host-emulator loop with scrcpy"),
        ("docker-ci", "Headless Docker profile for CI-style runs"),
        ("browser-demo", "Docker profile with the browser UI enabled"),
        ("low-ram", "Reduced-memory profile for constrained machines"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{apply_named_profile, built_in_profiles};
    use crate::{
        cli::{RuntimeBackend, UiBackend},
        config::RuntimeConfig,
    };

    #[test]
    fn host_fast_profile_switches_to_host_visible_scrcpy_loop() {
        let mut config = RuntimeConfig::default();
        apply_named_profile(&mut config, "host-fast").expect("profile should apply");

        assert_eq!(config.runtime_backend, RuntimeBackend::Host);
        assert!(!config.headless);
        assert_eq!(config.ui_backend, UiBackend::Scrcpy);
        assert_eq!(config.emulator_gpu_mode, "host");
        assert!(config.disable_google_play_services);
    }

    #[test]
    fn built_in_profiles_include_expected_public_names() {
        let profiles = built_in_profiles();
        let names: Vec<_> = profiles.iter().map(|profile| profile.name).collect();

        assert!(names.contains(&"fast-local"));
        assert!(names.contains(&"host-fast"));
        assert!(names.contains(&"docker-ci"));
    }

    #[test]
    fn unknown_profile_returns_clear_error() {
        let mut config = RuntimeConfig::default();
        let error = apply_named_profile(&mut config, "unknown-profile").expect_err("should fail");

        assert!(
            error
                .to_string()
                .contains("unknown profile 'unknown-profile'"),
            "unexpected error: {error}"
        );
    }
}
