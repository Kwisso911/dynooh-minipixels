use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::metadata::Orientation;
use image::{ColorType, DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use serde::Serialize;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::iter::Peekable;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::str::Chars;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::UNIX_EPOCH;
use tauri::http::Response;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "jpe", "jfif", "bmp", "dib", "gif", "tif", "tiff"];
const MEDIA_EXTENSIONS: &[&str] = &["mp3", "mp4", "mov"];
const THUMB_MAX: u32 = 1024;
/// Largest part of a media file sent per request; the player asks for the rest itself.
const RANGE_CHUNK: u64 = 4 * 1024 * 1024;

type Payload = (Vec<u8>, &'static str);

fn ext_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase()
}

fn is_media(path: &Path) -> bool {
    MEDIA_EXTENSIONS.contains(&ext_of(path).as_str())
}

/// Every file type the viewer opens: images, MP3 and MP4.
pub fn is_supported(path: &Path) -> bool {
    let ext = ext_of(path);
    IMAGE_EXTENSIONS.contains(&ext.as_str()) || MEDIA_EXTENSIONS.contains(&ext.as_str())
}

// ---------- Paths ----------

/// Supported files are kept, folders are replaced by the supported files they contain (not recursive).
pub fn expand(paths: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for p in paths {
        let path = Path::new(p);
        if path.is_dir() {
            out.extend(images_in(path));
        } else if path.is_file() && is_supported(path) {
            out.push(p.clone());
        }
    }
    out
}

fn images_in(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_supported(p))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.sort_by(|a, b| natural_cmp(a, b));
    files
}

pub fn siblings(file: &Path) -> Vec<String> {
    file.parent().map(images_in).unwrap_or_default()
}

/// Case-insensitive comparison where digit runs compare by value, so "img2" sorts before "img10".
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let n = take_digits(&mut x);
                let m = take_digits(&mut y);
                let (n, m) = (n.trim_start_matches('0'), m.trim_start_matches('0'));
                let ord = n.len().cmp(&m.len()).then_with(|| n.cmp(m));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(c), Some(d)) => {
                if c != d {
                    return c.cmp(&d);
                }
                x.next();
                y.next();
            }
        }
    }
}

fn take_digits(it: &mut Peekable<Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        s.push(c);
        it.next();
    }
    s
}

// ---------- Info ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    path: String,
    size: u64,
    modified: u64,
    width: u32,
    height: u32,
    format: String,
    /// Video codec of MP4/MOV files, e.g. "H.264" or "HEVC".
    codec: Option<String>,
    error: Option<String>,
}

pub fn info(path: &str) -> Info {
    let mut out = Info {
        path: path.into(),
        size: 0,
        modified: 0,
        width: 0,
        height: 0,
        format: String::new(),
        codec: None,
        error: None,
    };
    match std::fs::metadata(path) {
        Ok(meta) => {
            out.size = meta.len();
            out.modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as u64);
        }
        Err(e) => {
            out.error = Some(e.to_string());
            return out;
        }
    }
    if is_media(Path::new(path)) {
        out.format = ext_of(Path::new(path)).to_uppercase();
        if out.format != "MP3" {
            out.codec = catch_unwind(|| video_codec(Path::new(path))).ok().flatten();
        }
        return out;
    }
    match catch_unwind(|| probe(Path::new(path))) {
        Ok(Ok((w, h, format))) => {
            out.width = w;
            out.height = h;
            out.format = format;
        }
        Ok(Err(e)) => out.error = Some(e),
        Err(_) => out.error = Some("Bild konnte nicht gelesen werden".into()),
    }
    out
}

fn format_name(f: ImageFormat) -> String {
    match f {
        ImageFormat::Png => "PNG".into(),
        ImageFormat::Jpeg => "JPEG".into(),
        ImageFormat::Bmp => "BMP".into(),
        ImageFormat::Gif => "GIF".into(),
        ImageFormat::Tiff => "TIFF".into(),
        other => format!("{other:?}").to_uppercase(),
    }
}

fn open_reader(path: &Path) -> Result<ImageReader<BufReader<File>>, String> {
    let mut reader = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    reader.no_limits();
    Ok(reader)
}

