import { EN } from "./lang/en";
import { settings } from "./settings.svelte";

/** Translates German source text; `{name}` placeholders are filled from `params`. */
export function t(s: string, params?: Record<string, string | number>): string {
  const out = (settings.lang === "en" && EN[s]) || s;
  return params ? out.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m)) : out;
}

export const locale = () => (settings.lang === "de" ? "de-CH" : "en-GB");
