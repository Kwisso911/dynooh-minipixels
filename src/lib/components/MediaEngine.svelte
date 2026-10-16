<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n";
  import { settings } from "$lib/settings.svelte";
  import { view, fileUrl, pathUrl, setMeta, flash, step, position } from "$lib/files.svelte";
  import { playback, converted, mediaEl, setPark, stopPlayback, nextMedia, playItem } from "$lib/playback.svelte";

  /** True while the single view (tabs, or a file opened from tiles/list) is shown. */
  let { inViewer }: { inViewer: boolean } = $props();

  let park: HTMLDivElement;

  const current = $derived(view.items.find((it) => it.id === playback.id));
  const src = $derived.by(() => {
    if (!current) return "";
    const copy = converted[current.id];
    if (copy) return pathUrl(copy);
    return current.loaded ? fileUrl(current) : "";
  });

  $effect(() => {
    if (playback.id && !current) stopPlayback();
  });

  $effect(() => {
    if ((mediaEl.getAttribute("src") ?? "") === src) return;
    playback.time = 0;
    playback.duration = 0;
    if (src) {
      mediaEl.src = src;
    } else {
      mediaEl.removeAttribute("src");
      mediaEl.load();
    }
  });

  $effect(() => {
    void src;
    if (!src || playback.paused) {
      if (!mediaEl.paused) mediaEl.pause();
    } else if (mediaEl.paused) {
      mediaEl.play().catch((e: DOMException) => {
        if (e.name === "NotAllowedError") playback.paused = true;
      });
    }
  });

  $effect(() => {
    mediaEl.muted = settings.muted;
    mediaEl.volume = Math.min(1, Math.max(0, Number(settings.volume) || 0));
  });

  function select(id: number) {
    const i = view.items.findIndex((it) => it.id === id);
    if (i >= 0) view.active = i;
  }

  /** The single view steps through all files like the slideshow; otherwise the next audio/video
   *  file follows. Only the slideshow starts over at the end. */
  function onEnded() {
    playback.paused = true;
    const cur = current;
    if (!cur || !(settings.autoNext || settings.slideshow)) return;
    if (inViewer && view.items[view.active]?.id === cur.id) {
      const pos = position();
      if (settings.slideshow || (pos && pos[0] < pos[1])) step(1);
      return;
    }
    const next = nextMedia(cur.id, 1, settings.slideshow);
    if (!next) return;
    playItem(next);
    if (!inViewer) select(next.id);
  }

  function onError() {
    if (!mediaEl.getAttribute("src") || !playback.id) return;
    playback.paused = true;
    playback.failed = playback.id;
    if (!inViewer) flash(t("Datei kann nicht abgespielt werden."));
  }

  onMount(() => {
    setPark(park);
    const handlers: [string, () => void][] = [
      ["timeupdate", () => (playback.time = mediaEl.currentTime)],
      ["durationchange", () => (playback.duration = Number.isFinite(mediaEl.duration) ? mediaEl.duration : 0)],
      ["loadedmetadata", () => current && setMeta(current.id, { width: mediaEl.videoWidth, height: mediaEl.videoHeight, duration: mediaEl.duration })],
      ["ended", onEnded],
      ["error", onError],
    ];
    for (const [type, fn] of handlers) mediaEl.addEventListener(type, fn);
    return () => {
      for (const [type, fn] of handlers) mediaEl.removeEventListener(type, fn);
      setPark(null);
    };
  });
</script>

<div class="media-park" bind:this={park}></div>
