import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { SortKey } from "./files.svelte";

export type OpenMode = "window" | "tabs" | "tiles" | "list";
export type ThemeName = "light" | "dark" | "system";
export type Lang = "de" | "en";

export interface Settings {
  lang: Lang;
  theme: ThemeName;
  /** "window" makes every opened file start its own process (read by the backend at start). */
  openMode: OpenMode;
  tileSize: number;
  checker: boolean;
  upscale: boolean;
  /** Shared by every audio and video player. */
  muted: boolean;
  volume: number;
  rulers: boolean;
  guides: boolean;
  /** Read by the backend at start. */
  splash: boolean;
  /** Images advance by themselves; audio and video advance when they end. */
  slideshow: boolean;
  slideSeconds: number;
  /** Audio and video advance to the next file when they end. */
  autoNext: boolean;
  /** Empty: the user's own order (opening order or dragged). */
  sortKey: SortKey | "";
  sortAsc: boolean;
}

const CHOICES: Partial<Record<keyof Settings, readonly string[]>> = {
  lang: ["de", "en"],
  theme: ["light", "dark", "system"],
  openMode: ["window", "tabs", "tiles", "list"],
  sortKey: ["", "name", "format", "pixels", "duration", "size", "modified", "folder"],
};

const PALETTES = {
  dark: {
    bg: "#1e1f22",
    bg2: "#26282c",
    bg3: "#2f3136",
    line: "#3a3d43",
    text: "#d8dadf",
    muted: "#8a8f98",
    accent: "#3d8bfd",
    sel: "#24406b",
    hover: "#2a2c31",
    stage: "#161719",
    chk1: "#2a2b2e",
    chk2: "#3a3b3f",
    err: "#e5534b",
    guide: "#3fd3ee",
  },
  light: {
    bg: "#ffffff",
    bg2: "#f3f4f6",
    bg3: "#e7e9ed",
    line: "#d4d7dd",
    text: "#1f2328",
    muted: "#6b7280",
    accent: "#2563eb",
    sel: "#dbe6fd",
    hover: "#eceef1",
    stage: "#e8eaed",
    chk1: "#ffffff",
    chk2: "#dfe2e6",
    err: "#cf222e",
    guide: "#00a6c8",
  },
};

const media = window.matchMedia("(prefers-color-scheme: dark)");
const system = $state({ dark: media.matches });
media.addEventListener("change", () => (system.dark = media.matches));

export const settings = $state<Settings>({
  lang: "en",
  theme: "system",
  openMode: "tabs",
  tileSize: 180,
  checker: true,
  upscale: false,
  muted: true,
  volume: 1,
  rulers: false,
  guides: true,
  splash: true,
  slideshow: false,
  slideSeconds: 5,
  autoNext: false,
  sortKey: "",
  sortAsc: true,
});

/** Seconds per image in the slideshow, guarded against an empty or odd input. */
export function slideDelayMs(): number {
  const s = Number(settings.slideSeconds);
  return (Number.isFinite(s) && s >= 1 ? Math.min(s, 3600) : 5) * 1000;
}

let loaded = false;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
let windowTheme: string | undefined;

export async function loadSettings() {
  try {
    const stored = await invoke<Record<string, unknown>>("settings_get");
    const target = settings as unknown as Record<string, unknown>;
    for (const key of Object.keys(settings) as (keyof Settings)[]) {
      const v = stored?.[key];
      if (v === undefined || typeof v !== typeof settings[key]) continue;
      if (CHOICES[key] && !CHOICES[key].includes(v as string)) continue;
      target[key] = v;
    }
  } catch {
    // Defaults stay in place.
  }
  loaded = true;
}

export function isDark(): boolean {
  return settings.theme === "dark" || (settings.theme === "system" && system.dark);
}

function applyTheme() {
  const dark = isDark();
  const root = document.documentElement;
  for (const [k, v] of Object.entries(PALETTES[dark ? "dark" : "light"])) root.style.setProperty(`--${k}`, v);
  root.style.colorScheme = dark ? "dark" : "light";
  if (windowTheme !== settings.theme) {
    windowTheme = settings.theme;
    getCurrentWindow()
      .setTheme(settings.theme === "system" ? null : settings.theme)
      .catch(() => {});
  }
}

/** Applies the theme and stores every change; call once during component init. */
export function watchSettings() {
  $effect(() => {
    const snapshot = $state.snapshot(settings);
    applyTheme();
    if (!loaded) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => invoke("settings_set", { settings: snapshot }).catch(() => {}), 250);
  });
}
