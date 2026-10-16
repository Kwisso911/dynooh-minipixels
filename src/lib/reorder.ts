import { moveItem } from "./files.svelte";

/** Lets the user drag the children marked with data-index to a new place. "x" compares
 *  against the horizontal middle of the target (tabs, tiles), "y" against the vertical one (list).
 *  HTML drag and drop is not available because the window handles file drops itself. */
export function reorder(container: HTMLElement, axis: "x" | "y") {
  const THRESHOLD = 6;
  const EDGE = 36;
  let from = -1;
  let startX = 0;
  let startY = 0;
  let dragging = false;
  let source: HTMLElement | null = null;
  let marked: HTMLElement | null = null;
  let to = -1;

  function mark(el: HTMLElement | null, after = false) {
    marked?.classList.remove("drop-before", "drop-after");
    marked = el;
    el?.classList.add(after ? "drop-after" : "drop-before");
  }

  function scroller(): HTMLElement {
    for (let el: HTMLElement | null = container; el; el = el.parentElement) {
      if (el.scrollHeight > el.clientHeight || el.scrollWidth > el.clientWidth) return el;
    }
    return container;
  }

  function autoScroll(e: PointerEvent) {
    const s = scroller();
    const r = s.getBoundingClientRect();
    if (s.scrollWidth > s.clientWidth) {
      if (e.clientX < r.left + EDGE) s.scrollLeft -= 16;
      else if (e.clientX > r.right - EDGE) s.scrollLeft += 16;
    }
    if (s.scrollHeight > s.clientHeight) {
      if (e.clientY < r.top + EDGE) s.scrollTop -= 16;
      else if (e.clientY > r.bottom - EDGE) s.scrollTop += 16;
    }
  }

  function onMove(e: PointerEvent) {
    if (!dragging) {
      if (Math.hypot(e.clientX - startX, e.clientY - startY) < THRESHOLD) return;
      dragging = true;
      source?.classList.add("drag-source");
      document.body.classList.add("reordering");
    }
    autoScroll(e);
    const target = (document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null)?.closest<HTMLElement>("[data-index]");
    if (!target || !container.contains(target)) return;
    const r = target.getBoundingClientRect();
    const after = axis === "x" ? e.clientX > r.left + r.width / 2 : e.clientY > r.top + r.height / 2;
    to = Number(target.dataset.index) + (after ? 1 : 0);
    mark(target, after);
  }

  function onUp() {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onUp);
    source?.classList.remove("drag-source");
    document.body.classList.remove("reordering");
    mark(null);
    if (dragging) {
      // The click that ends the drag must not select or open anything.
      window.addEventListener("click", swallow, true);
      setTimeout(() => window.removeEventListener("click", swallow, true), 0);
      if (to >= 0) moveItem(from, to);
    }
    dragging = false;
    source = null;
  }

  function swallow(e: Event) {
    e.stopPropagation();
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const el = e.target as HTMLElement;
    if (el.closest(".play-badge, .tab-close, input")) return;
    source = el.closest<HTMLElement>("[data-index]");
    if (!source || !container.contains(source)) return;
    from = Number(source.dataset.index);
    to = -1;
    startX = e.clientX;
    startY = e.clientY;
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  container.addEventListener("pointerdown", onDown);
  return {
    destroy() {
      container.removeEventListener("pointerdown", onDown);
      onUp();
    },
  };
}
