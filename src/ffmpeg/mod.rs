use std::ops::Add;

use serde::{Deserialize, Serialize};

pub mod clip;
pub mod concat;
pub mod probe;
pub mod thumbnail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTimestamp {
    seconds: u64,
    milliseconds: u64,
}

impl VideoTimestamp {
    pub const fn zero() -> Self {
        Self {
            seconds: 0,
            milliseconds: 0,
        }
    }
    pub fn from_float_seconds(seconds: f64) -> Self {
        let s = seconds.floor() as u64;
        let ms = ((seconds - seconds.floor()) * 1000.0).round() as u64;
        Self {
            seconds: s,
            milliseconds: ms,
        }
    }
    pub fn as_float_seconds(&self) -> f64 {
        self.seconds as f64 + self.milliseconds as f64 / 1000.0
    }
    pub fn add_seconds(&self, seconds: i64) -> Self {
        let seconds = self.seconds as i64 + seconds;
        Self {
            seconds: seconds as u64,
            milliseconds: self.milliseconds,
        }
    }
    pub fn as_hms(&self) -> (u64, u64, u64) {
        let hours = self.seconds / 3600;
        let minutes = (self.seconds - hours * 3600) / 60;
        let seconds = self.seconds - hours * 3600 - minutes * 60;
        (hours, minutes, seconds)
    }
    pub fn as_ffmpeg_arg(&self) -> String {
        let (hours, minutes, seconds) = self.as_hms();
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            hours, minutes, seconds, self.milliseconds
        )
    }
    pub fn normalize(&mut self) {
        self.seconds += self.milliseconds / 1000;
        self.milliseconds %= 1000;
    }
}

impl Add for VideoTimestamp {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut ret = Self {
            seconds: self.seconds + rhs.seconds,
            milliseconds: self.milliseconds + rhs.milliseconds,
        };
        ret.normalize();
        ret
    }
}

impl PartialEq for VideoTimestamp {
    fn eq(&self, other: &Self) -> bool {
        self.seconds == other.seconds && self.milliseconds == other.milliseconds
    }
}

impl Eq for VideoTimestamp {}

impl PartialOrd for VideoTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(match self.seconds.cmp(&other.seconds) {
            std::cmp::Ordering::Equal => self.milliseconds.cmp(&other.milliseconds),
            x => x,
        })
    }
}

impl Ord for VideoTimestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.seconds.cmp(&other.seconds) {
            std::cmp::Ordering::Equal => self.milliseconds.cmp(&other.milliseconds),
            x => x,
        }
    }
}
