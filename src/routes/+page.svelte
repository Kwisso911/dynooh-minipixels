<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import Icon, { type IconName } from "$lib/components/Icon.svelte";
  import Viewer from "$lib/components/Viewer.svelte";
  import MediaPlayer from "$lib/components/MediaPlayer.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import TilesView from "$lib/components/TilesView.svelte";
  import ListView from "$lib/components/ListView.svelte";
  import SettingsDialog from "$lib/components/SettingsDialog.svelte";
  import ContextMenu, { type MenuEntry } from "$lib/components/ContextMenu.svelte";
  import { t } from "$lib/i18n";
  import { hasGuides, clearGuides } from "$lib/guides.svelte";
  import { settings, loadSettings, watchSettings, isDark, type OpenMode } from "$lib/settings.svelte";
  import {
    view,
    step,
    select,
    closeAt,
    position,
    openPaths,
    openFilesDialog,
    openFolderDialog,
    openInNewInstance,
    pasteClipboard,
    formatSize,
    formatDuration,
    formatLabel,
  } from "$lib/files.svelte";

  watchSettings();

  const MODES: { id: OpenMode; icon: IconName; label: string }[] = [
    { id: "window", icon: "window", label: "Neue Instanz" },
    { id: "tabs", icon: "tabs", label: "Tabs" },
    { id: "tiles", icon: "tiles", label: "Kacheln" },
    { id: "list", icon: "list", label: "Liste" },
  ];

  let viewer = $state<ReturnType<typeof Viewer>>();
  let player = $state<ReturnType<typeof MediaPlayer>>();
  let tilesView = $state<ReturnType<typeof TilesView>>();
  let scale = $state(1);
  let showSettings = $state(false);
  let menu = $state<{ x: number; y: number; entries: MenuEntry[] } | null>(null);

  const item = $derived(view.items[view.active]);
  const overview = $derived(settings.openMode === "tiles" || settings.openMode === "list");
  const showViewer = $derived(!!item && (!overview || view.viewing));
  const showImage = $derived(showViewer && item?.kind === "image");
  const pos = $derived(position());
  const dark = $derived(isDark());

  $effect(() => {
    const title = item ? `${item.name} – Dynooh MiniPixels` : "Dynooh MiniPixels";
    getCurrentWindow()
      .setTitle(title)
      .catch(() => {});
  });

  onMount(() => {
    const unlisten: (() => void)[] = [];
    (async () => {
      await loadSettings();
      unlisten.push(await listen<string[]>("open-paths", (e) => openPaths(e.payload)));
      unlisten.push(
        await getCurrentWebview().onDragDropEvent((e) => {
          const p = e.payload;
          if (p.type === "enter" || p.type === "over") {
            view.dragging = true;
          } else {
            view.dragging = false;
            if (p.type === "drop" && p.paths.length) openPaths(p.paths);
          }
        }),
      );
      const start = await invoke<string[]>("startup_paths").catch(() => [] as string[]);
      void openPaths(start);
      await tick();
      await invoke("app_ready");
    })();
    return () => unlisten.forEach((f) => f());
  });

  function setMode(mode: OpenMode) {
    settings.openMode = mode;
    view.viewing = false;
  }

  async function toggleFullscreen() {
    const w = getCurrentWindow();
    const next = !(await w.isFullscreen());
    await w.setFullscreen(next);
    view.fullscreen = next;
  }

  function closeLabel(): string {
    if (settings.openMode === "tabs") return t("Tab schliessen");
    if (settings.openMode === "window") return t("Schliessen");
    return t("Aus Liste entfernen");
  }

  function openMenu(e: MouseEvent, index: number) {
    e.preventDefault();
    const it = view.items[index];
    if (!it) return;
    view.active = index;
    const entries: MenuEntry[] = [];
    if (overview && !view.viewing) entries.push({ label: t("Anzeigen"), hint: "Enter", action: () => (view.viewing = true) });
    if (showImage && it === item) {
      entries.push(
        { label: settings.rulers ? t("Lineale ausblenden") : t("Lineale einblenden"), hint: "Ctrl+R", action: () => (settings.rulers = !settings.rulers) },
        { label: settings.guides ? t("Hilfslinien ausblenden") : t("Hilfslinien einblenden"), hint: "Ctrl+H", action: () => (settings.guides = !settings.guides) },
      );
      if (hasGuides(it.id)) entries.push({ label: t("Hilfslinien löschen"), action: () => clearGuides(it.id) });
      entries.push(null);
    }
    entries.push(
      { label: t("In neuem Fenster öffnen"), action: () => openInNewInstance([it.path]) },
      { label: t("Im Explorer zeigen"), action: () => revealItemInDir(it.path) },
      { label: t("Pfad kopieren"), action: () => navigator.clipboard.writeText(it.path) },
      null,
      { label: closeLabel(), hint: "Ctrl+W", action: () => closeAt(view.items.indexOf(it)) },
    );
    menu = { x: e.clientX, y: e.clientY, entries };
  }

  function onKey(e: KeyboardEvent) {
    // Ctrl+R would otherwise reload the whole app.
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r") e.preventDefault();
    if (menu) {
      if (e.key === "Escape") menu = null;
      return;
    }
    if (showSettings) {
      if (e.key === "Escape") showSettings = false;
      return;
    }
    if ((e.target as HTMLElement).closest("input, select, textarea, video, audio")) return;

    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key;
    if (ctrl && key.toLowerCase() === "o") {
      e.preventDefault();
      if (e.shiftKey) openFolderDialog();
      else openFilesDialog();
      return;
    }
    if (ctrl && key.toLowerCase() === "v") {
      e.preventDefault();
      pasteClipboard();
      return;
    }
    if (ctrl && key === ",") {
      e.preventDefault();
      showSettings = true;
      return;
    }
    if (ctrl && key.toLowerCase() === "w") {
      e.preventDefault();
      if (item) closeAt(view.active);
      return;
    }
    if (ctrl && (key.toLowerCase() === "r" || key.toLowerCase() === "h")) {
      e.preventDefault();
      if (!showImage) return;
      if (key.toLowerCase() === "r") settings.rulers = !settings.rulers;
      else settings.guides = !settings.guides;
      return;
    }
    if (ctrl && key === "Tab") {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
      return;
    }
    if (key === "F11") {
      e.preventDefault();
      toggleFullscreen();
      return;
    }
    if (key === "Escape" && view.fullscreen) {
      toggleFullscreen();
      return;
    }
    if (!item || ctrl) return;

    if (showViewer) {
      switch (key) {
        case "ArrowLeft":
        case "PageUp":
          step(-1);
          break;
        case "ArrowRight":
        case "PageDown":
          step(1);
          break;
        case " ":
          if (player) player.toggle();
          else step(1);
          break;
        case "+":
        case "=":
          viewer?.zoomBy(1.25);
          break;
        case "-":
          viewer?.zoomBy(0.8);
          break;
        case "0":
          viewer?.fit();
          break;
        case "1":
          viewer?.actual();
          break;
        case "r":
          viewer?.rotate(90);
          break;
        case "R":
          viewer?.rotate(-90);
          break;
        case "Escape":
        case "Backspace":
          if (!view.viewing) return;
          view.viewing = false;
          break;
        case "Delete":
          if (!viewer?.deleteSelectedGuide()) closeAt(view.active);
          break;
        default:
          return;
      }
    } else {
      const cols = settings.openMode === "tiles" ? (tilesView?.columns() ?? 1) : 1;
      switch (key) {
        case "ArrowLeft":
          if (cols > 1) select(view.active - 1);
          break;
        case "ArrowRight":
          if (cols > 1) select(view.active + 1);
          break;
        case "ArrowUp":
          select(view.active - cols);
          break;
        case "ArrowDown":
          select(view.active + cols);
          break;
        case "Home":
          select(0);
          break;
        case "End":
          select(view.items.length - 1);
          break;
        case "Enter":
        case " ":
          view.viewing = true;
          break;
        case "Delete":
          closeAt(view.active);
          break;
        default:
          return;
      }
    }
    e.preventDefault();
  }
