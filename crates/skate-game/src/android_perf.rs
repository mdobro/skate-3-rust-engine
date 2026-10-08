//! Android-only periodic perf line: fps, frame time p95 and process RSS.
//! Read with `adb logcat -s RustStdoutStderr:* | grep PERF` (see docs/android/rendering.md).
use crate::frame_timing::FrameTiming;
use bevy::prelude::*;
use std::time::Duration;

const PERIOD: Duration = Duration::from_secs(10);

pub(crate) struct AndroidPerfPlugin;
impl Plugin for AndroidPerfPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, log_perf);
    }
}

/// Resident set size in MiB from `/proc/self/statm` (second field, in pages).
pub(crate) fn rss_mib() -> Option<f64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    parse_rss_mib(&statm, 4096)
}

fn parse_rss_mib(statm: &str, page_bytes: u64) -> Option<f64> {
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some((pages * page_bytes) as f64 / (1024. * 1024.))
}

fn log_perf(time: Res<Time<Real>>, timing: Res<FrameTiming>, mut next: Local<Duration>) {
    let now = time.elapsed();
    if now < *next {
        return;
    }
    *next = now + PERIOD;
    let (fps, p95) = timing.window_fps_p95();
    let rss = rss_mib().map_or("n/a".to_owned(), |m| format!("{m:.0} MiB"));
    info!("PERF fps={fps:.1} frame_p95={p95:.1} ms rss={rss}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statm_resident_pages_to_mib() {
        assert_eq!(parse_rss_mib("1000 512 100 1 0 200 0", 4096), Some(2.0));
        assert_eq!(parse_rss_mib("garbage", 4096), None);
    }
}
