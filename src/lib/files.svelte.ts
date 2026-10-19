import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { locale, t } from "./i18n";
import { settings } from "./settings.svelte";

export const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff"];
export const MEDIA_EXTENSIONS = ["mp3", "mp4", "mov"];
/** Above this many files, "new instance" mode asks before starting that many processes. */
const ASK_WINDOWS = 8;

export type Kind = "image" | "audio" | "video";

export interface Item {
  id: number;
  path: string;
  name: string;
  ext: string;
  kind: Kind;
  size: number;
  modified: number;
  width: number;
  height: number;
  /** Seconds, known once the media metadata has been read. */
  duration: number;
  format: string;
  /** Video codec of MP4/MOV files, e.g. "H.264" or "HEVC". */
  codec: string | null;
  error: string | null;
  /** Set once size and modification time are known; the file URLs depend on them. */
  loaded: boolean;
}

interface Info {
  path: string;
  size: number;
  modified: number;
  width: number;
  height: number;
  format: string;
  codec: string | null;
  error: string | null;
}

export type SortKey = "name" | "format" | "pixels" | "duration" | "size" | "modified" | "folder";

export const view = $state({
  items: [] as Item[],
  active: -1,
  /** Tiles/list: the active image is shown full size. */
  viewing: false,
  dragging: false,
  fullscreen: false,
  message: "",
});

/** Supported files per folder (key: lower-case folder path), for browsing with the arrow keys. */
const folders = $state<Record<string, string[]>>({});
const folderReadAt = new Map<string, number>();
/** A folder list younger than this is reused instead of reading the folder again. */
const FOLDER_MAX_AGE = 3000;

let nextId = 1;
let messageTimer: ReturnType<typeof setTimeout> | undefined;

const keyOf = (p: string) => p.toLowerCase();

