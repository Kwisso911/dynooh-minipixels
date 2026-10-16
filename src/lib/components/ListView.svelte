<script lang="ts">
  import Thumb from "./Thumb.svelte";
  import PlayBadge from "./PlayBadge.svelte";
  import { t } from "$lib/i18n";
  import { settings } from "$lib/settings.svelte";
  import { reorder } from "$lib/reorder";
  import { view, thumbBucket, sortItems, formatSize, formatDate, formatDuration, formatLabel, dirOf, type SortKey } from "$lib/files.svelte";

  let { onmenu }: { onmenu: (e: MouseEvent, index: number) => void } = $props();

  const COLUMNS: { key: SortKey; label: string; num?: boolean }[] = [
    { key: "name", label: "Name" },
    { key: "format", label: "Format" },
    { key: "pixels", label: "Abmessungen", num: true },
    { key: "duration", label: "Dauer", num: true },
    { key: "size", label: "Grösse", num: true },
    { key: "modified", label: "Geändert" },
    { key: "folder", label: "Ordner" },
  ];

  let body: HTMLDivElement;

  const bucket = thumbBucket(36);

  $effect(() => {
    void view.active;
    body.querySelector("tr.active")?.scrollIntoView({ block: "nearest" });
  });
</script>

<div class="list-body" bind:this={body}>
  <table class="list">
    <thead>
      <tr>
        <th class="c-thumb"></th>
        {#each COLUMNS as col (col.key)}
          <th class:num={col.num} class:sorted={settings.sortKey === col.key} onclick={() => sortItems(col.key)}>
            {t(col.label)}
            {#if settings.sortKey === col.key}<span class="sort-arrow">{settings.sortAsc ? "▲" : "▼"}</span>{/if}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody use:reorder={"y"}>
      {#each view.items as item, i (item.id)}
        <tr
          data-index={i}
          class:active={i === view.active}
          onclick={() => (view.active = i)}
          ondblclick={() => {
            view.active = i;
            view.viewing = true;
          }}
          oncontextmenu={(e) => onmenu(e, i)}
        >
          <td class="c-thumb">
            <Thumb {item} size={bucket} iconSize={20} />
            {#if item.kind !== "image"}<PlayBadge {item} size={12} />{/if}
          </td>
          <td class="c-name" title={item.path}>{item.name}</td>
          <td>{formatLabel(item)}</td>
          <td class="num">{item.width ? `${item.width} × ${item.height}` : ""}</td>
          <td class="num">{item.duration ? formatDuration(item.duration) : ""}</td>
          <td class="num">{item.loaded ? formatSize(item.size) : ""}</td>
          <td>{formatDate(item.modified)}</td>
          <td class="c-dir" title={dirOf(item.path)}>{dirOf(item.path)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
