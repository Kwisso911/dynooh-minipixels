import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
// @ts-expect-error type error without @types/node package
import os from "node:os";
// @ts-expect-error type error without @types/node package
import path from "node:path";
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [sveltekit()],

  // Dropbox / Virenscanner sperren Umbenennungen im Projektordner
  cacheDir: path.join(os.tmpdir(), "dynooh-minipixels-vite"),

  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1431,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
