use std::process::Command;

use log::warn;

pub fn download_video(url: &str, output: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("/usr/bin/python");
    cmd.arg("-m").arg("youtube_dl");
    cmd.arg("-o").arg(format!("{}", output));
    cmd.arg("-f").arg("mp4[height=1080]+bestaudio");
    cmd.arg(url);

    cmd.stderr(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());

    if !cmd.spawn()?.wait()?.success() {
        return Err(anyhow::anyhow!("Failed to download video"));
    }

    Ok(())
}

pub fn download_video_with_retries(url: &str, output: &str, retries: usize) -> anyhow::Result<()> {
    for i in 0..retries {
        println!("Downloading video {} (attempt {})", url, i + 1);
        match download_video(url, output) {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!("Failed to download video: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }
    }
    Err(anyhow::anyhow!("Failed to download video"))
}
