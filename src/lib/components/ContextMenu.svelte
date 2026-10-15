<script lang="ts" module>
  /** `null` draws a separator. */
  export type MenuEntry = { label: string; hint?: string; action: () => unknown } | null;
</script>

<script lang="ts">
  import { flash } from "$lib/files.svelte";

  let { x, y, entries, onclose }: { x: number; y: number; entries: MenuEntry[]; onclose: () => void } = $props();

  let menu: HTMLDivElement;
  let left = $state(0);
  let top = $state(0);

  $effect(() => {
    const r = menu.getBoundingClientRect();
    left = Math.max(4, Math.min(x, innerWidth - r.width - 4));
    top = Math.max(4, Math.min(y, innerHeight - r.height - 4));
  });

  function run(action: () => unknown) {
    onclose();
    Promise.resolve()
      .then(action)
      .catch((e) => flash(String(e)));
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (!menu.contains(e.target as Node)) onclose();
  }}
  onblur={onclose}
  onresize={onclose}
/>

<div class="ctx" bind:this={menu} style:left="{left}px" style:top="{top}px" role="menu">
  {#each entries as entry}
    {#if entry}
      <button class="ctx-item" role="menuitem" onclick={() => run(entry.action)}>
        <span>{entry.label}</span>
        {#if entry.hint}<span class="ctx-hint">{entry.hint}</span>{/if}
      </button>
    {:else}
      <div class="ctx-sep"></div>
    {/if}
  {/each}
</div>
