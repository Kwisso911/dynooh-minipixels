<script lang="ts">
  import { t } from "$lib/i18n";
  import { settings, isDark } from "$lib/settings.svelte";
  import { fileUrl, type Item } from "$lib/files.svelte";
  import { guidesFor } from "$lib/guides.svelte";

  let {
    item,
    scale = $bindable(1),
    oncontextmenu,
  }: { item: Item; scale?: number; oncontextmenu?: (e: MouseEvent) => void } = $props();

  const MIN = 0.02;
  const MAX = 64;
  const WHEEL_STEP = 1.15;
  const RULER = 20;
  /** Labelled ruler steps in image pixels; the first one at least `LABEL_GAP` screen pixels wide is used. */
  const STEPS = [1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1000, 2500, 5000, 10000, 25000, 50000, 100000];
  const LABEL_GAP = 64;
  /** The pixel grid appears above this zoom factor. */
  const GRID_ZOOM = 4;

  let host: HTMLDivElement;
  let gridCanvas = $state<HTMLCanvasElement>();
  let hRuler = $state<HTMLCanvasElement>();
  let vRuler = $state<HTMLCanvasElement>();
  let cw = $state(0);
  let ch = $state(0);
  let nw = $state(0);
  let nh = $state(0);
  let x = $state(0);
  let y = $state(0);
  let rotation = $state(0);
  let fitted = $state(true);
  let status = $state<"loading" | "ok" | "error">("loading");
  let grabbing = $state(false);
  let drag: { sx: number; sy: number; ox: number; oy: number } | null = null;
  let pointer = $state<{ x: number; y: number } | null>(null);
  let guideDrag = $state<{ axis: "x" | "y"; index: number } | null>(null);
  let selected = $state<{ axis: "x" | "y"; index: number } | null>(null);
  const guides = $derived(guidesFor(item.id));

  const turned = $derived(rotation % 180 !== 0);
  const fitScale = $derived.by(() => {
    const rw = turned ? nh : nw;
    const rh = turned ? nw : nh;
    if (!rw || !rh || !cw || !ch) return 1;
    return Math.min(cw / rw, ch / rh, settings.upscale ? Infinity : 1);
  });
  /** Top-left corner of the displayed image, relative to the viewport. */
  const left0 = $derived(cw / 2 + x - ((turned ? nh : nw) * scale) / 2);
  const top0 = $derived(ch / 2 + y - ((turned ? nw : nh) * scale) / 2);
  const showGrid = $derived(status === "ok" && scale > GRID_ZOOM);

  $effect(() => {
    if (!showGrid || !gridCanvas) return;
    drawPixelGrid(gridCanvas, cw, ch, left0, top0, turned ? nh : nw, turned ? nw : nh, scale);
  });

  $effect(() => {
    if (fitted) {
      scale = fitScale;
      x = 0;
      y = 0;
    }
  });

  $effect(() => {
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const [px, py] = fromCenter(e);
      zoomBy(e.deltaY < 0 ? WHEEL_STEP : 1 / WHEEL_STEP, px, py);
    };
    host.addEventListener("wheel", onWheel, { passive: false });
    return () => host.removeEventListener("wheel", onWheel);
  });

  $effect(() => {
    if (!settings.rulers || !hRuler || !vRuler) return;
    isDark();
    const h = hRuler;
    const v = vRuler;
    const args = { w: cw, h: ch, left: left0, top: top0, s: scale, px: pointer?.x, py: pointer?.y };
    // Next frame, so the theme colours are already applied to the CSS variables.
    const frame = requestAnimationFrame(() => {
      drawRuler(h, args.w, args.left, args.s, true, args.px);
      drawRuler(v, args.h, args.top, args.s, false, args.py);
    });
    return () => cancelAnimationFrame(frame);
  });

  function drawRuler(canvas: HTMLCanvasElement, length: number, origin: number, s: number, horizontal: boolean, mark?: number) {
    const dpr = window.devicePixelRatio || 1;
    const w = horizontal ? length : RULER;
    const h = horizontal ? RULER : length;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    const c = canvas.getContext("2d");
    if (!c) return;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    const css = getComputedStyle(canvas);
    const color = (name: string) => css.getPropertyValue(name).trim();

    c.fillStyle = color("--bg2");
    c.fillRect(0, 0, w, h);
    c.strokeStyle = color("--line");
    c.beginPath();
    if (horizontal) {
      c.moveTo(0, h - 0.5);
      c.lineTo(w, h - 0.5);
    } else {
      c.moveTo(w - 0.5, 0);
      c.lineTo(w - 0.5, h);
    }
    c.stroke();

    const step = STEPS.find((v) => v * s >= LABEL_GAP) ?? STEPS[STEPS.length - 1];
    const sub = [10, 5, 2].find((d) => step % d === 0 && (step / d) * s >= 5) ?? 1;
    const minor = step / sub;
    const from = Math.floor(-origin / s / minor);
    const to = Math.ceil((length - origin) / s / minor);

    c.strokeStyle = color("--muted");
    c.fillStyle = color("--muted");
    c.font = "10px 'Segoe UI', sans-serif";
    c.beginPath();
    for (let i = from; i <= to; i++) {
      const value = i * minor;
      const p = Math.round(origin + value * s) + 0.5;
      const rest = ((i % sub) + sub) % sub;
      const tick = rest === 0 ? RULER : sub === 10 && rest === 5 ? 8 : 4;
      if (horizontal) {
        c.moveTo(p, h);
        c.lineTo(p, h - tick);
      } else {
        c.moveTo(w, p);
        c.lineTo(w - tick, p);
      }
      if (rest !== 0) continue;
      if (horizontal) {
        c.fillText(String(value), p + 3, 10);
      } else {
        c.save();
        c.translate(10, p + 3);
        c.rotate(-Math.PI / 2);
        c.textAlign = "right";
        c.fillText(String(value), 0, 0);
        c.restore();
      }
    }
    c.stroke();

    if (mark !== undefined) {
      const p = Math.round(mark) + 0.5;
      c.strokeStyle = color("--accent");
      c.beginPath();
      if (horizontal) {
        c.moveTo(p, 0);
        c.lineTo(p, h);
      } else {
        c.moveTo(0, p);
        c.lineTo(w, p);
      }
      c.stroke();
    }
  }

  /** Fine lines between the image pixels, only for the visible part of the image. */
  function drawPixelGrid(canvas: HTMLCanvasElement, w: number, h: number, left: number, top: number, iw: number, ih: number, s: number) {
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    const c = canvas.getContext("2d");
    if (!c) return;
    const x0 = Math.max(0, Math.floor(-left / s));
    const x1 = Math.min(iw, Math.ceil((w - left) / s));
    const y0 = Math.max(0, Math.floor(-top / s));
    const y1 = Math.min(ih, Math.ceil((h - top) / s));
    if (x0 > x1 || y0 > y1) return;
    const device = (v: number) => Math.round(v * dpr) + 0.5;
    const ys = device(top + y0 * s);
    const ye = device(top + y1 * s);
    const xs = device(left + x0 * s);
    const xe = device(left + x1 * s);
    c.strokeStyle = `rgba(0, 0, 0, ${Math.min(0.14, 0.04 + (s - GRID_ZOOM) * 0.01)})`;
    c.lineWidth = 1;
    c.beginPath();
    for (let i = x0; i <= x1; i++) {
      const p = device(left + i * s);
      c.moveTo(p, ys);
      c.lineTo(p, ye);
    }
    for (let i = y0; i <= y1; i++) {
      const p = device(top + i * s);
      c.moveTo(xs, p);
      c.lineTo(xe, p);
    }
    c.stroke();
  }

  function fromCenter(e: MouseEvent): [number, number] {
    const [hx, hy] = inHost(e);
    return [hx - cw / 2, hy - ch / 2];
  }

  function inHost(e: MouseEvent): [number, number] {
    const r = host.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  /** Zooms so the point (px, py), relative to the viewport centre, stays in place. */
  function zoomTo(s: number, px = 0, py = 0) {
    const next = Math.min(MAX, Math.max(MIN, s));
    const f = next / scale;
    x = px - (px - x) * f;
    y = py - (py - y) * f;
    scale = next;
    fitted = false;
  }

  export function zoomBy(f: number, px = 0, py = 0) {
    zoomTo(scale * f, px, py);
  }

  export function fit() {
    fitted = true;
  }

  export function actual() {
    zoomTo(1);
  }

  export function rotate(deg: number) {
    rotation = (rotation + deg + 360) % 360;
    fitted = true;
  }

  function onLoad(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    nw = img.naturalWidth;
    nh = img.naturalHeight;
    status = "ok";
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0) return;
    selected = null;
    host.setPointerCapture(e.pointerId);
    drag = { sx: e.clientX, sy: e.clientY, ox: x, oy: y };
    grabbing = true;
  }

  function onMove(e: PointerEvent) {
    const [hx, hy] = inHost(e);
    pointer = { x: hx, y: hy };
    if (!drag) return;
    const dx = e.clientX - drag.sx;
    const dy = e.clientY - drag.sy;
    if (fitted && Math.abs(dx) + Math.abs(dy) < 3) return;
    fitted = false;
    x = drag.ox + dx;
    y = drag.oy + dy;
  }

  function onUp() {
    drag = null;
    grabbing = false;
  }

  function onDblClick(e: MouseEvent) {
    if (!fitted) return fit();
    const [px, py] = fromCenter(e);
    if (scale < 1) zoomTo(1, px, py);
    else zoomBy(2, px, py);
  }

  function guideValue(axis: "x" | "y", e: PointerEvent): number {
    const [hx, hy] = inHost(e);
    return Math.round(axis === "x" ? (hx - left0) / scale : (hy - top0) / scale);
  }

  /** Starts moving guide `index`, or a new guide pulled out of a ruler when `index` is -1. */
  function startGuide(e: PointerEvent, axis: "x" | "y", index = -1) {
    if (e.button !== 0 || status !== "ok") return;
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    if (index < 0) {
      settings.guides = true;
      index = guides[axis].push(guideValue(axis, e)) - 1;
    }
    guideDrag = { axis, index };
    selected = { axis, index };
  }

  /** Removes the selected guide; false if none is selected. */
  export function deleteSelectedGuide(): boolean {
    if (!selected || !settings.guides || selected.index >= guides[selected.axis].length) return false;
    guides[selected.axis].splice(selected.index, 1);
    selected = null;
    return true;
  }

  function moveGuide(e: PointerEvent) {
    if (!guideDrag) return;
    const [hx, hy] = inHost(e);
    pointer = { x: hx, y: hy };
    guides[guideDrag.axis][guideDrag.index] = guideValue(guideDrag.axis, e);
  }

  /** A guide released outside the image area is removed. */
  function endGuide(e: PointerEvent) {
    if (!guideDrag) return;
    const [hx, hy] = inHost(e);
    if (hx < 0 || hy < 0 || hx > cw || hy > ch) {
      guides[guideDrag.axis].splice(guideDrag.index, 1);
      selected = null;
    }
    guideDrag = null;
  }
