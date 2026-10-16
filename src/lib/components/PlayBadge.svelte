<script lang="ts">
  import Icon from "./Icon.svelte";
  import { t } from "$lib/i18n";
  import { view, type Item } from "$lib/files.svelte";
  import { playback, togglePlay } from "$lib/playback.svelte";

  let { item, size = 14 }: { item: Item; size?: number } = $props();

  const playing = $derived(playback.id === item.id && !playback.paused);
</script>

<!-- Inside the tile button, so it cannot be a button itself. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<span
  role="button"
  tabindex="-1"
  class="play-badge"
  class:on={playback.id === item.id}
  title={playing ? t("Pause") : t("Abspielen")}
  onclick={(e) => {
    e.stopPropagation();
    view.active = view.items.indexOf(item);
    togglePlay(item);
  }}
  ondblclick={(e) => e.stopPropagation()}
>
  <Icon name={playing ? "pause" : "play"} {size} />
</span>
