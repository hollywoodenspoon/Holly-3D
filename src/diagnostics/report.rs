//! The Markdown crash report, laid out so it can be pasted straight into
//! Claude Code: where it crashed, the last 50 user actions, the scene and
//! selection state, MiniMax's own stack frames, then the full backtrace.

use super::breadcrumbs::Crumb;
use std::time::{SystemTime, UNIX_EPOCH};

/// What went wrong.
#[derive(Clone, Debug, Default)]
pub struct CrashInfo {
    pub message: String,
    /// `src/.../file.rs:line:col`, when known.
    pub location: Option<String>,
    pub thread: String,
    /// `std::backtrace::Backtrace` text.
    pub backtrace: String,
    /// Seconds since 1970 (UTC).
    pub unix_time: u64,
    /// "panic", "signal SIGSEGV", "simulated test"...
    pub cause: String,
}

/// The program and machine.
#[derive(Clone, Debug, Default)]
pub struct BuildInfo {
    pub version: String,
    pub git: String,
    pub os: String,
    pub kernel: String,
    pub gpu: String,
}

impl BuildInfo {
    pub fn current(gpu: String) -> Self {
        let os = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| s.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())))
            .unwrap_or_else(|| std::env::consts::OS.to_string());
        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").map(|s| s.trim().to_string()).unwrap_or_else(|_| "unknown".into());
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            git: option_env!("MINIMAX_GIT_SHA").unwrap_or("unknown").into(),
            os: format!("{os} ({})", std::env::consts::ARCH),
            kernel,
            gpu,
        }
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `(year, month, day, hour, min, sec)` in UTC (Howard Hinnant's civil-from-days).
pub fn civil(unix: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, (secs / 3600) as u32, (secs / 60 % 60) as u32, (secs % 60) as u32)
}

/// `2026-11-11 21:05:09 UTC`.
pub fn timestamp(unix: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}

/// `20261111-210509` (for file names).
pub fn file_stamp(unix: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(unix);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

/// One backtrace frame: the function and its `at file:line:col` lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub function: String,
    pub at: Vec<String>,
}

/// Splits `std::backtrace` text into frames.
pub fn frames(backtrace: &str) -> Vec<Frame> {
    let mut out: Vec<Frame> = vec![];
    for line in backtrace.lines() {
        let t = line.trim();
        if let Some(at) = t.strip_prefix("at ") {
            if let Some(f) = out.last_mut() {
                f.at.push(at.to_string());
            }
        } else if let Some((num, rest)) = t.split_once(": ")
            && !num.is_empty()
            && num.chars().all(|c| c.is_ascii_digit())
        {
            out.push(Frame { function: rest.to_string(), at: vec![] });
        }
    }
    out
}

/// MiniMax's own frames (not the crash reporter's), innermost first.
pub fn minimax_frames(backtrace: &str) -> Vec<Frame> {
    frames(backtrace)
        .into_iter()
        .filter(|f| {
            let own = f.function.starts_with("minimax::") || f.function.starts_with("<minimax::");
            own && !f.function.contains("minimax::diagnostics::")
        })
        .collect()
}

/// The full Markdown report.
pub fn format(info: &CrashInfo, build: &BuildInfo, crumbs: Option<&[Crumb]>, scene: Option<&str>) -> String {
    let mut s = String::new();
    s.push_str("# MiniMax Crash Report\n\n");
    s.push_str("> Paste this whole file into Claude Code and ask it to find and fix the bug. ");
    s.push_str("Section 1 says where it crashed, section 2 what you were doing, section 4 which MiniMax functions were running.\n\n");
    s.push_str(&format!("- **When:** {}\n", timestamp(info.unix_time)));
    s.push_str(&format!("- **MiniMax:** {} (git `{}`)\n", build.version, build.git));
    s.push_str(&format!("- **System:** {}, kernel {}\n", build.os, build.kernel));
    s.push_str(&format!("- **GPU:** {}\n\n", build.gpu));

    s.push_str("## 1. Crash Summary & Location\n\n");
    s.push_str(&format!("- **Cause:** {}\n", info.cause));
    s.push_str(&format!("- **Thread:** `{}`\n", info.thread));
    let own = minimax_frames(&info.backtrace);
    let loc = info.location.clone().or_else(|| own.iter().find_map(|f| f.at.first().cloned()));
    s.push_str(&format!("- **Location:** `{}`\n\n", loc.as_deref().unwrap_or("unknown")));
    s.push_str("```text\n");
    s.push_str(info.message.trim_end());
    s.push_str("\n```\n\n");

    s.push_str("## 2. Last 50 User Actions (Flight Recorder Breadcrumbs)\n\n");
    match crumbs {
        Some([]) => s.push_str("_Nothing was recorded before the crash._\n"),
        Some(c) => {
            s.push_str("Oldest first; the last line is what happened just before the crash. Times are seconds since MiniMax started.\n\n");
            for (i, cr) in c.iter().enumerate() {
                s.push_str(&format!("{}. {}\n", i + 1, cr.line()));
            }
        }
        None => s.push_str("_The flight recorder was busy when the crash happened, so it could not be read._\n"),
    }
    s.push('\n');

    s.push_str("## 3. Active Scene & Selection State\n\n");
    s.push_str(scene.unwrap_or("_Not available (the crash happened before the first frame, or the state was being updated)._\n"));
    s.push('\n');

    s.push_str("## 4. MiniMax Stack Frames\n\n");
    if own.is_empty() {
        s.push_str("_No MiniMax frames. The crash was inside a library or driver; see the full backtrace._\n");
    } else {
        s.push_str("Innermost first.\n\n```text\n");
        for (i, f) in own.iter().enumerate() {
            s.push_str(&format!("{i:>3}: {}\n", f.function));
            for a in &f.at {
                s.push_str(&format!("       at {a}\n"));
            }
        }
        s.push_str("```\n");
    }
    s.push('\n');

    s.push_str("## 5. Full Backtrace\n\n```text\n");
    s.push_str(if info.backtrace.trim().is_empty() { "(empty)" } else { info.backtrace.trim_end() });
    s.push_str("\n```\n");
    s
}
