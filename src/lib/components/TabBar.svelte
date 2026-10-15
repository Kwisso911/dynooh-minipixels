<script lang="ts">
  import Icon from "./Icon.svelte";
  import Thumb from "./Thumb.svelte";
  import { t } from "$lib/i18n";
  import { view, closeAt } from "$lib/files.svelte";

  let { onmenu }: { onmenu: (e: MouseEvent, index: number) => void } = $props();

  let bar: HTMLElement;

  $effect(() => {
    void view.active;
    bar.querySelector(".tab.active")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });

  function onWheel(e: WheelEvent) {
    if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) bar.scrollLeft += e.deltaY;
  }
</script>

<nav class="tabbar" bind:this={bar} onwheel={onWheel}>
  {#each view.items as item, i (item.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="tab"
      class:active={i === view.active}
      title={item.path}
      onmousedown={(e) => e.button === 1 && e.preventDefault()}
      onauxclick={(e) => e.button === 1 && closeAt(i)}
      oncontextmenu={(e) => onmenu(e, i)}
    >
      <button class="tab-label" onclick={() => (view.active = i)}>
        <span class="tab-thumb"><Thumb {item} size={64} iconSize={14} /></span>
        <span>{item.name}</span>
      </button>
      <button class="tab-close" title="{t('Schliessen')} (Ctrl+W)" onclick={() => closeAt(i)}>
        <Icon name="close" size={12} />
      </button>
    </div>
  {/each}
</nav>
