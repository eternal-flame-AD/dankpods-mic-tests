use std::{path::PathBuf, process::Command};

use super::VideoTimestamp;

#[derive(Debug)]
pub struct Thumbnail {
    pub seq: u64,
    pub path: String,
    pub timestamp: VideoTimestamp,
}

pub fn collect_thumbnail_into(
    output: PathBuf,
    from: Option<VideoTimestamp>,
    fps: (u64, u64),
) -> anyhow::Result<Vec<Thumbnail>> {
    let mut thumbs = Vec::new();

    for entry in std::fs::read_dir(output)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_str().unwrap();
            if !filename.starts_with("thumb") || !filename.ends_with(".jpg") {
                continue;
            }
            let seq = filename
                .strip_prefix("thumb")
                .unwrap()
                .strip_suffix(".jpg")
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let timestamp = VideoTimestamp::from_float_seconds(
                from.as_ref()
                    .map(|from| from.as_float_seconds())
                    .unwrap_or(0.0)
                    + (seq as f64 - 0.5) / (fps.0 as f64 / fps.1 as f64),
            );
            thumbs.push(Thumbnail {
                seq,
                path: path.to_str().unwrap().into(),
                timestamp,
            });
        }
    }

    Ok(thumbs)
}

pub fn generate_thumbnails(
    input: &str,
    output: PathBuf,
    from: Option<VideoTimestamp>,
    to: Option<VideoTimestamp>,
    fps: (u64, u64),
) -> anyhow::Result<Vec<Thumbnail>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(input);
    if let Some(ref from) = from {
        cmd.arg("-ss").arg(from.as_ffmpeg_arg());
    }
    if let Some(ref to) = to {
        cmd.arg("-to").arg(to.as_ffmpeg_arg());
    }
    cmd.arg("-vf").arg(format!("fps={}/{}", fps.0, fps.1));
    cmd.arg("-vsync").arg("0");
    cmd.arg("-qscale:v").arg("2");
    cmd.arg("-f").arg("image2");
    cmd.arg(output.join("thumb%04d.jpg"));
    cmd.stderr(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    if !cmd.spawn()?.wait()?.success() {
        return Err(anyhow::anyhow!("Failed to generate thumbnails"));
    }

    Ok(collect_thumbnail_into(output, from, fps)?)
}
