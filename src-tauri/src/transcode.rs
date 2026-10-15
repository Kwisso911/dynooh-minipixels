//! Converts videos WebView2 cannot decode to H.264 with FFmpeg, which is downloaded on request.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter};

/// LGPL builds call no GPL code; the shared variant is the smallest download.
const RELEASES: &str = "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest";
const FALLBACK_URL: &str =
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl-shared.zip";
/// Windows' own encoder first (hardware accelerated where available), OpenH264 as fallback.
const ENCODERS: &[&str] = &["h264_mf", "libopenh264"];
/// Converted videos are kept this long, so reopening a file does not convert it again.
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Serialize)]
struct DownloadProgress {
    received: u64,
    total: u64,
}

#[derive(Clone, Serialize)]
struct TranscodeProgress {
    path: String,
    ratio: f64,
}

fn base_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join(crate::IDENTIFIER)
}

fn running() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static RUNNING: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    RUNNING.get_or_init(Default::default)
}

/// The downloaded FFmpeg, otherwise one found on PATH.
pub fn find() -> Option<PathBuf> {
    let own = base_dir().join("ffmpeg").join("ffmpeg.exe");
    if own.is_file() {
        return Some(own);
    }
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).map(|d| d.join("ffmpeg.exe")).find(|p| p.is_file())
}

pub fn install(app: &AppHandle) -> Result<(), String> {
    let dir = base_dir().join("ffmpeg");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let url = latest_asset().unwrap_or_else(|| FALLBACK_URL.to_string());
    let response = ureq::get(&url).call().map_err(|e| e.to_string())?;
    let total = response.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let archive = dir.join("download.zip");
    let mut file = File::create(&archive).map_err(|e| e.to_string())?;
    let mut reader = response.into_reader();
    let mut buf = vec![0u8; 256 * 1024];
    let mut received = 0u64;
    let mut last = Instant::now();
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        received += n as u64;
        if last.elapsed() > Duration::from_millis(200) {
            last = Instant::now();
            let _ = app.emit("ffmpeg-progress", DownloadProgress { received, total });
        }
    }
    let _ = app.emit("ffmpeg-progress", DownloadProgress { received, total });
    drop(file);
    let extracted = extract(&archive, &dir);
    let _ = std::fs::remove_file(&archive);
    extracted?;
    if !dir.join("ffmpeg.exe").is_file() {
        return Err("ffmpeg.exe fehlt im Download".into());
    }
    Ok(())
}

/// Highest numbered release build, e.g. `ffmpeg-n9.0-latest-win64-lgpl-shared-9.0.zip`.
fn latest_asset() -> Option<String> {
    let release: Value = ureq::get(RELEASES).call().ok()?.into_json().ok()?;
    let mut best: Option<(Vec<u32>, String)> = None;
    for asset in release["assets"].as_array()? {
        let (Some(name), Some(url)) = (asset["name"].as_str(), asset["browser_download_url"].as_str()) else {
            continue;
        };
        let Some(rest) = name.strip_prefix("ffmpeg-n") else { continue };
        if !name.contains("-latest-win64-lgpl-shared-") || !name.ends_with(".zip") {
            continue;
        }
        let version: Vec<u32> = rest.split('-').next()?.split('.').filter_map(|v| v.parse().ok()).collect();
        if best.as_ref().is_none_or(|(b, _)| version > *b) {
            best = Some((version, url.to_string()));
        }
    }
    best.map(|(_, url)| url)
}