/// Dimensions as displayed, i.e. with the EXIF orientation applied.
fn probe(path: &Path) -> Result<(u32, u32, String), String> {
    let reader = open_reader(path)?;
    let format = reader.format().map(format_name).unwrap_or_default();
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    let turned = matches!(
        decoder.orientation(),
        Ok(Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH)
    );
    Ok(if turned { (h, w, format) } else { (w, h, format) })
}

fn decode(path: &Path) -> Result<DynamicImage, String> {
    let mut decoder = open_reader(path)?.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    img.apply_orientation(orientation);
    Ok(img)
}

// ---------- Video codec ----------

/// Reads the sample entry of the first video track (moov/trak/mdia/minf/stbl/stsd) of an MP4 or MOV file.
fn video_codec(path: &Path) -> Option<String> {
    let mut f = File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let mut handler = [0u8; 4];
    find_codec(&mut f, 0, len, 0, &mut handler)
}

fn find_codec(f: &mut File, mut pos: u64, end: u64, depth: u8, handler: &mut [u8; 4]) -> Option<String> {
    while pos + 8 <= end {
        f.seek(SeekFrom::Start(pos)).ok()?;
        let mut head = [0u8; 8];
        f.read_exact(&mut head).ok()?;
        let mut size = u64::from(u32::from_be_bytes([head[0], head[1], head[2], head[3]]));
        let kind = [head[4], head[5], head[6], head[7]];
        let mut header = 8;
        if size == 1 {
            let mut large = [0u8; 8];
            f.read_exact(&mut large).ok()?;
            size = u64::from_be_bytes(large);
            header = 16;
        } else if size == 0 {
            size = end - pos;
        }
        if size < header {
            return None;
        }
        let body = pos + header;
        let box_end = (pos + size).min(end);
        match &kind {
            b"trak" if depth < 8 => {
                *handler = [0; 4];
                if let Some(c) = find_codec(f, body, box_end, depth + 1, handler) {
                    return Some(c);
                }
            }
            b"moov" | b"mdia" | b"minf" | b"stbl" if depth < 8 => {
                if let Some(c) = find_codec(f, body, box_end, depth + 1, handler) {
                    return Some(c);
                }
            }
            // QuickTime also has a data-handler `hdlr` inside `minf`; only the media one directly in `mdia` names the track type.
            b"hdlr" if handler == &[0; 4] => {
                // version/flags (4), pre_defined (4), handler_type (4)
                f.seek(SeekFrom::Start(body + 8)).ok()?;
                f.read_exact(handler).ok()?;
            }
            b"stsd" if handler == b"vide" => {
                // version/flags (4), entry_count (4), first entry: size (4), format (4)
                f.seek(SeekFrom::Start(body + 8)).ok()?;
                let mut entry = [0u8; 8];
                f.read_exact(&mut entry).ok()?;
                return Some(codec_name(&entry[4..8]));
            }
            _ => {}
        }
        pos += size;
    }
    None
}

fn codec_name(fourcc: &[u8]) -> String {
    match fourcc {
        b"avc1" | b"avc3" => "H.264".into(),
        b"hvc1" | b"hev1" | b"dvh1" | b"dvhe" => "HEVC".into(),
        b"av01" => "AV1".into(),
        b"vp09" => "VP9".into(),
        b"mp4v" => "MPEG-4 Part 2".into(),
        b"apcn" | b"apch" | b"apcs" | b"apco" | b"ap4h" | b"ap4x" => "ProRes".into(),
        b"rle " => "QuickTime Animation".into(),
        b"jpeg" | b"mjpa" | b"mjpb" => "Motion JPEG".into(),
        b"raw " | b"2vuy" | b"yuv2" | b"v210" | b"v308" | b"v408" | b"v410" | b"b48r" | b"b64a" => "Unkomprimiert".into(),
        b"dvc " | b"dvcp" | b"dvpp" | b"dv5n" | b"dv5p" | b"dvh5" | b"dvh6" => "DV".into(),
        b"vp08" => "VP8".into(),
        other => String::from_utf8_lossy(other).trim().to_string(),
    }
}

