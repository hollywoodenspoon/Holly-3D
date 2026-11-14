//! The black-box recorder's trigger: a panic hook plus (on Unix) handlers
//! for fatal signals. Either one writes the Markdown report and the
//! emergency scene once, then hands over to whatever ran before (Rust's
//! default panic message, its stack-overflow handler, or the OS default),
//! so the process still ends the way it would have.

use super::recovery::{self, Bundle};
use super::report::{self, BuildInfo, CrashInfo};
use super::{breadcrumbs, state};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    /// Set while `contain` runs a fallible third-party parser on this thread.
    static CONTAINED: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f`, turning a panic inside it into an error message instead of a
/// crash report (for file parsers that panic on input they do not support).
pub fn contain<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    let was = CONTAINED.with(|c| c.replace(true));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    CONTAINED.with(|c| c.set(was));
    r.map_err(|e| e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned()).unwrap_or_else(|| "unsupported data".into()))
}

/// Set once the first real crash has been written; later panics on other
/// threads (often knock-on effects) do not overwrite it.
static REPORTED: AtomicBool = AtomicBool::new(false);

/// Writes the report, the emergency scene and the pending marker into the
/// crashes folder. `info.backtrace`, `unix_time` and `thread` are filled in
/// by the caller.
pub fn write_crash(info: &CrashInfo) -> Option<Bundle> {
    let build = BuildInfo::current(state::gpu());
    let crumbs = breadcrumbs::try_snapshot();
    let summary = state::summary();
    let md = report::format(info, &build, crumbs.as_deref(), summary.as_deref());
    let dir = recovery::crash_dir();
    let stamp = report::file_stamp(info.unix_time);
    let res = state::with_scene(|s| recovery::write_bundle(&dir, &stamp, &md, Some(s)));
    let res = res.unwrap_or_else(|| recovery::write_bundle(&dir, &stamp, &md, None));
    match res {
        Ok(b) => {
            eprintln!("Ilex crash report written to {}", b.report.display());
            Some(b)
        }
        Err(e) => {
            eprintln!("Ilex could not write its crash report: {e}\n\n{md}");
            None
        }
    }
}

fn thread_name() -> String {
    let t = std::thread::current();
    t.name().map_or_else(|| format!("{:?}", t.id()), str::to_string)
}

/// Builds a report for a crash that did not happen (Help -> View Crash
/// Reports -> Simulate), so the whole path can be checked safely.
pub fn simulate() -> Option<Bundle> {
    breadcrumbs::record("Test", "Simulate Test Crash Report");
    let info = CrashInfo {
        message: "This is a simulated crash report. Ilex did not crash; the button only checks that reports are written.".into(),
        location: Some(format!("{}:{}:{}", file!(), line!(), column!())),
        thread: thread_name(),
        backtrace: std::backtrace::Backtrace::force_capture().to_string(),
        unix_time: report::unix_now(),
        cause: "simulated test (no crash happened)".into(),
    };
    write_crash(&info)
}

/// Installs the panic hook and the signal handlers. Call once, first thing
/// in `main`.
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        if CONTAINED.with(Cell::get) {
            return;
        }
        if !REPORTED.swap(true, Ordering::SeqCst) {
            let payload = p.payload();
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "(panic with a non-text payload)".into());
            let info = CrashInfo {
                message,
                location: p.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())),
                thread: thread_name(),
                backtrace: std::backtrace::Backtrace::force_capture().to_string(),
                unix_time: report::unix_now(),
                cause: "panic".into(),
            };
            write_crash(&info);
        }
        previous(p);
    }));
    #[cfg(unix)]
    signals::install();
}

#[cfg(unix)]
mod signals {
    //! A fatal signal can arrive on a nearly-full stack (or on the small
    //! alternate signal stack), so the handler does no real work itself: it
    //! starts a reporter thread with a fresh stack, waits for it a few
    //! seconds, puts the previous handler back and returns. The faulting
    //! instruction runs again (a signal sent by `kill` or `abort` is raised
    //! again) and the previous handler (or the default) ends the process as
    //! usual.
    use super::*;
    use std::sync::Mutex;

    const SIGNALS: [libc::c_int; 5] = [libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGFPE, libc::SIGABRT];
    static PREVIOUS: Mutex<Vec<(libc::c_int, libc::sigaction)>> = Mutex::new(Vec::new());
    static DONE: AtomicBool = AtomicBool::new(false);

    fn name(sig: libc::c_int) -> &'static str {
        match sig {
            libc::SIGSEGV => "SIGSEGV (invalid memory access)",
            libc::SIGBUS => "SIGBUS (bus error)",
            libc::SIGILL => "SIGILL (illegal instruction)",
            libc::SIGFPE => "SIGFPE (arithmetic fault)",
            libc::SIGABRT => "SIGABRT (abort)",
            _ => "unknown signal",
        }
    }

    pub fn install() {
        let mut prev = PREVIOUS.lock().unwrap_or_else(|p| p.into_inner());
        for sig in SIGNALS {
            // SAFETY: a zeroed sigaction is valid; the handler has the
            // SA_SIGINFO signature and lives for the whole program.
            unsafe {
                let mut act: libc::sigaction = std::mem::zeroed();
                act.sa_sigaction = handler as *const () as usize;
                act.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK | libc::SA_NODEFER;
                libc::sigemptyset(&mut act.sa_mask);
                let mut old: libc::sigaction = std::mem::zeroed();
                if libc::sigaction(sig, &act, &mut old) == 0 {
                    prev.push((sig, old));
                }
            }
        }
    }

    extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, _ctx: *mut libc::c_void) {
        // A signal sent by kill/raise/abort (si_code <= 0) does not happen
        // again when the handler returns, so it is raised again explicitly.
        // SAFETY: the kernel passes a valid siginfo with SA_SIGINFO.
        let sent = info.is_null() || unsafe { (*info).si_code } <= 0;
        // Put the previous handler back first so a second fault cannot loop.
        if let Ok(prev) = PREVIOUS.try_lock() {
            for (s, old) in prev.iter() {
                if *s == sig {
                    // SAFETY: restoring a sigaction the OS gave us.
                    unsafe { libc::sigaction(sig, old, std::ptr::null_mut()) };
                }
            }
        } else {
            // SAFETY: resetting to the default action is always valid.
            unsafe { libc::signal(sig, libc::SIG_DFL) };
        }
        if REPORTED.swap(true, Ordering::SeqCst) {
            reraise(sig, sent);
            return;
        }
        let thread = thread_name();
        let spawned = std::thread::Builder::new().name("ilex-crash-report".into()).stack_size(16 << 20).spawn(move || {
            let info = CrashInfo {
                message: format!("Ilex was stopped by {}.", name(sig)),
                location: None,
                thread,
                backtrace: "(A fatal signal cannot be unwound safely, so the crashing thread's backtrace is not available. \
                            The breadcrumbs above show what was happening; run Ilex from a terminal to see any driver message.)"
                    .into(),
                unix_time: report::unix_now(),
                cause: format!("signal {}", name(sig)),
            };
            write_crash(&info);
            DONE.store(true, Ordering::SeqCst);
        });
        if spawned.is_ok() {
            for _ in 0..100 {
                if DONE.load(Ordering::SeqCst) {
                    break;
                }
                let ts = libc::timespec { tv_sec: 0, tv_nsec: 50_000_000 };
                // SAFETY: nanosleep is async-signal-safe.
                unsafe { libc::nanosleep(&ts, std::ptr::null_mut()) };
            }
        }
        reraise(sig, sent);
    }

    fn reraise(sig: libc::c_int, sent: bool) {
        if sent {
            // SAFETY: signal and raise are async-signal-safe; SA_NODEFER
            // leaves `sig` unblocked, so the restored handler runs now. If it
            // returns (Rust's own SIGSEGV handler only resets to the default
            // and returns), the default action ends the process.
            unsafe {
                libc::raise(sig);
                libc::signal(sig, libc::SIG_DFL);
                libc::raise(sig);
            }
        }
    }
}
