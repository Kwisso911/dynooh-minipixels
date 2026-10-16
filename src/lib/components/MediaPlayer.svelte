<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Icon from "./Icon.svelte";
  import Thumb from "./Thumb.svelte";
  import { t } from "$lib/i18n";
  import { coverUrl, flash, type Item } from "$lib/files.svelte";
  import { codecPlayable } from "$lib/media";
  import { ffmpeg, checkFfmpeg, installFfmpeg, transcode, cancelTranscode } from "$lib/ffmpeg.svelte";
  import { playback, converted, playItem, togglePlay } from "$lib/playback.svelte";

  let { item, oncontextmenu }: { item: Item; oncontextmenu?: (e: MouseEvent) => void } = $props();

  /** Microsoft Store: the free edition is not offered on every PC, the paid one always is. */
  const HEVC_FREE = "ms-windows-store://pdp/?ProductId=9n4wgh0z6vhq";
  const HEVC_PAID = "ms-windows-store://pdp/?ProductId=9nmzlz57r3t7";

  let hasCover = $state(true);
  let tryAnyway = $state(false);
  let convertRequested = $state(false);
  let converting = $state(false);
  let progress = $state(0);
  let convertError = $state("");

  const isCurrent = $derived(playback.id === item.id);
  const failed = $derived(playback.failed === item.id);
  const copy = $derived(converted[item.id]);
  const hevc = $derived(item.codec === "HEVC");
  const unsupported = $derived(item.kind === "video" && !codecPlayable(item.codec));
  const showBox = $derived(!copy && item.kind === "video" && ((unsupported && !tryAnyway) || failed));
  /** HEVC can also be fixed with the Windows extension, so it is only converted on request. */
  const wantConvert = $derived(showBox && (convertRequested || (unsupported && !hevc)));

  checkFfmpeg();

  $effect(() => {
    if (wantConvert && ffmpeg.available && !converting && !convertError) untrack(convert);
  });

  onDestroy(() => {
    if (converting) cancelTranscode(item.path);
  });

  async function convert() {
    converting = true;
    progress = 0;
    try {
      converted[item.id] = await transcode(item.path, (r) => (progress = r));
      playItem(item);
    } catch (e) {
      convertError = String(e);
    } finally {
      converting = false;
    }
  }

  function requestConvert() {
    convertError = "";
    convertRequested = true;
    if (ffmpeg.available === false) installFfmpeg();
  }

  function playAnyway() {
    tryAnyway = true;
    playItem(item);
  }

  function store(url: string) {
    openUrl(url).catch((e) => flash(String(e)));
  }

  const mb = (bytes: number) => (bytes / 1024 / 1024).toFixed(0);
</script>

<div class="player" {oncontextmenu} role="region" aria-label={item.name}>
  {#if !item.loaded}
    <div class="viewer-msg">{t("Lade …")}</div>
  {:else if showBox}
    <div class="codec-box">
      <Icon name="video" size={40} />
      {#if converting}
        <strong>{t("Video wird umgewandelt …")}</strong>
        <div class="progress"><div style:width="{progress * 100}%"></div></div>
        <p class="muted small">{Math.round(progress * 100)} %</p>
      {:else}
        {#if hevc && !failed}
          <strong>{t("HEVC-Video (H.265)")}</strong>
          <p>{t("Für dieses Video braucht Windows die HEVC-Videoerweiterung von Microsoft. Nach der Installation MiniPixels neu starten.")}</p>
          <div class="codec-actions">
            <button class="primary" onclick={() => store(HEVC_FREE)}>{t("Gratis-Version (vom Gerätehersteller)")}</button>
            <button onclick={() => store(HEVC_PAID)}>{t("Kostenpflichtige Version")}</button>
          </div>
          <p class="muted small">{t("Die Gratis-Version ist nicht auf jedem PC verfügbar; dann die kostenpflichtige Version installieren.")}</p>
        {:else if unsupported && item.codec}
          <strong>{t("Video-Codec wird nicht direkt unterstützt")}</strong>
          <p>{t("Videos im Format «{codec}» kann MiniPixels nur nach einer Umwandlung abspielen.", { codec: t(item.codec) })}</p>
        {:else}
          <strong>{t("Datei kann nicht abgespielt werden.")}</strong>
          <p>{t("Eine Umwandlung nach H.264 kann helfen.")}</p>
        {/if}

        {#if convertError}
          <p class="error-text small">{t("Umwandlung fehlgeschlagen:")} {convertError}</p>
        {/if}
        {#if ffmpeg.installing}
          <p>{t("FFmpeg wird heruntergeladen …")}</p>
          <div class="progress"><div style:width="{ffmpeg.total ? (ffmpeg.received / ffmpeg.total) * 100 : 0}%"></div></div>
          {#if ffmpeg.total}<p class="muted small">{mb(ffmpeg.received)} / {mb(ffmpeg.total)} MB</p>{/if}
        {:else if ffmpeg.available}
          {#if !wantConvert || convertError}
            <button class:primary={!hevc} onclick={requestConvert}>{t("Umwandeln und abspielen")}</button>
          {/if}
        {:else if ffmpeg.available === false}
          <p class="muted small">{t("Dafür lädt MiniPixels einmalig FFmpeg herunter (ca. 75 MB).")}</p>
          {#if ffmpeg.error}<p class="error-text small">{ffmpeg.error}</p>{/if}
          <button class:primary={!hevc} onclick={requestConvert}>{t("FFmpeg herunterladen und umwandeln")}</button>
        {/if}

        {#if hevc && !failed}
          <button class="link" onclick={playAnyway}>{t("Trotzdem abspielen")}</button>
        {/if}
      {/if}
    </div>
  {:else if item.kind === "video"}
    <!-- The playing video itself is shown by the image viewer. -->
    <button class="video-poster" title={t("Abspielen")} onclick={() => playItem(item)}>
      <Thumb {item} size={512} iconSize={64} />
      <span class="big-play"><Icon name="play" size={34} /></span>
    </button>
  {:else}
    <div class="audio-card">
      <button class="cover-btn" title={isCurrent && !playback.paused ? t("Pause") : t("Abspielen")} onclick={() => togglePlay(item)}>
        {#if hasCover}
          <img class="cover" src={coverUrl(item)} alt="" draggable="false" onerror={() => (hasCover = false)} />
        {:else}
          <span class="cover cover-empty"><Icon name="music" size={96} /></span>
        {/if}
        {#if !isCurrent || playback.paused}<span class="big-play"><Icon name="play" size={34} /></span>{/if}
      </button>
      <div class="audio-title" title={item.path}>{item.name}</div>
      {#if failed}<div class="error-text">{t("Datei kann nicht abgespielt werden.")}</div>{/if}
    </div>
  {/if}
</div>