// ---------- px:// protocol ----------

pub fn thumbnail_size(query: Option<&str>) -> Option<u32> {
    query?
        .split('&')
        .find_map(|kv| kv.strip_prefix("w="))?
        .parse::<u32>()
        .ok()
        .map(|w| w.clamp(16, THUMB_MAX))
}

fn wants_cover(query: Option<&str>) -> bool {
    query.is_some_and(|q| q.split('&').any(|kv| kv == "cover"))
}

/// Serves `px://<encoded path>`: browser formats as they are, TIFF converted to PNG, `?w=` as thumbnail,
/// MP3/MP4 in byte ranges, `?cover` as the embedded MP3 cover.
pub fn serve(uri_path: &str, query: Option<&str>, range: Option<&str>) -> Response<Vec<u8>> {
    let decoded = percent_encoding::percent_decode_str(uri_path.trim_start_matches('/'))
        .decode_utf8_lossy()
        .into_owned();
    let file = PathBuf::from(decoded);
    if !is_supported(&file) {
        return respond(403, b"Kein unterstuetztes Dateiformat".to_vec(), "text/plain; charset=utf-8");
    }
    let size = thumbnail_size(query);
    if is_media(&file) {
        if size.is_none() && !wants_cover(query) {
            return serve_range(&file, range);
        }
        if ext_of(&file) != "mp3" {
            return respond(404, Vec::new(), "text/plain");
        }
    }
    let result = catch_unwind(AssertUnwindSafe(|| match size {
        Some(size) if is_media(&file) => cover(&file)
            .and_then(|data| image::load_from_memory(&data).map_err(|e| e.to_string()))
            .and_then(|img| encode_thumbnail(img, size)),
        Some(size) => decode(&file).and_then(|img| encode_thumbnail(img, size)),
        None if is_media(&file) => cover(&file).map(|data| {
            let mime = cover_mime(&data);
            (data, mime)
        }),
        None if matches!(ext_of(&file).as_str(), "tif" | "tiff") => decode(&file).and_then(encode_png),
        None => std::fs::read(&file).map(|b| (b, mime_of(&file))).map_err(|e| e.to_string()),
    }))
    .unwrap_or_else(|_| Err("Bild konnte nicht gelesen werden".into()));
    match result {
        Ok((body, mime)) => respond(200, body, mime),
        Err(e) if is_media(&file) => respond(404, e.into_bytes(), "text/plain; charset=utf-8"),
        Err(e) => respond(500, e.into_bytes(), "text/plain; charset=utf-8"),
    }
}

/// Embedded front cover of an MP3 (ID3v2 APIC frame), or the first picture if none is marked as cover.
fn cover(path: &Path) -> Result<Vec<u8>, String> {
    let tag = id3::Tag::read_from_path(path).map_err(|e| e.to_string())?;
    let pic = tag
        .pictures()
        .find(|p| p.picture_type == id3::frame::PictureType::CoverFront)
        .or_else(|| tag.pictures().next())
        .ok_or("Kein Cover")?;
    Ok(pic.data.clone())
}

