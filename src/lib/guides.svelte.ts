/** Guide positions in pixels of the displayed (rotated) image: `x` holds vertical guides, `y` horizontal ones. */
export interface Guides {
  x: number[];
  y: number[];
}

const sets: Record<number, Guides> = {};

export function guidesFor(id: number): Guides {
  if (!sets[id]) {
    const guides = $state<Guides>({ x: [], y: [] });
    sets[id] = guides;
  }
  return sets[id];
}

export function hasGuides(id: number): boolean {
  const g = sets[id];
  return !!g && g.x.length + g.y.length > 0;
}

export function clearGuides(id: number) {
  const g = sets[id];
  if (!g) return;
  g.x.length = 0;
  g.y.length = 0;
}
