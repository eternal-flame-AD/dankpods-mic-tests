use std::process::Command;

use super::VideoTimestamp;

pub fn make_multiple_clip<I>(
    input: &str,
    output: &str,
    ranges: I,
    cuda: bool,
    overwrite: bool,
) -> anyhow::Result<()>
where
    I: IntoIterator<Item = (VideoTimestamp, VideoTimestamp)>,
{
    let mut cmd = Command::new("ffmpeg");
    if cuda {
        cmd.arg("-hwaccel").arg("cuda");
    }

    if overwrite {
        cmd.arg("-y");
    }

    cmd.arg("-i").arg(input);

    let mut vf = String::from("select='");
    let mut af = String::from("aselect='");

    for (i, (begin, end)) in ranges.into_iter().enumerate() {
        if i > 0 {
            vf.push_str("+");
            af.push_str("+");
        }
        vf.push_str(&format!(
            "between(t,{:.3},{:.3})",
            begin.as_float_seconds(),
            end.as_float_seconds()
        ));
        af.push_str(&format!(
            "between(t,{:.3},{:.3})",
            begin.as_float_seconds(),
            end.as_float_seconds()
        ));
    }

    vf.push_str("',setpts=N/FRAME_RATE/TB");
    af.push_str("',asetpts=N/SR/TB");

    cmd.arg("-vf").arg(vf);
    cmd.arg("-af").arg(af);

    if cuda {
        cmd.arg("-c:v").arg("h264_nvenc");
    }

    cmd.arg(output);

    if !cmd.spawn()?.wait()?.success() {
        return Err(anyhow::anyhow!("Failed to make clip"));
    }

    Ok(())
}