/// Keeps only `bin/ffmpeg.exe` and the DLLs it needs.
fn extract(archive: &Path, dir: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        let Some(file) = name.split("/bin/").nth(1) else { continue };
        if file.contains('/') || !(file == "ffmpeg.exe" || file.ends_with(".dll")) {
            continue;
        }
        let mut out = File::create(dir.join(file)).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Path of an H.264 copy of `path`, converted now or taken from the cache.
pub fn transcode(app: &AppHandle, path: &str) -> Result<String, String> {
    let ffmpeg = find().ok_or("FFmpeg ist nicht installiert")?;
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let mut hasher = DefaultHasher::new();
    path.to_lowercase().hash(&mut hasher);
    meta.len().hash(&mut hasher);
    meta.modified().ok().hash(&mut hasher);
    let key = format!("{:016x}", hasher.finish());

    let dir = base_dir().join("converted");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    remove_old(&dir);
    let out = dir.join(format!("{key}.mp4"));
    if out.is_file() {
        return Ok(out.to_string_lossy().into_owned());
    }

    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(previous) = running().lock().unwrap().insert(path.to_string(), cancel.clone()) {
        previous.store(true, Ordering::Relaxed);
    }
    let part = dir.join(format!("{key}.{}.part", std::process::id()));
    let mut error = String::new();
    for encoder in ENCODERS {
        match run(app, &ffmpeg, path, &part, encoder, &cancel) {
            Ok(()) => {
                error.clear();
                break;
            }
            Err(e) => error = e,
        }
        if cancel.load(Ordering::Relaxed) {
            break;
        }
    }
    {
        let mut map = running().lock().unwrap();
        if map.get(path).is_some_and(|c| Arc::ptr_eq(c, &cancel)) {
            map.remove(path);
        }
    }
    if !error.is_empty() {
        let _ = std::fs::remove_file(&part);
        return Err(error);
    }
    std::fs::rename(&part, &out).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().into_owned())
}

pub fn cancel(path: &str) {
    if let Some(flag) = running().lock().unwrap().get(path) {
        flag.store(true, Ordering::Relaxed);
    }
}

fn run(app: &AppHandle, ffmpeg: &Path, input: &str, output: &Path, encoder: &str, cancel: &AtomicBool) -> Result<(), String> {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-nostdin", "-y", "-i", input, "-map", "0:v:0", "-map", "0:a:0?"])
        // yuv420p needs even dimensions and is the only pixel format every H.264 decoder accepts.
        .args(["-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p", "-c:v", encoder]);
    if encoder == "h264_mf" {
        cmd.args(["-rate_control", "quality", "-quality", "90"]);
    } else {
        cmd.args(["-b:v", "12M"]);
    }
    cmd.args(["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", "-f", "mp4", "-nostats", "-progress", "pipe:1"])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    let duration = Arc::new(Mutex::new(None::<f64>));
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr = child.stderr.take().ok_or("no stderr")?;
    let reader = {
        let duration = duration.clone();
        let log = log.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(rest) = line.trim_start().strip_prefix("Duration:") {
                    let mut d = duration.lock().unwrap();
                    if d.is_none() {
                        *d = parse_time(rest.split(',').next().unwrap_or("").trim());
                    }
                }
                let mut log = log.lock().unwrap();
                log.push(line);
                if log.len() > 8 {
                    log.remove(0);
                }
            }
        })
    };

    let stdout = child.stdout.take().ok_or("no stdout")?;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            break;
        }
        let Some(us) = line.strip_prefix("out_time_us=").and_then(|v| v.parse::<f64>().ok()) else { continue };
        if let Some(total) = *duration.lock().unwrap() {
            if total > 0.0 {
                let ratio = (us / 1_000_000.0 / total).clamp(0.0, 1.0);
                let _ = app.emit("transcode-progress", TranscodeProgress { path: input.to_string(), ratio });
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = reader.join();
    if cancel.load(Ordering::Relaxed) {
        return Err("Abgebrochen".into());
    }
    if status.success() {
        Ok(())
    } else {
        Err(log.lock().unwrap().join("\n"))
    }
}

fn parse_time(text: &str) -> Option<f64> {
    let mut seconds = 0.0;
    for part in text.split(':') {
        seconds = seconds * 60.0 + part.parse::<f64>().ok()?;
    }
    Some(seconds)
}

fn remove_old(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > KEEP);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