export function dirOf(path: string): string {
  return path.slice(0, Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/")));
}

function folderOf(path: string): string[] {
  return folders[keyOf(dirOf(path))] ?? [];
}

/** Reads the supported files in the folder of `path`, unless that happened just before. */
export async function loadFolder(path: string): Promise<string[]> {
  const key = keyOf(dirOf(path));
  const at = folderReadAt.get(key);
  if (at && Date.now() - at < FOLDER_MAX_AGE && folders[key]) return folders[key];
  folderReadAt.set(key, Date.now());
  folders[key] = await invoke<string[]>("folder_images", { path }).catch(() => []);
  return folders[key];
}

function makeItem(path: string): Item {
  const name = path.split(/[\\/]/).pop() ?? path;
  const dot = name.lastIndexOf(".");
  const ext = dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  return {
    id: nextId++,
    path,
    name,
    ext,
    kind: ext === "mp3" ? "audio" : ext === "mp4" || ext === "mov" ? "video" : "image",
    size: 0,
    modified: 0,
    width: 0,
    height: 0,
    duration: 0,
    format: "",
    codec: null,
    error: null,
    loaded: false,
  };
}

export function flash(message: string) {
  view.message = message;
  clearTimeout(messageTimer);
  messageTimer = setTimeout(() => (view.message = ""), 5000);
}

async function loadInfo(paths: string[]) {
  for (let i = 0; i < paths.length; i += 64) {
    const chunk = paths.slice(i, i + 64);
    let infos: Info[];
    try {
      infos = await invoke<Info[]>("files_info", { paths: chunk });
    } catch (e) {
      infos = chunk.map((path) => ({ path, size: 0, modified: 0, width: 0, height: 0, format: "", codec: null, error: String(e) }));
    }
    const byKey = new Map(view.items.map((it) => [keyOf(it.path), it]));
    for (const info of infos) {
      const it = byKey.get(keyOf(info.path));
      if (it) Object.assign(it, info, { path: it.path, error: info.error ? t(info.error) : null, loaded: true });
    }
  }
}

/** Stores dimensions and duration read by the player or the preview of an audio/video file. */
export function setMeta(id: number, meta: { width?: number; height?: number; duration?: number }) {
  const it = view.items.find((i) => i.id === id);
  if (!it) return;
  if (meta.width && meta.height) {
    it.width = meta.width;
    it.height = meta.height;
  }
  if (meta.duration && Number.isFinite(meta.duration)) it.duration = meta.duration;
}

/** Adds files that are not open yet and makes the first given file the active one. */
function addPaths(paths: string[]) {
  const known = new Map(view.items.map((it) => [keyOf(it.path), it]));
  const fresh: Item[] = [];
  let first: Item | undefined;
  for (const p of paths) {
    const k = keyOf(p);
    let it = known.get(k);
    if (!it) {
      it = makeItem(p);
      fresh.push(it);
      known.set(k, it);
    }
    first ??= it;
  }
  if (fresh.length) view.items.push(...fresh);
  view.active = view.items.findIndex((it) => it.id === first?.id);
  if (!fresh.length) return;
  applySort();
  // Date, size and dimensions are only known once the info has arrived.
  void loadInfo(fresh.map((it) => it.path)).then(applySort);
}

async function showSingle(path: string) {
  view.items = [makeItem(path)];
  view.active = 0;
  void loadInfo([path]);
  await loadFolder(path);
}

/** Opens files and folders according to the chosen open mode. */
export async function openPaths(raw: string[]) {
  if (!raw.length) return;
  try {
    if (settings.openMode === "window") return await openAsWindows(raw);
    const paths = await invoke<string[]>("expand_paths", { paths: raw });
    if (!paths.length) return flash(t("Keine unterstützten Dateien gefunden."));
    addPaths(paths);
    view.viewing = false;
  } catch (e) {
    flash(String(e));
  }
}

/** One window per given file or folder; this window takes the first one if it is still empty. */
async function openAsWindows(raw: string[]) {
  const targets: { path: string; first: string }[] = [];
  for (const path of raw) {
    const files = await invoke<string[]>("expand_paths", { paths: [path] });
    if (files.length) targets.push({ path, first: files[0] });
  }
  if (!targets.length) return flash(t("Keine unterstützten Dateien gefunden."));
  let rest = targets;
  if (!view.items.length) {
    await showSingle(targets[0].first);
    rest = targets.slice(1);
  }
  if (!rest.length) return;
  if (rest.length > ASK_WINDOWS) {
    const ok = await ask(t("{n} neue Fenster öffnen?", { n: rest.length }), { title: "Dynooh MiniPixels", kind: "warning" });
    if (!ok) return;
  }
  await openInNewInstance(rest.map((r) => r.path));
}

export async function openInNewInstance(paths: string[]) {
  await invoke("open_in_new_instance", { paths }).catch((e) => flash(String(e)));
}

/** Opens copied files, or saves a copied image as PNG in the temp folder and opens that. */
export async function pasteClipboard() {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  const stamp = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}.${p(d.getMinutes())}.${p(d.getSeconds())}`;
  try {
    const paths = await invoke<string[]>("clipboard_paste", { name: `${t("Zwischenablage")} ${stamp}` });
    if (!paths.length) return flash(t("Die Zwischenablage enthält weder ein Bild noch Dateien."));
    await openPaths(paths);
  } catch (e) {
    flash(String(e));
  }
}

export async function openFilesDialog() {
  const picked = await open({
    multiple: true,
    filters: [
      { name: t("Alle unterstützten Dateien"), extensions: [...IMAGE_EXTENSIONS, ...MEDIA_EXTENSIONS] },
      { name: t("Bilder"), extensions: IMAGE_EXTENSIONS },
      { name: t("Audio und Video"), extensions: MEDIA_EXTENSIONS },
    ],
  });
  if (picked) await openPaths(Array.isArray(picked) ? picked : [picked]);
}

export async function openFolderDialog() {
  const picked = await open({ directory: true });
  if (typeof picked === "string") await openPaths([picked]);
}

/** New-instance mode with a single file steps through its folder instead of the open files. */
function folderMode(): boolean {
  const cur = view.items[view.active];
  return settings.openMode === "window" && view.items.length <= 1 && !!cur && folderOf(cur.path).length > 1;
}

/** Shows the file `delta` places further in the folder of the active file, in place of the active file. */
function stepInFolder(cur: Item, list: string[], delta: number) {
  const i = list.findIndex((p) => keyOf(p) === keyOf(cur.path));
  const next = list[(i + delta + list.length) % list.length];
  const open = view.items.findIndex((it) => keyOf(it.path) === keyOf(next));
  if (open >= 0) {
    view.active = open;
    return;
  }
  view.items[view.active] = makeItem(next);
  void loadInfo([next]);
}

/** Next/previous of the open files (Ctrl+Tab, slideshow, auto-next). */
export function step(delta: number) {
  const cur = view.items[view.active];
  if (!cur) return;
  if (folderMode()) return stepInFolder(cur, folderOf(cur.path), delta);
  const n = view.items.length;
  if (n > 1) view.active = (view.active + delta + n) % n;
}

/** Arrow keys: next/previous file in the folder of the shown file; falls back to the open files. */
export async function browse(delta: number) {
  const cur = view.items[view.active];
  if (!cur) return;
  const list = await loadFolder(cur.path);
  if (view.items[view.active]?.id !== cur.id) return;
  if (list.length > 1) stepInFolder(cur, list, delta);
  else step(delta);
}

export function select(index: number) {
  if (view.items.length) view.active = Math.max(0, Math.min(view.items.length - 1, index));
}

export function closeAt(index: number) {
  view.items.splice(index, 1);
  if (view.active > index || view.active >= view.items.length) view.active--;
  if (!view.items.length) {
    view.active = -1;
    view.viewing = false;
  }
}

/** 1-based position and total of the active file among the open files, or null if there is nothing to step through. */
export function position(): [number, number] | null {
  const cur = view.items[view.active];
  if (!cur) return null;
  if (folderMode()) return folderPosition();
  return view.items.length > 1 ? [view.active + 1, view.items.length] : null;
}

/** 1-based position and total of the active file in its folder, or null if the folder has no other file. */
export function folderPosition(): [number, number] | null {
  const cur = view.items[view.active];
  if (!cur) return null;
  const list = folderOf(cur.path);
  const i = list.findIndex((p) => keyOf(p) === keyOf(cur.path));
  return list.length > 1 && i >= 0 ? [i + 1, list.length] : null;
}

function comparator(key: SortKey): (a: Item, b: Item) => number {
  const coll = new Intl.Collator(locale(), { numeric: true, sensitivity: "base" });
  const byName = (a: Item, b: Item) => coll.compare(a.name, b.name);
  const cmp: Record<SortKey, (a: Item, b: Item) => number> = {
    name: byName,
    format: (a, b) => coll.compare(a.format || a.ext, b.format || b.ext) || byName(a, b),
    pixels: (a, b) => a.width * a.height - b.width * b.height || byName(a, b),
    duration: (a, b) => a.duration - b.duration || byName(a, b),
    size: (a, b) => a.size - b.size || byName(a, b),
    modified: (a, b) => a.modified - b.modified || byName(a, b),
    folder: (a, b) => coll.compare(dirOf(a.path), dirOf(b.path)) || byName(a, b),
  };
  return cmp[key];
}

/** Sorts the open files by the chosen key; the user's own order stays untouched. */
export function applySort() {
  const key = settings.sortKey;
  if (!key || view.items.length < 2) return;
  const cmp = comparator(key);
  const dir = settings.sortAsc ? 1 : -1;
  const activeId = view.items[view.active]?.id;
  const sorted = [...view.items].sort((a, b) => dir * cmp(a, b));
  if (sorted.every((it, i) => it === view.items[i])) return;
  view.items = sorted;
  view.active = sorted.findIndex((it) => it.id === activeId);
}

export function setSort(key: SortKey | "", asc = settings.sortAsc) {
  settings.sortKey = key;
  settings.sortAsc = asc;
  applySort();
}

/** Column header click: same column flips the direction. */
export function sortItems(key: SortKey) {
  setSort(key, settings.sortKey === key ? !settings.sortAsc : true);
}

/** Moves a file to another place; `to` is the insertion index before the move. Switches to the user's own order. */
export function moveItem(from: number, to: number) {
  if (to > from) to--;
  if (from === to || !view.items[from]) return;
  const activeId = view.items[view.active]?.id;
  const items = [...view.items];
  const [it] = items.splice(from, 1);
  items.splice(to, 0, it);
  view.items = items;
  view.active = items.findIndex((i) => i.id === activeId);
  settings.sortKey = "";
}

/** Modification time and size in the URL make the cached answer of the px protocol safe to keep. */
export function fileUrl(it: Item): string {
  return `${convertFileSrc(it.path, "px")}?m=${it.modified}-${it.size}`;
}

export function pathUrl(path: string): string {
  return convertFileSrc(path, "px");
}

export function thumbUrl(it: Item, px: number): string {
  return `${fileUrl(it)}&w=${px}`;
}

/** Embedded cover of an MP3. */
export function coverUrl(it: Item): string {
  return `${fileUrl(it)}&cover`;
}

/** "MOV · HEVC" for videos with a known codec, otherwise the file format. */
export function formatLabel(it: Item): string {
  const format = it.format || it.ext.toUpperCase();
  return it.codec ? `${format} · ${t(it.codec)}` : format;
}

export function formatDuration(seconds: number): string {
  const s = Math.round(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Few fixed thumbnail sizes so the browser cache can reuse them. */
export function thumbBucket(cssPx: number): number {
  const px = cssPx * (window.devicePixelRatio || 1);
  return px <= 64 ? 64 : px <= 128 ? 128 : px <= 256 ? 256 : 512;
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v.toLocaleString(locale(), { maximumFractionDigits: 1 })} ${units[u]}`;
}

export function formatDate(ms: number): string {
  return ms ? new Date(ms).toLocaleString(locale(), { dateStyle: "short", timeStyle: "short" }) : "";
}
