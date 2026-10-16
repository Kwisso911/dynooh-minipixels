<script lang="ts">
  import Thumb from "./Thumb.svelte";
  import PlayBadge from "./PlayBadge.svelte";
  import { settings } from "$lib/settings.svelte";
  import { view, thumbBucket, formatDuration, type Item } from "$lib/files.svelte";
  import { reorder } from "$lib/reorder";

  let { onmenu }: { onmenu: (e: MouseEvent, index: number) => void } = $props();

  let grid: HTMLDivElement;

  const bucket = $derived(thumbBucket(settings.tileSize));

  $effect(() => {
    void view.active;
    grid.querySelector(".tile.active")?.scrollIntoView({ block: "nearest" });
  });

  function meta(item: Item): string {
    const parts: string[] = [];
    if (item.width) parts.push(`${item.width} × ${item.height}`);
    if (item.duration) parts.push(formatDuration(item.duration));
    return parts.join(" · ") || item.ext.toUpperCase();
  }

  /** Number of tiles per row, for moving the selection up and down. */
  export function columns(): number {
    return grid ? getComputedStyle(grid).gridTemplateColumns.split(" ").length : 1;
  }
</script>

<div class="tiles" bind:this={grid} style:--tile="{settings.tileSize}px" use:reorder={"x"}>
  {#each view.items as item, i (item.id)}
    <button
      class="tile"
      data-index={i}
      class:active={i === view.active}
      title={item.path}
      onclick={() => (view.active = i)}
      ondblclick={() => {
        view.active = i;
        view.viewing = true;
      }}
      oncontextmenu={(e) => onmenu(e, i)}
    >
      <span class="thumb">
        <Thumb {item} size={bucket} iconSize={Math.round(settings.tileSize / 3)} />
        {#if item.kind !== "image"}<PlayBadge {item} />{/if}
      </span>
      <span class="tile-name">{item.name}</span>
      <span class="tile-meta">{meta(item)}</span>
    </button>
  {/each}
</div>