</script>

<svelte:window
  onkeydown={onKey}
  oncontextmenu={(e) => {
    if (!(e.target as HTMLElement).closest("input, textarea")) e.preventDefault();
  }}
/>

<div class="app" class:fullscreen={view.fullscreen}>
  <header class="toolbar">
    <button class="icon-btn" title="{t('Dateien öffnen')} (Ctrl+O)" onclick={openFilesDialog}><Icon name="image" /></button>
    <button class="icon-btn" title="{t('Ordner öffnen')} (Ctrl+Shift+O)" onclick={openFolderDialog}><Icon name="folder" /></button>
    <button class="icon-btn" title="{t('Aus Zwischenablage einfügen')} (Ctrl+V)" onclick={pasteClipboard}><Icon name="paste" /></button>
    <span class="divider"></span>
    <div class="seg" role="group" aria-label={t("Öffnen als")}>
      {#each MODES as mode (mode.id)}
        <button class:active={settings.openMode === mode.id} title={t(mode.label)} onclick={() => setMode(mode.id)}>
          <Icon name={mode.icon} size={16} />
          <span>{t(mode.label)}</span>
        </button>
      {/each}
    </div>
    {#if settings.openMode === "tiles" && !view.viewing}
      <label class="size-slider" title={t("Kachelgrösse")}>
        <Icon name="tiles" size={14} />
        <input type="range" min="96" max="360" step="8" bind:value={settings.tileSize} />
      </label>
    {/if}

    <span class="spacer"></span>

    {#if showViewer}
      {#if view.viewing}
        <button class="icon-btn" title="{t('Zurück zur Übersicht')} (Esc)" onclick={() => (view.viewing = false)}><Icon name="back" /></button>
      {/if}
      {#if pos}
        <button class="icon-btn" title="{t('Vorherige Datei')} (←)" onclick={() => step(-1)}><Icon name="prev" /></button>
        <button class="icon-btn" title="{t('Nächste Datei')} (→)" onclick={() => step(1)}><Icon name="next" /></button>
        <span class="divider"></span>
      {/if}
      {#if showImage}
        <button class="icon-btn" title="{t('Verkleinern')} (-)" onclick={() => viewer?.zoomBy(0.8)}><Icon name="zoomOut" /></button>
        <span class="zoom-label">{Math.round(scale * 100)} %</span>
        <button class="icon-btn" title="{t('Vergrössern')} (+)" onclick={() => viewer?.zoomBy(1.25)}><Icon name="zoomIn" /></button>
        <button class="icon-btn" title="{t('Einpassen')} (0)" onclick={() => viewer?.fit()}><Icon name="fit" /></button>
        <button class="icon-btn" title="{t('Originalgrösse')} (1)" onclick={() => viewer?.actual()}><Icon name="actual" /></button>
        <button class="icon-btn" title="{t('Nach links drehen')} (Shift+R)" onclick={() => viewer?.rotate(-90)}><Icon name="rotateLeft" /></button>
        <button class="icon-btn" title="{t('Nach rechts drehen')} (R)" onclick={() => viewer?.rotate(90)}><Icon name="rotateRight" /></button>
        <button
          class="icon-btn"
          class:active={settings.rulers}
          title="{t('Lineale')} (Ctrl+R) · {t('Hilfslinien')} (Ctrl+H)"
          onclick={() => (settings.rulers = !settings.rulers)}
        >
          <Icon name="ruler" />
        </button>
      {/if}
      <button class="icon-btn" title="{t('Vollbild')} (F11)" onclick={toggleFullscreen}><Icon name="fullscreen" /></button>
      <span class="divider"></span>
    {/if}

    <button
      class="icon-btn"
      title={dark ? t("Helles Design") : t("Dunkles Design")}
      onclick={() => (settings.theme = dark ? "light" : "dark")}
    >
      <Icon name={dark ? "sun" : "moon"} />
    </button>
    <button class="icon-btn" title="{t('Einstellungen')} (Ctrl+,)" onclick={() => (showSettings = true)}><Icon name="settings" /></button>
  </header>

  {#if settings.openMode === "tabs" && view.items.length}
    <TabBar onmenu={openMenu} />
  {/if}

  <main class="stage">
    {#if !item}
      <div class="empty">
        <Icon name="image" size={56} />
        <div class="empty-title">{t("Dateien hierher ziehen")}</div>
        <div class="row">
          <button class="primary" onclick={openFilesDialog}>{t("Dateien öffnen")}</button>
          <button onclick={openFolderDialog}>{t("Ordner öffnen")}</button>
          <button title="Ctrl+V" onclick={pasteClipboard}>{t("Aus Zwischenablage einfügen")}</button>
        </div>
        <div class="small">PNG · JPG · BMP · GIF · TIF · MP3 · MP4 · MOV</div>
      </div>
    {:else if showViewer}
      {#key item.id}
        {#if item.kind === "image"}
          <Viewer bind:this={viewer} bind:scale {item} oncontextmenu={(e) => openMenu(e, view.active)} />
        {:else}
          <MediaPlayer bind:this={player} {item} oncontextmenu={(e) => openMenu(e, view.active)} />
        {/if}
      {/key}
    {:else if settings.openMode === "tiles"}
      <TilesView bind:this={tilesView} onmenu={openMenu} />
    {:else}
      <ListView onmenu={openMenu} />
    {/if}
    {#if view.dragging}
      <div class="drop-overlay">{t("Loslassen zum Öffnen")}</div>
    {/if}
  </main>

  <footer class="statusbar">
    {#if item}
      <span class="status-name" title={item.path}>{item.name}</span>
      {#if item.width}<span>{item.width} × {item.height} px</span>{/if}
      {#if item.duration}<span>{formatDuration(item.duration)}</span>{/if}
      {#if item.loaded}<span>{formatSize(item.size)}</span>{/if}
      {#if item.loaded}<span>{formatLabel(item)}</span>{/if}
    {:else}
      <span class="muted">{t("Keine Dateien geöffnet")}</span>
    {/if}
    <span class="spacer"></span>
    {#if view.message}<span class="status-msg">{view.message}</span>{/if}
    {#if pos}<span>{pos[0]} / {pos[1]}</span>{/if}
  </footer>
</div>

{#if menu}
  <ContextMenu {...menu} onclose={() => (menu = null)} />
{/if}

{#if showSettings}
  <SettingsDialog onclose={() => (showSettings = false)} />
{/if}
