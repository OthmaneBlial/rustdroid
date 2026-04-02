#![allow(dead_code)]

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

pub struct TestContext {
    _temp_dir: TempDir,
    pub config_path: PathBuf,
}

impl TestContext {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let config_path = temp_dir.path().join("rustdroid.test.toml");
        Self {
            _temp_dir: temp_dir,
            config_path,
        }
    }
}

pub fn rustdroid_command(context: &TestContext) -> Command {
    let mut command = Command::new(rustdroid_bin());
    command.arg("--config").arg(&context.config_path);
    command
}

pub fn run_command(command: &mut Command) -> Output {
    command.output().expect("command should run")
}

pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn assert_output_contains(output: &Output, needle: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "expected output to contain '{}'\nstdout:\n{}\nstderr:\n{}",
        needle,
        stdout,
        stderr
    );
}

pub fn read_to_string(path: &Path) -> String {
    std::fs::read_to_string(path).expect("file should be readable")
}

fn rustdroid_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rustdroid")
}

#[allow(dead_code)]
pub fn os(value: &str) -> &OsStr {
    OsStr::new(value)
}
