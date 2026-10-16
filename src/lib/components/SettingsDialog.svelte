<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import Icon, { type IconName } from "./Icon.svelte";
  import { t } from "$lib/i18n";
  import { settings, type OpenMode } from "$lib/settings.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const MODES: { id: OpenMode; icon: IconName; label: string; hint: string }[] = [
    { id: "window", icon: "window", label: "Neue Instanz", hint: "Jede Datei öffnet in einem eigenen Programmfenster." },
    { id: "tabs", icon: "tabs", label: "Tabs", hint: "Alle Dateien in einem Fenster, ein Tab pro Datei." },
    { id: "tiles", icon: "tiles", label: "Kacheln", hint: "Alle Dateien als Vorschau-Kacheln, Doppelklick öffnet die Datei." },
    { id: "list", icon: "list", label: "Liste", hint: "Alle Dateien als Liste mit Details, Doppelklick öffnet die Datei." },
  ];

  let version = $state("");
  getVersion()
    .then((v) => (version = v))
    .catch(() => {});
</script>

<div
  class="modal-backdrop"
  role="presentation"
  onpointerdown={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="modal" role="dialog" aria-label={t("Einstellungen")}>
    <div class="modal-head">
      <strong>{t("Einstellungen")}</strong>
      <span class="spacer"></span>
      <button class="icon-btn" title="{t('Schliessen')} (Esc)" onclick={onclose}><Icon name="close" /></button>
    </div>

    <fieldset>
      <legend>{t("Öffnen als")}</legend>
      {#each MODES as mode (mode.id)}
        <label class="mode-option" class:active={settings.openMode === mode.id}>
          <input type="radio" name="open-mode" value={mode.id} bind:group={settings.openMode} />
          <Icon name={mode.icon} />
          <span>
            <strong>{t(mode.label)}</strong>
            <small>{t(mode.hint)}</small>
          </span>
        </label>
      {/each}
    </fieldset>

    <fieldset>
      <legend>{t("Darstellung")}</legend>
      <div class="grid2">
        <label>
          {t("Design")}
          <select bind:value={settings.theme}>
            <option value="light">{t("Hell")}</option>
            <option value="dark">{t("Dunkel")}</option>
            <option value="system">{t("Wie Windows")}</option>
          </select>
        </label>
        <label>
          {t("Sprache")}
          <select bind:value={settings.lang}>
            <option value="de">Deutsch</option>
            <option value="en">English</option>
          </select>
        </label>
      </div>
      <label class="check"><input type="checkbox" bind:checked={settings.checker} /> {t("Transparenz als Schachbrett anzeigen")}</label>
      <label class="check"><input type="checkbox" bind:checked={settings.upscale} /> {t("Kleine Bilder auf Fenstergrösse vergrössern")}</label>
      <label class="check"><input type="checkbox" bind:checked={settings.splash} /> {t("Startlogo beim Programmstart anzeigen")}</label>
    </fieldset>

    <fieldset>
      <legend>{t("Wiedergabe")}</legend>
      <div class="row">
        <label class="check"><input type="checkbox" bind:checked={settings.slideshow} /> {t("Diashow")}</label>
        <span class="spacer"></span>
        <label class="row muted nowrap">
          <input class="seconds" type="number" min="1" max="3600" step="1" bind:value={settings.slideSeconds} />
          {t("Sekunden pro Bild")}
        </label>
      </div>
      <label class="check"><input type="checkbox" bind:checked={settings.autoNext} /> {t("Nach dem Ende von Audio und Video zur nächsten Datei springen")}</label>
    </fieldset>

    <p class="muted small">Dynooh MiniPixels {version} · PNG · JPG · BMP · GIF · TIF · MP3 · MP4 · MOV</p>
  </div>
</div>
