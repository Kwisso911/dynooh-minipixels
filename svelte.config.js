// Tauri has no Node.js server, so adapter-static with an index.html fallback runs the app as SPA.
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// MPX_WEB_OUT moves the output out of synced folders (Dropbox locks files while they are rewritten).
const out = process.env.MPX_WEB_OUT || "build";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: out,
      assets: out,
      fallback: "index.html",
    }),
  },
};

export default config;
