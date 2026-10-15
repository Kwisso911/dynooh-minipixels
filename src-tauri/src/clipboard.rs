use clipboard_win::{formats, get_clipboard, is_format_avail, register_format};
use image::ImageFormat;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Pasted images are kept this long, so they stay viewable across restarts for a while.
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Paths to open for the clipboard content: copied files, a copied path, or the image saved as PNG.
/// Empty if the clipboard holds nothing usable.
pub fn paste(name: &str) -> Result<Vec<String>, String> {
    if let Ok(files) = get_clipboard::<Vec<String>, _>(formats::FileList) {
        if !files.is_empty() {
            return Ok(files);
        }
    }
    // Browsers and the Snipping Tool also offer a PNG, which keeps transparency.
    if let Some(png) = register_format("PNG") {
        if is_format_avail(png.get()) {
            if let Ok(bytes) = get_clipboard::<Vec<u8>, _>(formats::RawData(png.get())) {
                if !bytes.is_empty() {
                    return save(name, &bytes).map(|p| vec![p]);
                }
            }
        }
    }
    if is_format_avail(formats::CF_DIB) || is_format_avail(formats::CF_BITMAP) {
        let bmp = get_clipboard::<Vec<u8>, _>(formats::Bitmap).map_err(|e| e.to_string())?;
        let img = image::load_from_memory_with_format(&bmp, ImageFormat::Bmp).map_err(|e| e.to_string())?;
        let mut png = Cursor::new(Vec::new());
        img.write_to(&mut png, ImageFormat::Png).map_err(|e| e.to_string())?;
        return save(name, png.get_ref()).map(|p| vec![p]);
    }
    if let Ok(text) = get_clipboard::<String, _>(formats::Unicode) {
        let path = text.trim().trim_matches('"');
        if !path.is_empty() && !path.contains('\n') && Path::new(path).exists() {
            return Ok(vec![path.to_string()]);
        }
    }
    Ok(Vec::new())
}

fn save(name: &str, png: &[u8]) -> Result<String, String> {
    let dir = std::env::temp_dir().join("Dynooh MiniPixels");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    remove_old(&dir);
    let safe: String = name.chars().map(|c| if r#"\/:*?"<>|"#.contains(c) { '-' } else { c }).collect();
    let mut path = dir.join(format!("{safe}.png"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{safe} ({n}).png"));
        n += 1;
    }
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn remove_old(dir: &PathBuf) {
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
