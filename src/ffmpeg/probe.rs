use std::{collections::HashMap, process::Command};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StreamInfo {
    pub format: StreamFormat,
}

#[derive(Debug, Deserialize)]
pub struct StreamFormat {
    pub filename: String,
    pub nb_streams: u64,
    pub nb_programs: u64,
    pub format_name: String,
    pub format_long_name: String,
    pub start_time: String,
    pub duration: String,
    pub size: String,
    pub bit_rate: String,
    pub probe_score: u64,
    pub tags: HashMap<String, String>,
}

pub fn probe_format(input: &str) -> anyhow::Result<StreamInfo> {
    let mut cmd = Command::new("ffprobe");
    cmd.arg("-v").arg("quiet");
    cmd.arg("-print_format").arg("json");
    cmd.arg("-show_format");
    cmd.arg(input);

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to probe format"));
    }

    let info: StreamInfo = serde_json::from_slice(&output.stdout)?;

    Ok(info)
}
