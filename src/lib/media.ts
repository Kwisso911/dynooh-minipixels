export interface MediaPreview {
  /** JPEG data URL of a video frame; null for audio or if no frame could be read. */
  frame: string | null;
  width: number;
  height: number;
  duration: number;
}

/** Codecs that need a decoder Windows does not always have; H.264 is always present. */
const CODEC_TYPES: Record<string, string> = {
  HEVC: 'video/mp4; codecs="hvc1.1.6.L93.B0"',
  AV1: 'video/mp4; codecs="av01.0.05M.08"',
  VP9: 'video/mp4; codecs="vp09.00.10.08"',
};

let probe: HTMLVideoElement | undefined;

/** False for every video codec WebView2 cannot decode (ProRes, uncompressed, HEVC without the Windows extension, …). */
export function codecPlayable(codec: string | null): boolean {
  if (!codec || codec === "H.264") return true;
  const type = CODEC_TYPES[codec];
  if (!type) return false;
  probe ??= document.createElement("video");
  return probe.canPlayType(type) !== "";
}

/** Each preview loads a media element, so only a few run at the same time. */
const PARALLEL = 2;
const TIMEOUT_MS = 15000;

const cache = new Map<string, Promise<MediaPreview | null>>();
const waiting: (() => void)[] = [];
let running = 0;

/** Reads duration (and for `frameSize > 0` a video frame) of a media URL; results are cached per URL and size. */
export function mediaPreview(url: string, frameSize: number): Promise<MediaPreview | null> {
  const key = `${url}|${frameSize}`;
  let p = cache.get(key);
  if (!p) {
    p = new Promise((resolve) => {
      const run = () => {
        running++;
        read(url, frameSize)
          .then(resolve, () => resolve(null))
          .finally(() => {
            running--;
            waiting.shift()?.();
          });
      };
      if (running < PARALLEL) run();
      else waiting.push(run);
    });
    cache.set(key, p);
  }
  return p;
}

function read(url: string, frameSize: number): Promise<MediaPreview | null> {
  return new Promise((resolve) => {
    const el = document.createElement(frameSize ? "video" : "audio");
    // Without CORS the canvas is tainted and the frame cannot be exported.
    el.crossOrigin = "anonymous";
    el.muted = true;
    el.preload = "metadata";
    let timer = 0;
    const finish = (result: MediaPreview | null) => {
      clearTimeout(timer);
      el.removeAttribute("src");
      el.load();
      resolve(result);
    };
    timer = window.setTimeout(() => finish(null), TIMEOUT_MS);
    el.onerror = () => finish(null);
    el.onloadedmetadata = () => {
      if (!(el instanceof HTMLVideoElement)) return finish({ frame: null, width: 0, height: 0, duration: el.duration });
      el.currentTime = Math.min(1, (el.duration || 0) / 2) || 0.001;
    };
    if (el instanceof HTMLVideoElement) {
      el.onseeked = () => {
        const w = el.videoWidth;
        const h = el.videoHeight;
        let frame: string | null = null;
        if (w && h) {
          const s = Math.min(1, frameSize / Math.max(w, h));
          const canvas = document.createElement("canvas");
          canvas.width = Math.round(w * s);
          canvas.height = Math.round(h * s);
          canvas.getContext("2d")?.drawImage(el, 0, 0, canvas.width, canvas.height);
          try {
            frame = canvas.toDataURL("image/jpeg", 0.8);
          } catch {
            frame = null;
          }
        }
        finish({ frame, width: w, height: h, duration: el.duration });
      };
    }
    el.src = url;
  });
}
