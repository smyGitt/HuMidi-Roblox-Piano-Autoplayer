export const EDGE_MARGIN = 8;

export interface PopupRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface PopupFit {
  dx: number;
  above: boolean;
}

export const NO_SHIFT: PopupFit = { dx: 0, above: false };

export function fitToViewport(rect: PopupRect, viewport: { width: number; height: number }): PopupFit {
  let dx = 0;
  const rightLimit = viewport.width - EDGE_MARGIN;
  if (rect.right > rightLimit) dx = rightLimit - rect.right;
  if (rect.left + dx < EDGE_MARGIN) dx = EDGE_MARGIN - rect.left;
  const above = rect.bottom > viewport.height - EDGE_MARGIN;
  return dx === 0 && !above ? NO_SHIFT : { dx, above };
}