</script>

<div class="viewer-wrap" class:rulers={settings.rulers}>
  {#if settings.rulers}
    <div class="ruler-corner"></div>
    <canvas
      class="ruler ruler-h"
      bind:this={hRuler}
      title={t("Ziehen für horizontale Hilfslinie")}
      onpointerdown={(e) => startGuide(e, "y")}
      onpointermove={moveGuide}
      onpointerup={endGuide}
      onpointercancel={endGuide}
    ></canvas>
    <canvas
      class="ruler ruler-v"
      bind:this={vRuler}
      title={t("Ziehen für vertikale Hilfslinie")}
      onpointerdown={(e) => startGuide(e, "x")}
      onpointermove={moveGuide}
      onpointerup={endGuide}
      onpointercancel={endGuide}
    ></canvas>
  {/if}
  <div
    class="viewer"
    class:grabbing
    bind:this={host}
    bind:clientWidth={cw}
    bind:clientHeight={ch}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={onUp}
    onpointercancel={onUp}
    onpointerleave={() => (pointer = null)}
    ondblclick={onDblClick}
    {oncontextmenu}
    role="img"
    aria-label={item.name}
  >
    {#if item.loaded}
      <img
        src={fileUrl(item)}
        alt={item.name}
        draggable="false"
        class:checker={settings.checker}
        class:pixelated={scale >= 2}
        class:hidden={status !== "ok"}
        style:width="{nw * scale}px"
        style:height="{nh * scale}px"
        style:transform="translate(-50%, -50%) translate({x}px, {y}px) rotate({rotation}deg)"
        onload={onLoad}
        onerror={() => (status = "error")}
      />
    {/if}
    {#if showGrid}
      <canvas class="pixel-grid" bind:this={gridCanvas}></canvas>
    {/if}
    {#if status === "ok" && settings.guides}
      {#each guides.x as gx, i}
        <div
          class="guide guide-v"
          class:active={selected?.axis === "x" && selected.index === i}
          style:left="{Math.round(left0 + gx * scale)}px"
          onpointerdown={(e) => startGuide(e, "x", i)}
          onpointermove={moveGuide}
          onpointerup={endGuide}
          onpointercancel={endGuide}
          role="separator"
          aria-orientation="vertical"
        ></div>
      {/each}
      {#each guides.y as gy, i}
        <div
          class="guide guide-h"
          class:active={selected?.axis === "y" && selected.index === i}
          style:top="{Math.round(top0 + gy * scale)}px"
          onpointerdown={(e) => startGuide(e, "y", i)}
          onpointermove={moveGuide}
          onpointerup={endGuide}
          onpointercancel={endGuide}
          role="separator"
          aria-orientation="horizontal"
        ></div>
      {/each}
    {/if}
    {#if guideDrag && pointer}
      <div class="guide-tip" style:left="{pointer.x + 12}px" style:top="{pointer.y + 12}px">
        {guideDrag.axis === "x" ? "X" : "Y"}: {guides[guideDrag.axis][guideDrag.index]} px
      </div>
    {/if}
    {#if status === "loading"}
      <div class="viewer-msg">{t("Lade …")}</div>
    {:else if status === "error"}
      <div class="viewer-msg error">
        {t("Bild kann nicht angezeigt werden.")}
        {#if item.error}<small>{item.error}</small>{/if}
      </div>
    {/if}
  </div>
</div>
