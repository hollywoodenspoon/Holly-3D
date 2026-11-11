//! Blocking HTTP GET for the asset downloader (runs on a worker thread).

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Largest single file accepted.
const MAX_BYTES: u64 = 1 << 30;

/// True for servers on this machine (the test server): no proxy for those.
fn is_local(url: &str) -> bool {
    let host = url.split("://").nth(1).unwrap_or(url);
    host.starts_with("127.0.0.1") || host.starts_with("localhost") || host.starts_with("[::1]")
}

/// A client that names only the program, never the user.
pub fn agent(for_url: &str) -> ureq::Agent {
    let b = ureq::AgentBuilder::new().user_agent("MiniMax-asset-downloader/0.1").timeout_connect(Duration::from_secs(20)).timeout_read(Duration::from_secs(60));
    if is_local(for_url) { b.build() } else { b.try_proxy_from_env(true).build() }
}

/// Downloads `url`. `on_bytes` hears every chunk; setting `cancel` stops
/// the transfer.
pub fn get(agent: &ureq::Agent, url: &str, cancel: &AtomicBool, on_bytes: &mut dyn FnMut(u64)) -> Result<Vec<u8>, String> {
    let resp = agent.get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("{url}: HTTP {code}"),
        other => format!("{url}: {other}"),
    })?;
    let mut reader = resp.into_reader().take(MAX_BYTES);
    let (mut out, mut buf) = (vec![], vec![0u8; 64 * 1024]);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let n = reader.read(&mut buf).map_err(|e| format!("{url}: {e}"))?;
        if n == 0 {
            return Ok(out);
        }
        out.extend_from_slice(&buf[..n]);
        on_bytes(n as u64);
    }
}

pub fn get_json(agent: &ureq::Agent, url: &str, cancel: &AtomicBool) -> Result<serde_json::Value, String> {
    let bytes = get(agent, url, cancel, &mut |_| {})?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{url}: {e}"))
}
