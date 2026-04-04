use rand::Rng;

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:138.0) Gecko/20100101 Firefox/138.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.7; rv:139.0) Gecko/20100101 Firefox/139.0",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
];

const FALLBACK_TZ: &str = "UTC";

/// Pick a random user agent from the built-in pool.
pub fn random_ua() -> &'static str {
    let idx = rand::rng().random_range(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

/// Detect the system's IANA timezone name.
///
/// Tries `TZ` env, `/etc/timezone`, `/etc/localtime` symlink.
/// Falls back to `"UTC"` when detection fails.
pub fn system_timezone() -> String {
    if let Some(tz) = tz_from_env() {
        return tz;
    }
    if let Some(tz) = tz_from_etc_timezone() {
        return tz;
    }
    if let Some(tz) = tz_from_localtime_link() {
        return tz;
    }
    FALLBACK_TZ.to_string()
}

fn tz_from_env() -> Option<String> {
    match std::env::var("TZ") {
        Ok(tz) => {
            let tz = tz.trim().to_string();
            if !tz.is_empty() && tz.contains('/') { Some(tz) } else { None }
        }
        Err(_) => None,
    }
}

fn tz_from_etc_timezone() -> Option<String> {
    match std::fs::read_to_string("/etc/timezone") {
        Ok(content) => {
            let tz = content.trim().to_string();
            if !tz.is_empty() && tz.contains('/') { Some(tz) } else { None }
        }
        Err(_) => None,
    }
}

fn tz_from_localtime_link() -> Option<String> {
    match std::fs::read_link("/etc/localtime") {
        Ok(link) => {
            let path = link.to_string_lossy();
            match path.split("/zoneinfo/").nth(1) {
                Some(tz) if !tz.is_empty() => Some(tz.to_string()),
                _ => None,
            }
        }
        Err(_) => None,
    }
}
