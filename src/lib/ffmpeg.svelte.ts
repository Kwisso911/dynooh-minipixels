import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Shared by every player, so a running download shows everywhere. */
export const ffmpeg = $state({
  available: null as boolean | null,
  installing: false,
  received: 0,
  total: 0,
  error: "",
});

export async function checkFfmpeg() {
  if (ffmpeg.available === null) ffmpeg.available = await invoke<boolean>("ffmpeg_available").catch(() => false);
}

export async function installFfmpeg() {
  if (ffmpeg.installing) return;
  ffmpeg.installing = true;
  ffmpeg.error = "";
  ffmpeg.received = 0;
  ffmpeg.total = 0;
  const unlisten = await listen<{ received: number; total: number }>("ffmpeg-progress", (e) => {
    ffmpeg.received = e.payload.received;
    ffmpeg.total = e.payload.total;
  });
  try {
    await invoke("ffmpeg_install");
    ffmpeg.available = true;
  } catch (e) {
    ffmpeg.error = String(e);
  } finally {
    unlisten();
    ffmpeg.installing = false;
  }
}

/** Converts a video to H.264 and returns the path of the converted copy. */
export async function transcode(path: string, onProgress: (ratio: number) => void): Promise<string> {
  const unlisten = await listen<{ path: string; ratio: number }>("transcode-progress", (e) => {
    if (e.payload.path === path) onProgress(e.payload.ratio);
  });
  try {
    return await invoke<string>("transcode_video", { path });
  } finally {
    unlisten();
  }
}

export function cancelTranscode(path: string) {
  invoke("transcode_cancel", { path }).catch(() => {});
}
