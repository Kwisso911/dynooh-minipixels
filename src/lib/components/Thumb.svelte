<script lang="ts">
  import Icon from "./Icon.svelte";
  import { settings } from "$lib/settings.svelte";
  import { fileUrl, thumbUrl, setMeta, type Item } from "$lib/files.svelte";
  import { codecPlayable, mediaPreview } from "$lib/media";

  let { item, size, iconSize = 32 }: { item: Item; size: number; iconSize?: number } = $props();

  let failed = $state(false);
  let frame = $state<string | null>(null);

  $effect(() => {
    if (!item.loaded || item.kind === "image" || !codecPlayable(item.codec)) return;
    const id = item.id;
    let alive = true;
    mediaPreview(fileUrl(item), item.kind === "video" ? size : 0).then((p) => {
      if (!alive || !p) return;
      frame = p.frame;
      setMeta(id, p);
    });
    return () => {
      alive = false;
    };
  });
</script>

{#if !item.loaded}
  <span class="ext-badge">{item.ext}</span>
{:else if item.kind === "video"}
  {#if frame}
    <img src={frame} alt="" draggable="false" />
  {:else}
    <span class="media-icon"><Icon name="video" size={iconSize} /></span>
  {/if}
{:else if !failed && !item.error}
  <img
    src={thumbUrl(item, size)}
    alt=""
    loading="lazy"
    draggable="false"
    class:checker={settings.checker && item.kind === "image"}
    onerror={() => (failed = true)}
  />
{:else if item.kind === "audio"}
  <span class="media-icon"><Icon name="music" size={iconSize} /></span>
{:else}
  <span class="ext-badge">{item.ext}</span>
{/if}
