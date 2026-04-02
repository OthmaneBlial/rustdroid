use anyhow::{Context, Result};
use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value)
        .context("failed to serialize command output as JSON")?;
    println!("{text}");
    Ok(())
}
