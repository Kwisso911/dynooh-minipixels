<script lang="ts">
  import Icon from "./Icon.svelte";
  import Thumb from "./Thumb.svelte";
  import { t } from "$lib/i18n";
  import { settings } from "$lib/settings.svelte";
  import { view, coverUrl, formatDuration, type Item } from "$lib/files.svelte";
  import { playback, isMedia, togglePlay, playItem, nextMedia, hostMedia, seek } from "$lib/playback.svelte";

  let coverFailed = $state(0);

  const current = $derived(view.items.find((it) => it.id === playback.id));
  const selected = $derived<Item | undefined>(view.items[view.active]);
  /** What the play button starts while nothing is loaded. */
  const target = $derived(current ?? (selected && isMedia(selected) ? selected : nextMedia(selected?.id ?? 0)));
  const playing = $derived(!!current && !playback.paused);
  const duration = $derived(current && playback.duration ? playback.duration : (target?.duration ?? 0));

  function select(id: number) {
    const i = view.items.findIndex((it) => it.id === id);
    if (i >= 0) view.active = i;
  }

  function skip(delta: number) {
    const from = current ?? target;
    const next = from && nextMedia(from.id, delta);
    if (!next) return;
    playItem(next);
    select(next.id);
  }
</script>

<div class="mediabar" class:idle={!current}>
  <button class="icon-btn" title={t("Vorheriger Titel")} disabled={!target} onclick={() => skip(-1)}><Icon name="skipBack" /></button>
  <button class="mb-play" title={playing ? t("Pause") : t("Abspielen")} disabled={!target} onclick={() => target && togglePlay(target)}>
    <Icon name={playing ? "pause" : "play"} size={20} />
  </button>
  <button class="icon-btn" title={t("Nächster Titel")} disabled={!target} onclick={() => skip(1)}><Icon name="skipForward" /></button>

  <button
    class="mb-preview"
    class:audio={current?.kind !== "video"}
    title={current?.path ?? ""}
    disabled={!current}
    onclick={() => current && select(current.id)}
    ondblclick={() => {
      if (!current) return;
      select(current.id);
      view.viewing = true;
    }}
  >
    {#if current?.kind === "video"}
      {#if playback.bigView}
        <Thumb item={current} size={128} iconSize={22} />
      {:else}
        <span class="mb-video" use:hostMedia={0}></span>
      {/if}
    {:else if current}
      {#if coverFailed !== current.id}
        <img src={coverUrl(current)} alt="" draggable="false" onerror={() => (coverFailed = current.id)} />
      {:else}
        <Icon name="music" size={22} />
      {/if}
    {:else}
      <Icon name={target?.kind === "video" ? "video" : "music"} size={22} />
    {/if}
  </button>

  <div class="mb-info">
    <span class="mb-title" title={(current ?? target)?.path ?? ""}>
      {(current ?? target)?.name ?? t("Keine Audio- oder Videodatei")}
    </span>
    <div class="mb-seek">
      <span>{formatDuration(current ? playback.time : 0)}</span>
      <input
        type="range"
        min="0"
        max={current ? playback.duration : 0}
        step="any"
        disabled={!current || !playback.duration}
        value={current ? playback.time : 0}
        oninput={(e) => seek(Number(e.currentTarget.value))}
        aria-label={t("Position")}
      />
      <span>{formatDuration(duration)}</span>
    </div>
  </div>

  <button class="icon-btn" title={settings.muted ? t("Ton einschalten") : t("Ton ausschalten")} onclick={() => (settings.muted = !settings.muted)}>
    <Icon name={settings.muted || settings.volume === 0 ? "volumeOff" : "volume"} />
  </button>
  <input class="mb-volume" type="range" min="0" max="1" step="0.01" bind:value={settings.volume} oninput={() => (settings.muted = false)} aria-label={t("Lautstärke")} />
</div>
