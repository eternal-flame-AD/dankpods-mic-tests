use std::{io::Write, process::Command};

pub fn concat_videos<I>(inputs: I, output: &str, copy: bool) -> anyhow::Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let filelist_path = format!("{}.txt", output);
    let mut filelist = std::fs::File::create(&filelist_path)?;
    for input in inputs {
        let abs_path = std::fs::canonicalize(input.as_ref())?;
        writeln!(filelist, "file '{}'", abs_path.display())?;
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-f").arg("concat");
    cmd.arg("-safe").arg("0");
    cmd.arg("-i").arg(filelist_path);
    cmd.arg("-fflags").arg("+igndts");
    if copy {
        cmd.arg("-c").arg("copy");
    }
    cmd.arg(output);

    cmd.stderr(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());

    if !cmd.spawn()?.wait()?.success() {
        return Err(anyhow::anyhow!("Failed to concat videos"));
    }

    Ok(())
}

pub fn concat_videos_filter<I>(
    inputs: I,
    output: &str,
    subtitle: Option<&str>,
    cuda: bool,
) -> anyhow::Result<()>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut filter_args = String::new();
    let mut cmd = Command::new("ffmpeg");

    if cuda {
        cmd.arg("-hwaccel").arg("cuda");
    }

    let mut n = 0;
    for (i, input) in inputs.into_iter().enumerate() {
        n += 1;
        cmd.arg("-i").arg(input.as_ref());
        filter_args.push_str(&format!("[{}:v:0][{}:a:0]", i, i));
    }
    filter_args.push_str(format!("concat=n={}:v=1:a=1[outv][outa]", n).as_str());
    if let Some(subtitle) = subtitle {
        filter_args.push_str(&format!(";[outv]subtitles={}:force_style='Alignment=1,OutlineColour=&H100000000,BorderStyle=3,Outline=1,Shadow=0,Fontsize=18'[outv]", subtitle));
    }
    cmd.arg("-filter_complex").arg(filter_args);
    cmd.arg("-map").arg("[outv]");
    cmd.arg("-map").arg("[outa]");

    cmd.arg("-preset").arg("slow");

    if cuda {
        cmd.arg("-c:v").arg("h264_nvenc");
    }

    cmd.arg(output);

    cmd.stderr(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());

    if !cmd.spawn()?.wait()?.success() {
        return Err(anyhow::anyhow!("Failed to concat videos"));
    }

    Ok(())
}
