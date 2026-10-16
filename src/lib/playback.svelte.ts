import { view, flash, type Item } from "./files.svelte";
import { t } from "./i18n";
import { codecPlayable } from "./media";

/** The one audio/video file the app plays, in every view. */
export const playback = $state({
  id: 0,
  paused: true,
  time: 0,
  duration: 0,
  /** Id of the file that could not be played. */
  failed: 0,
  /** The single view shows the video itself, so the bar only shows a still. */
  bigView: false,
});

/** H.264 copies of videos the browser cannot play directly, by item id. */
export const converted = $state<Record<number, string>>({});

/** A single element for all views: moving it around keeps it playing, a new one would start over. */
export const mediaEl = document.createElement("video");
mediaEl.preload = "auto";

let park: HTMLElement | null = null;
const hosts: { node: HTMLElement; prio: number }[] = [];

function place() {
  let top: (typeof hosts)[number] | null = null;
  for (const h of hosts) if (!top || h.prio >= top.prio) top = h;
  const target = top?.node ?? park;
  if (target && mediaEl.parentNode !== target) target.appendChild(mediaEl);
  playback.bigView = !!top && top.prio > 0;
}

/** Hidden place for the element while no view shows it; it keeps playing there. */
export function setPark(node: HTMLElement | null) {
  park = node;
  place();
}

/** Svelte action: shows the media element inside the node. The highest priority wins. */
export function hostMedia(node: HTMLElement, prio = 0) {
  const host = { node, prio };
  hosts.push(host);
  place();
  return {
    destroy() {
      hosts.splice(hosts.indexOf(host), 1);
      place();
    },
  };
}

/** Mode switches must not take over a file that is already playing. */
let holdUntil = 0;
export function holdPlayback() {
  holdUntil = performance.now() + 500;
}
export function playbackHeld(): boolean {
  return performance.now() < holdUntil && !!playback.id && !playback.paused;
}

export function isMedia(it: Item | undefined): boolean {
  return !!it && it.kind !== "image";
}

/** Videos with a codec the browser lacks play only after a conversion in the single view. */
export function barPlayable(it: Item): boolean {
  return it.kind === "audio" || (it.kind === "video" && (codecPlayable(it.codec) || !!converted[it.id]));
}

export function playItem(it: Item) {
  playback.id = it.id;
  playback.paused = false;
  if (playback.failed === it.id) playback.failed = 0;
}

/** Pauses or resumes the given file if it is the current one, otherwise starts it. */
export function togglePlay(it: Item) {
  if (playback.id === it.id) playback.paused = !playback.paused;
  else if (barPlayable(it)) playItem(it);
  else flash(t("Dieses Video muss zuerst umgewandelt werden: Doppelklick öffnet es in der Einzelansicht."));
}

export function seek(seconds: number) {
  if (playback.id) mediaEl.currentTime = seconds;
}

export function stopPlayback() {
  playback.id = 0;
  playback.paused = true;
}

/** Next playable audio/video file after (or before) the given one; null if there is none. */
export function nextMedia(fromId: number, delta = 1, wrap = true): Item | null {
  const n = view.items.length;
  const start = view.items.findIndex((it) => it.id === fromId);
  for (let k = 1; k < n; k++) {
    const i = start + delta * k;
    if (!wrap && (i < 0 || i >= n)) return null;
    const it = view.items[((i % n) + n) % n];
    if (isMedia(it) && barPlayable(it)) return it;
  }
  return null;
}
