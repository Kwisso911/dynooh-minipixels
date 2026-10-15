#[cfg(windows)]
mod clipboard;
mod imaging;
mod transcode;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Must match `identifier` in tauri.conf.json: Tauri's app_config_dir is `%APPDATA%\<identifier>`.
const IDENTIFIER: &str = "ch.visual-fantastix.minipixels";
/// Starts a process that never hands its files over to an already running instance.
const NEW_INSTANCE_FLAG: &str = "--new-instance";

fn settings_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join(IDENTIFIER).join("settings.json"))
}

/// Read without an AppHandle because the open mode decides which plugins are registered.
fn read_settings() -> Value {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| Value::Object(Default::default()))
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

fn resolve_args(args: impl IntoIterator<Item = String>, cwd: &Path) -> Vec<String> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| {
            let p = PathBuf::from(&a);
            if p.is_absolute() { p } else { cwd.join(p) }
        })
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[tauri::command]
fn settings_get() -> Value {
    read_settings()
}

#[tauri::command]
fn settings_set(settings: Value) -> Result<(), String> {
    let path = settings_path().ok_or("APPDATA ist nicht gesetzt")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn startup_paths() -> Vec<String> {
    resolve_args(std::env::args().skip(1), &std::env::current_dir().unwrap_or_default())
}

#[tauri::command]
async fn expand_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    blocking(move || imaging::expand(&paths)).await
}

#[tauri::command]
async fn folder_images(path: String) -> Result<Vec<String>, String> {
    blocking(move || imaging::siblings(Path::new(&path))).await
}

#[tauri::command]
async fn files_info(paths: Vec<String>) -> Result<Vec<imaging::Info>, String> {
    blocking(move || paths.iter().map(|p| imaging::info(p)).collect()).await
}

#[tauri::command]
async fn clipboard_paste(name: String) -> Result<Vec<String>, String> {
    #[cfg(windows)]
    return blocking(move || clipboard::paste(&name)).await?;
    #[cfg(not(windows))]
    return Err(format!("{name}: clipboard is only supported on Windows"));
}

#[tauri::command]
fn ffmpeg_available() -> bool {
    transcode::find().is_some()
}

#[tauri::command]
async fn ffmpeg_install(app: AppHandle) -> Result<(), String> {
    blocking(move || transcode::install(&app)).await?
}

#[tauri::command]
async fn transcode_video(app: AppHandle, path: String) -> Result<String, String> {
    blocking(move || transcode::transcode(&app, &path)).await?
}

#[tauri::command]
fn transcode_cancel(path: String) {
    transcode::cancel(&path);
}

#[tauri::command]
fn open_in_new_instance(paths: Vec<String>) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    for p in paths {
        std::process::Command::new(&exe)
            .arg(NEW_INSTANCE_FLAG)
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

static STARTED: OnceLock<Instant> = OnceLock::new();
const SPLASH_MIN: Duration = Duration::from_millis(1600);

/// The window starts hidden so it never flashes in the wrong theme; the start screen stays up at least `SPLASH_MIN`.
#[tauri::command]
fn app_ready(app: AppHandle) {
    let wait = match (STARTED.get(), app.get_webview_window("splash")) {
        (Some(t), Some(_)) => SPLASH_MIN.saturating_sub(t.elapsed()),
        _ => Duration::ZERO,
    };
    std::thread::spawn(move || {
        std::thread::sleep(wait);
        let a = app.clone();
        let _ = app.run_on_main_thread(move || show_main(&a));
    });
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if !w.is_visible().unwrap_or(true) {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
}

fn open_splash(app: &tauri::App, stored: &Value) -> tauri::Result<()> {
    let lang = stored.get("lang").and_then(Value::as_str).unwrap_or("en").to_string();
    let mut builder = WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash.html".into()))
        .title("Dynooh MiniPixels")
        // Room around the 420×280 card for its shadow (30 px blur, 10 px down).
        .inner_size(520.0, 380.0)
        .center()
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .initialization_script(&format!("window.__MPX_LANG = {};", Value::String(lang)));
    // WebView2 only shares one browser process between webviews started with identical arguments.
    if let Some(args) = app.config().app.windows.first().and_then(|w| w.additional_browser_args.clone()) {
        builder = builder.additional_browser_args(&args);
    }
    builder.build()?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri_plugin_window_state::StateFlags;

    let stored = read_settings();
    let spawned = std::env::args().any(|a| a == NEW_INSTANCE_FLAG);
    let standalone = spawned || stored.get("openMode").and_then(Value::as_str) == Some("window");
    // Instances started from a running MiniPixels skip the start screen.
    let splash = !spawned && stored.get("splash").and_then(Value::as_bool) != Some(false);

    let mut builder = tauri::Builder::default();
    if !standalone {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let paths = resolve_args(argv.into_iter().skip(1), Path::new(&cwd));
            if !paths.is_empty() {
                let _ = app.emit("open-paths", paths);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .with_denylist(&["splash"])
                .build(),
        )
        .register_asynchronous_uri_scheme_protocol("px", |_ctx, request, responder| {
            let path = request.uri().path().to_string();
            let query = request.uri().query().map(str::to_string);
            let range = request.headers().get("range").and_then(|v| v.to_str().ok()).map(str::to_string);
            let thumbnail = imaging::thumbnail_size(query.as_deref()).is_some();
            let job = move || responder.respond(imaging::serve(&path, query.as_deref(), range.as_deref()));
            if thumbnail {
                imaging::spawn_thumbnail(job);
            } else {
                std::thread::spawn(job);
            }
        })
        .setup(move |app| {
            STARTED.get_or_init(Instant::now);
            if splash {
                open_splash(app, &stored)?;
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(5));
                let h = handle.clone();
                let _ = handle.run_on_main_thread(move || show_main(&h));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_ready,
            settings_get,
            settings_set,
            startup_paths,
            expand_paths,
            folder_images,
            files_info,
            clipboard_paste,
            ffmpeg_available,
            ffmpeg_install,
            transcode_video,
            transcode_cancel,
            open_in_new_instance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
