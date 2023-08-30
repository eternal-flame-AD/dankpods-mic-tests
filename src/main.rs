use std::{
    fs::{create_dir_all, read_dir},
    io::Write,
    path::Path,
    sync::Mutex,
};

use chrono::{DateTime, FixedOffset};
use clap::Parser;
use dankpods_mic_tests::{
    download::download_video_with_retries,
    ffmpeg::{
        clip::{self, make_multiple_clip},
        concat::{concat_videos, concat_videos_filter},
        probe::probe_format,
        thumbnail::{collect_thumbnail_into, generate_thumbnails},
        VideoTimestamp,
    },
    iter::iter_continuous_range,
    recog::image_file_is_mictest,
};
use itertools::Itertools;
use log::info;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Commands,
}

#[derive(Parser)]
pub enum Commands {
    #[clap(name = "find-clips")]
    FindClips(FindClipsArgs),
    #[clap(name = "make-clips")]
    MakeClips(MakeClipsArgs),
    #[clap(name = "concat")]
    Concat,
}

#[derive(Parser)]
pub struct FindClipsArgs {
    #[clap(long)]
    pub skip_existing_clips: bool,
    #[clap(long)]
    pub from_id: Option<String>,
}

#[derive(Parser)]
pub struct MakeClipsArgs {
    #[clap(long)]
    pub skip_existing_clips: bool,
    #[clap(long)]
    pub video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistItemResponse {
    items: Vec<PlaylistItem>,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistItem {
    snippet: PlaylistItemSnippet,
    #[serde(rename = "contentDetails")]
    content_details: PlaylistItemContentDetails,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistItemSnippet {
    title: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistItemContentDetails {
    #[serde(rename = "videoId")]
    video_id: String,
    #[serde(rename = "videoPublishedAt")]
    video_published_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipsInfo {
    ranges: Vec<(VideoTimestamp, VideoTimestamp)>,
}

fn find_clips(id: &str, args: &FindClipsArgs) {
    let video_path = format!("data/videos/{}", id);
    let video_path = if Path::new(&video_path).exists() {
        video_path
    } else if Path::new(&format!("data/videos/{}.mp4", id)).exists() {
        format!("data/videos/{}.mp4", id)
    } else if Path::new(&format!("data/videos/{}.mkv", id)).exists() {
        format!("data/videos/{}.mkv", id)
    } else {
        println!("Downloading video {}", id);
        download_video_with_retries(
            &format!("https://www.youtube.com/watch?v={}", id),
            &video_path,
            5,
        )
        .expect("Failed to download video");
        if Path::new(&format!("data/videos/{}.mp4", id)).exists() {
            format!("data/videos/{}.mp4", id)
        } else if Path::new(&format!("data/videos/{}.mkv", id)).exists() {
            format!("data/videos/{}.mkv", id)
        } else {
            panic!("Failed to download video")
        }
    };

    let clips_file = format!("data/clips/{}.json", id);
    if args.skip_existing_clips && Path::new(&clips_file).exists() {
        return;
    }

    let second_thumbnail_dir = format!("data/thumbnails/{}/second", id);
    let second_thumbnail_info = if !std::path::Path::new(&second_thumbnail_dir).exists() {
        create_dir_all(&second_thumbnail_dir).expect("Failed to create thumbnail directory");
        generate_thumbnails(&video_path, second_thumbnail_dir.into(), None, None, (1, 1))
            .expect("Failed to generate thumbnails")
    } else {
        collect_thumbnail_into(second_thumbnail_dir.into(), None, (1, 1))
            .expect("Failed to collect thumbnails")
    };

    let tp = ThreadPoolBuilder::new()
        .build()
        .expect("Failed to create thread pool");

    let mictest_thumbnails = Mutex::new(Vec::new());
    tp.scope(|f| {
        for thumbnail in &second_thumbnail_info {
            let mictest_thumbnails = &mictest_thumbnails;
            f.spawn(move |_| {
                if image_file_is_mictest(&thumbnail.path).expect("Failed to check image") {
                    mictest_thumbnails.lock().unwrap().push(thumbnail.clone());
                }
            });
        }
    });
    let mut mictest_thumbnails = mictest_thumbnails.into_inner().unwrap();
    mictest_thumbnails.sort_by_key(|t| t.seq);

    let mictest_ranges =
        iter_continuous_range(mictest_thumbnails.iter(), |a, b| a.seq + 1 == b.seq)
            .filter(|(a, b)| b.timestamp.as_float_seconds() - a.timestamp.as_float_seconds() > 4.0);

    let accurate_mictest_ranges = Mutex::new(Vec::new());

    tp.scope(|f| {
        for (begin_rough, end_rough) in mictest_ranges {
            f.spawn(|_| {
                let begin_rough = begin_rough.clone();
                let end_rough = end_rough.clone();

                let begin_range_start = begin_rough.timestamp.add_seconds(-2);
                let begin_range_end = begin_rough.timestamp.add_seconds(2);
                let begin_test_thumbnail_dir = format!(
                    "data/thumbnails/{}/{}-{}",
                    id,
                    begin_range_start.as_ffmpeg_arg(),
                    begin_range_end.as_ffmpeg_arg()
                );
                let begin_thumbnails = if !std::path::Path::new(&begin_test_thumbnail_dir).exists()
                {
                    create_dir_all(&begin_test_thumbnail_dir)
                        .expect("Failed to create thumbnail directory");
                    generate_thumbnails(
                        &video_path,
                        begin_test_thumbnail_dir.into(),
                        Some(begin_range_start.clone()),
                        Some(begin_range_end.clone()),
                        (30, 1),
                    )
                    .expect("Failed to generate thumbnails")
                } else {
                    collect_thumbnail_into(
                        begin_test_thumbnail_dir.into(),
                        Some(begin_range_start.clone()),
                        (30, 1),
                    )
                    .expect("Failed to collect thumbnails")
                };
                let begin = begin_thumbnails
                    .iter()
                    .find(|t| image_file_is_mictest(&t.path).unwrap())
                    .unwrap();

                let end_range_start = end_rough.timestamp.add_seconds(-2);
                let end_range_end = end_rough.timestamp.add_seconds(2);
                let end_test_thumbnail_dir = format!(
                    "data/thumbnails/{}/{}-{}",
                    id,
                    end_range_start.as_ffmpeg_arg(),
                    end_range_end.as_ffmpeg_arg()
                );
                let end_thumbnails = if !std::path::Path::new(&end_test_thumbnail_dir).exists() {
                    create_dir_all(&end_test_thumbnail_dir)
                        .expect("Failed to create thumbnail directory");
                    generate_thumbnails(
                        &video_path,
                        end_test_thumbnail_dir.into(),
                        Some(end_range_start.clone()),
                        Some(end_range_end.clone()),
                        (30, 1),
                    )
                    .expect("Failed to generate thumbnails")
                } else {
                    collect_thumbnail_into(
                        end_test_thumbnail_dir.into(),
                        Some(end_range_start.clone()),
                        (30, 1),
                    )
                    .expect("Failed to collect thumbnails")
                };
                let end = end_thumbnails
                    .iter()
                    .tuple_windows()
                    .find(|(a, b)| {
                        image_file_is_mictest(&a.path).unwrap()
                            && !image_file_is_mictest(&b.path).unwrap()
                    })
                    .unwrap()
                    .0;

                accurate_mictest_ranges
                    .lock()
                    .unwrap()
                    .push((begin.timestamp.clone(), end.timestamp.clone()));
            });
        }
    });
    let mut accurate_mictest_ranges = accurate_mictest_ranges.into_inner().unwrap();
    accurate_mictest_ranges.sort_by_key(|x| x.0.clone());

    serde_json::to_writer(
        std::fs::File::create(&clips_file).expect("Failed to create clips file"),
        &ClipsInfo {
            ranges: accurate_mictest_ranges.clone(),
        },
    )
    .expect("Failed to write clips file");
}

fn cmd_find_clips(args: &FindClipsArgs) {
    let regex_complete = Regex::new("The Complete (.*) Season").unwrap();
    let mut waiting_for_from = args.from_id.clone();
    read_dir("urls/aftershow/")
        .unwrap()
        .into_iter()
        .chain(read_dir("urls/uploads/").unwrap())
        .map(|url_file| {
            let url_file = url_file.unwrap();

            let items = serde_json::from_reader::<_, PlaylistItemResponse>(
                std::fs::File::open(url_file.path()).unwrap(),
            );

            items.unwrap().items
        })
        .flatten()
        .filter(|x| match waiting_for_from {
            Some(ref from) => {
                if &x.content_details.video_id == from {
                    waiting_for_from = None;
                    true
                } else {
                    false
                }
            }
            None => true,
        })
        .filter(|x| {
            if x.content_details.video_id == "lED1vIbaivA" {
                return false;
            }
            if regex_complete.is_match(&x.snippet.title) {
                return false;
            }
            true
        })
        .for_each(|video| {
            info!("Processing {}", video.content_details.video_id);
            find_clips(&video.content_details.video_id, args);
        });
}

fn cmd_make_clips(args: &MakeClipsArgs) {
    let ids = if let Some(ref video_id) = args.video_id {
        vec![video_id.clone()]
    } else {
        read_dir("urls/aftershow/")
            .unwrap()
            .into_iter()
            .chain(read_dir("urls/uploads/").unwrap())
            .map(|url_file| {
                let url_file = url_file.unwrap();

                let items = serde_json::from_reader::<_, PlaylistItemResponse>(
                    std::fs::File::open(url_file.path()).unwrap(),
                );

                items.unwrap().items
            })
            .flatten()
            .map(|item| item.content_details.video_id)
            .collect::<Vec<_>>()
    };
    for id in ids {
        let clips_json_file = format!("data/clips/{}.json", id);
        if !Path::new(&clips_json_file).exists() {
            continue;
        }
        let clips_mkv_file = format!("data/clips/{}.mkv", id);
        let clips_info = serde_json::from_reader::<_, ClipsInfo>(
            std::fs::File::open(&clips_json_file).expect("Failed to open clips file"),
        )
        .expect("Failed to parse clips file");
        if clips_info.ranges.is_empty() {
            continue;
        }
        let input_file = [
            format!("data/videos/{}.mkv", id),
            format!("data/videos/{}.mp4", id),
        ]
        .into_iter()
        .find(|x| Path::new(x).exists())
        .unwrap();
        if !args.skip_existing_clips || !Path::new(&clips_mkv_file).exists() {
            make_multiple_clip(
                &input_file,
                &clips_mkv_file,
                clips_info
                    .ranges
                    .iter()
                    .sorted_by_key(|x| x.0.clone())
                    .cloned(),
                true,
                true,
            )
            .expect("Failed to make multiple clip");
        }
    }
}

fn cmd_concat() {
    let items = read_dir("urls/aftershow/")
        .unwrap()
        .into_iter()
        .chain(read_dir("urls/uploads/").unwrap())
        .map(|url_file| {
            let url_file = url_file.unwrap();

            let items = serde_json::from_reader::<_, PlaylistItemResponse>(
                std::fs::File::open(url_file.path()).unwrap(),
            );

            items.unwrap().items
        })
        .flatten()
        .sorted_by_key(|x| x.content_details.video_published_at)
        .filter_map(|item| {
            let mkv_path = format!("data/clips/{}.mkv", item.content_details.video_id);

            if Path::new(&mkv_path).exists() {
                Some((item.snippet.title, mkv_path))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    info!("Concatenating {} videos", items.len());

    let mut srt_file = std::fs::File::create("data/combined.srt").unwrap();
    let mut srt_seq = 1;
    let mut start_ts = VideoTimestamp::zero();
    for (title, path) in &items {
        let info = probe_format(&path).expect("Failed to probe format");
        let duration = info.format.duration.parse().unwrap();
        let duration = VideoTimestamp::from_float_seconds(duration);
        let end_ts = start_ts.clone() + duration;

        writeln!(
            srt_file,
            "{}\n{} --> {}\n{}\n",
            srt_seq,
            start_ts.as_ffmpeg_arg(),
            end_ts.as_ffmpeg_arg(),
            title,
        )
        .unwrap();

        start_ts = end_ts;
        srt_seq += 1;
    }

    concat_videos_filter(
        items.into_iter().map(|(_title, path)| path),
        "data/combined.mkv",
        Some("data/combined.srt"),
        false,
    )
    .expect("Failed to concat videos");
}

fn main() {
    env_logger::init();
    /*
    println!(
        "{:?}",
        image_file_is_mictest("data/thumbnails/-QUNwXd_QeQ/second/thumb0309.jpg")
    );
    return;
    */

    let cli = Cli::parse();
    match cli.subcommand {
        Commands::FindClips(ref args) => cmd_find_clips(args),
        Commands::MakeClips(ref args) => cmd_make_clips(args),
        Commands::Concat => cmd_concat(),
    }
}
