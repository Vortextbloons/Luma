export interface StickVector {
  x: number;
  y: number;
}

export function stickVector(dx: number, dy: number, range: number): StickVector {
  const span = Math.max(32, range * 0.22);
  let x = dx / span;
  let y = dy / span;
  const length = Math.hypot(x, y);
  if (length > 1) {
    x /= length;
    y /= length;
  }
  return { x, y };
}

export function isTap(travel: number, elapsed: number): boolean {
  return travel < 6 && elapsed < 350;
}