fn cover_mime(data: &[u8]) -> &'static str {
    match image::guess_format(data) {
        Ok(ImageFormat::Png) => "image/png",
        Ok(ImageFormat::Jpeg) => "image/jpeg",
        Ok(ImageFormat::Gif) => "image/gif",
        Ok(ImageFormat::Bmp) => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Parses `bytes=a-b`, `bytes=a-` and `bytes=-n` into an inclusive range; no header means the whole file.
fn parse_range(header: Option<&str>, len: u64) -> Option<(u64, u64)> {
    let Some(spec) = header.and_then(|h| h.trim().strip_prefix("bytes=")) else {
        return Some((0, len.saturating_sub(1)));
    };
    let (a, b) = spec.split(',').next()?.split_once('-')?;
    let (a, b) = (a.trim(), b.trim());
    if a.is_empty() {
        let n: u64 = b.parse().ok()?;
        return Some((len.saturating_sub(n), len.saturating_sub(1)));
    }
    let start: u64 = a.parse().ok()?;
    let end = if b.is_empty() { len.saturating_sub(1) } else { b.parse::<u64>().ok()?.min(len.saturating_sub(1)) };
    (start <= end && start < len).then_some((start, end))
}

fn serve_range(file: &Path, range: Option<&str>) -> Response<Vec<u8>> {
    // MOV is served as MP4 so the Chromium MP4 demuxer (which also reads QuickTime files) handles it.
    let mime = match ext_of(file).as_str() {
        "mp3" => "audio/mpeg",
        _ => "video/mp4",
    };
    let read = || -> Result<Response<Vec<u8>>, String> {
        let mut f = File::open(file).map_err(|e| e.to_string())?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        let Some((start, end)) = parse_range(range, len) else {
            return Ok(Response::builder()
                .status(416)
                .header("Content-Range", format!("bytes */{len}"))
                .body(Vec::new())
                .unwrap_or_default());
        };
        let end = end.min(start + RANGE_CHUNK - 1);
        let mut body = vec![0; (end - start + 1) as usize];
        f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
        f.read_exact(&mut body).map_err(|e| e.to_string())?;
        Ok(Response::builder()
            .status(206)
            .header("Content-Type", mime)
            .header("Content-Length", body.len().to_string())
            .header("Content-Range", format!("bytes {start}-{end}/{len}"))
            .header("Accept-Ranges", "bytes")
            .header("Cache-Control", "no-cache")
            .header("Access-Control-Allow-Origin", "*")
            .body(body)
            .unwrap_or_default())
    };
    read().unwrap_or_else(|e| respond(500, e.into_bytes(), "text/plain; charset=utf-8"))
}

fn respond(status: u16, body: Vec<u8>, mime: &str) -> Response<Vec<u8>> {
    // The URL carries modification time and size, so a successful answer never changes.
    let cache = if status == 200 { "max-age=31536000, immutable" } else { "no-store" };
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Cache-Control", cache)
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap_or_default()
}

fn mime_of(path: &Path) -> &'static str {
    match ext_of(path).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" | "jfif" => "image/jpeg",
        "bmp" | "dib" => "image/bmp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

fn encode_thumbnail(img: DynamicImage, size: u32) -> Result<Payload, String> {
    let img = if img.width() > size || img.height() > size { img.thumbnail(size, size) } else { img };
    let mut buf = Cursor::new(Vec::new());
    if img.color().has_alpha() {
        img.to_rgba8()
            .write_with_encoder(PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::Adaptive))
            .map_err(|e| e.to_string())?;
        Ok((buf.into_inner(), "image/png"))
    } else {
        img.to_rgb8()
            .write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 85))
            .map_err(|e| e.to_string())?;
        Ok((buf.into_inner(), "image/jpeg"))
    }
}

fn encode_png(img: DynamicImage) -> Result<Payload, String> {
    let img = match img.color() {
        ColorType::L8
        | ColorType::La8
        | ColorType::Rgb8
        | ColorType::Rgba8
        | ColorType::L16
        | ColorType::La16
        | ColorType::Rgb16
        | ColorType::Rgba16 => img,
        _ => DynamicImage::ImageRgba8(img.to_rgba8()),
    };
    let mut buf = Cursor::new(Vec::new());
    img.write_with_encoder(PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::Adaptive))
        .map_err(|e| e.to_string())?;
    Ok((buf.into_inner(), "image/png"))
}

// ---------- Thumbnail workers ----------

type Job = Box<dyn FnOnce() + Send>;
static THUMBS: OnceLock<mpsc::Sender<Job>> = OnceLock::new();

/// Thumbnails run on a few fixed workers so a large folder does not decode hundreds of images at once.
pub fn spawn_thumbnail(job: impl FnOnce() + Send + 'static) {
    let tx = THUMBS.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(2, 8);
        for _ in 0..workers {
            let rx = Arc::clone(&rx);
            std::thread::spawn(move || {
                loop {
                    let job = rx.lock().map_err(|_| ()).and_then(|r| r.recv().map_err(|_| ()));
                    match job {
                        Ok(job) => job(),
                        Err(()) => break,
                    }
                }
            });
        }
        tx
    });
    let _ = tx.send(Box::new(job));
}
