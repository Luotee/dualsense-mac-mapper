// SVG-based DualSense renderer with 25 clickable hit zones.
//
// The SVG uses a 240×130 viewBox. Coordinates place:
//   triggers (L2/R2) at the very top,
//   shoulders (L1/R1) just below them on the body's top edge,
//   D-pad at top-left of body, face buttons diamond at top-right,
//   touchpad centred above the body,
//   L-stick and R-stick wells in the lower half.
//   Virtual stick directions are wedges around each stick well.
//
// Public API consumed by mappings.js (Task 20):
//   render(parent, bindings)       — mount the SVG inside `parent`, sized by CSS
//   flashPress(svg, id)            — yellow live-press highlight + ring animation
//   clearPress(svg, id)            — remove the live-press visuals
//   selectButton(svg, id)          — show the bind-popup-selection ring
//   clearSelection(svg)            — remove the selection ring
//
// `bindings` is an object keyed by stringified id:
//   { "0": { kind: "key"|"macro"|"unbound", value? }, ... }
//
// Click events bubble up from the hit zone. Each hit zone has `data-id="<n>"`;
// mappings.js listens for clicks on the SVG and reads
//   target.closest('[data-id]')

const VIEWBOX = '0 0 240 130';

// Body silhouette: stylised pill with grip lobes at the bottom.
const BODY_PATH =
  'M 50 30 Q 38 30 36 50 Q 32 80 62 92 Q 75 102 95 102 ' +
  'L 145 102 Q 165 102 178 92 Q 208 80 204 50 Q 202 30 190 30 L 50 30 Z';

// Touchpad shape: rounded rect centred at (120, 44).
const TOUCHPAD = { x: 101, y: 36, w: 38, h: 16, rx: 5 };

// Stick well centres
const L_STICK = { cx: 84,  cy: 82, r: 9 };
const R_STICK = { cx: 156, cy: 82, r: 9 };

// ─── Element descriptors ──────────────────────────────────────────────────────
//
// Returns array of element descriptors. Each descriptor with a numeric `id`
// produces ONE hit zone (<g data-id="N">). Sprite-only descriptors (no id)
// render decoration graphics only.

function elements() {
  return [
    // Triggers (top) — same dimensions as the shoulders below, and the
    // vertical gap between trigger / shoulder / body is uniformly 2 px
    // (L2 ends at y=20, L1 starts y=22; L1 ends y=28, body top y=30).
    el(23, 'trigger',    { rx: 48,  ry: 14, w: 22, h: 6 },  'L2'),
    el(24, 'trigger',    { rx: 170, ry: 14, w: 22, h: 6 },  'R2'),

    // Shoulders (just below triggers, on body's top edge)
    el(9,  'shoulder',   { rx: 48,  ry: 22, w: 22, h: 6 },  'L1'),
    el(10, 'shoulder',   { rx: 170, ry: 22, w: 22, h: 6 },  'R1'),

    // Meta buttons (Share / Options)
    el(4,  'meta_rect',  { rx: 82,  ry: 38, w: 7,  h: 3 },  'Share'),
    el(6,  'meta_rect',  { rx: 151, ry: 38, w: 7,  h: 3 },  'Options'),

    // PS logo button (centre, below touchpad)
    el(5,  'circle',     { cx: 120, cy: 62, r: 3 },          'PS'),

    // D-pad: four label-shaped pentagons (flat base outward, apex
    // pointing toward the centre). Replaces the v1.1.0 cross sprite +
    // outward-arrow wedges — the user wanted the d-pad to read like
    // four independent face-button-style targets, with no underlying
    // cross silhouette competing for attention.
    el(11, 'dpad_wedge', { cx: 59, cy: 57, dir: 'up'    },   'D-up'),
    el(12, 'dpad_wedge', { cx: 59, cy: 57, dir: 'down'  },   'D-down'),
    el(13, 'dpad_wedge', { cx: 59, cy: 57, dir: 'left'  },   'D-left'),
    el(14, 'dpad_wedge', { cx: 59, cy: 57, dir: 'right' },   'D-right'),

    // Face buttons (right diamond)
    el(3,  'face',       { cx: 184, cy: 50, r: 4 },           'Triangle'),  // top
    el(1,  'face',       { cx: 192, cy: 58, r: 4 },           'Circle'),    // right
    el(0,  'face',       { cx: 184, cy: 66, r: 4 },           'Cross'),     // bottom
    el(2,  'face',       { cx: 176, cy: 58, r: 4 },           'Square'),    // left

    // L stick well sprite (no hit zone)
    { sprite: 'stick_well', side: 'L', c: L_STICK },

    // L stick virtual wedges (ids 15–18) — rendered BEFORE L3 so L3 stays on top
    el(15, 'stick_wedge', { ...L_STICK, dir: 'up'    }, 'L-up'),
    el(16, 'stick_wedge', { ...L_STICK, dir: 'down'  }, 'L-down'),
    el(17, 'stick_wedge', { ...L_STICK, dir: 'left'  }, 'L-left'),
    el(18, 'stick_wedge', { ...L_STICK, dir: 'right' }, 'L-right'),

    // L3 (depressed L stick) — rendered last in L cluster so it sits on top of wedges
    el(7,  'stick_press', { cx: L_STICK.cx, cy: L_STICK.cy, r: 5 }, 'L3'),

    // R stick well sprite
    { sprite: 'stick_well', side: 'R', c: R_STICK },

    // R stick virtual wedges (ids 19–22)
    el(19, 'stick_wedge', { ...R_STICK, dir: 'up'    }, 'R-up'),
    el(20, 'stick_wedge', { ...R_STICK, dir: 'down'  }, 'R-down'),
    el(21, 'stick_wedge', { ...R_STICK, dir: 'left'  }, 'R-left'),
    el(22, 'stick_wedge', { ...R_STICK, dir: 'right' }, 'R-right'),

    // R3 (depressed R stick) — on top of R wedges
    el(8,  'stick_press', { cx: R_STICK.cx, cy: R_STICK.cy, r: 5 }, 'R3'),
  ];
}

function el(id, kind, geo, label) {
  return { id, kind, geo, label };
}

// ─── Binding → CSS class ──────────────────────────────────────────────────────

function kindClass(kind) {
  switch (kind) {
    case 'key':     return 'binding-key';
    case 'macro':   return 'binding-macro';
    case 'unbound': return 'binding-unbound';
    default:        return 'binding-unbound';
  }
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Mount the controller SVG inside `parent`. Returns the <svg> element.
 *
 * @param {Element} parent   - Container element (its innerHTML will be cleared).
 * @param {Object}  bindings - Map of id (string) → { kind, value? }.
 * @returns {SVGElement}
 */
export function render(parent, bindings) {
  const ns = 'http://www.w3.org/2000/svg';
  parent.innerHTML = '';

  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', VIEWBOX);
  svg.setAttribute('xmlns', ns);
  svg.classList.add('controller');

  // Body silhouette
  const body = document.createElementNS(ns, 'path');
  body.setAttribute('d', BODY_PATH);
  body.classList.add('body');
  svg.appendChild(body);

  // Touchpad decorative shape
  const tp = mkRect(ns, TOUCHPAD.x, TOUCHPAD.y, TOUCHPAD.w, TOUCHPAD.h, 'touchpad');
  tp.setAttribute('rx', String(TOUCHPAD.rx));
  svg.appendChild(tp);

  // Render each element descriptor
  for (const e of elements()) {
    // ── Sprite-only (decoration, no hit zone) ──
    if (e.sprite === 'dpad_cross') {
      // Horizontal arm
      const horiz = mkRect(ns, 52, 54, 14, 5, 'dpad-arm');
      horiz.setAttribute('rx', '1');
      svg.appendChild(horiz);
      // Vertical arm
      const vert  = mkRect(ns, 57, 49, 4, 14, 'dpad-arm');
      vert.setAttribute('rx', '1');
      svg.appendChild(vert);
      continue;
    }

    if (e.sprite === 'stick_well') {
      const ring = mkCircle(ns, e.c.cx, e.c.cy, e.c.r, 'stick-ring');
      svg.appendChild(ring);
      continue;
    }

    // ── Hit zone ──
    const cls = kindClass((bindings[String(e.id)] || {}).kind);

    let shape;
    switch (e.kind) {
      case 'trigger':
      case 'shoulder':
      case 'meta_rect': {
        shape = mkRect(ns, e.geo.rx, e.geo.ry, e.geo.w, e.geo.h, `hit ${cls}`);
        shape.setAttribute('rx', '2');
        break;
      }
      case 'circle':
      case 'face':
      case 'stick_press': {
        shape = mkCircle(ns, e.geo.cx, e.geo.cy, e.geo.r, `hit ${cls}`);
        break;
      }
      case 'dpad_wedge': {
        // Pentagon-shaped arrow pointing outward from the d-pad centre.
        // Replaces the v1.0.x triangle wedge so a bound direction tints
        // visibly with its own arrow outline (the press-ring clone then
        // animates the same arrow shape on physical press).
        shape = mkArrow(ns, e.geo.cx, e.geo.cy, e.geo.dir, `hit wedge ${cls}`);
        break;
      }
      case 'stick_wedge': {
        // Donut quarter arc around the stick well. Each quarter is its
        // own hit zone; the inner L3/R3 circle (stick_press) covers the
        // central press. Same `wedge` class so unbound is transparent
        // and bound tints with the binding colour.
        shape = mkQuarter(ns, e.geo.cx, e.geo.cy, e.geo.dir, `hit wedge ${cls}`);
        break;
      }
      default:
        continue;
    }

    shape.dataset.id    = String(e.id);
    shape.dataset.label = e.label;
    svg.appendChild(shape);
  }

  parent.appendChild(svg);
  return svg;
}

// ─── Live-press overlay ───────────────────────────────────────────────────────

// Keyed by stringified id → overlay <element>
const _flashes = new Map();

/**
 * Show a yellow press-ring animation over the hit zone for `id`.
 * Safe to call repeatedly — no-ops if already flashing.
 *
 * @param {SVGElement} svg
 * @param {number|string} id
 */
export function flashPress(svg, id) {
  const key = String(id);
  if (_flashes.has(key)) return;  // already active
  const target = svg.querySelector(`[data-id="${key}"]`);
  if (!target) return;

  const ring = _cloneAsOverlay(target, 'press-ring');
  svg.appendChild(ring);
  _flashes.set(key, ring);

  // Auto-remove after the animation finishes (250 ms + 20 ms buffer).
  setTimeout(() => {
    if (ring.parentNode) ring.remove();
    if (_flashes.get(key) === ring) _flashes.delete(key);
  }, 270);
}

/**
 * Remove the live-press ring for `id`, if present.
 *
 * @param {SVGElement} svg
 * @param {number|string} id
 */
export function clearPress(svg, id) {
  const key = String(id);
  const ring = _flashes.get(key);
  if (ring) {
    if (ring.parentNode) ring.remove();
    _flashes.delete(key);
  }
}

// ─── Selection ring ───────────────────────────────────────────────────────────

let _selectionRing = null;

/**
 * Highlight `id` as the currently selected button (for the remap popup).
 * Replaces any previous selection.
 *
 * @param {SVGElement} svg
 * @param {number|string} id
 */
export function selectButton(svg, id) {
  clearSelection(svg);
  const target = svg.querySelector(`[data-id="${String(id)}"]`);
  if (!target) return;
  const ring = _cloneAsOverlay(target, 'selection-ring');
  svg.appendChild(ring);
  _selectionRing = ring;
}

/**
 * Remove the current selection ring, if any.
 *
 * @param {SVGElement} svg
 */
export function clearSelection(_svg) {
  if (_selectionRing) {
    if (_selectionRing.parentNode) _selectionRing.remove();
    _selectionRing = null;
  }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/**
 * Clone a hit zone shape (geometry only) and assign the given overlay class.
 * Removes hit-zone-specific attributes so it doesn't capture clicks.
 *
 * @param {Element} target
 * @param {string}  cls     - CSS class ('press-ring' | 'selection-ring')
 * @returns {Element}
 */
function _cloneAsOverlay(target, cls) {
  const ring = target.cloneNode(false);
  ring.removeAttribute('class');
  ring.removeAttribute('data-id');
  ring.removeAttribute('data-label');
  ring.classList.add(cls);
  return ring;
}

// ─── SVG primitive factories ──────────────────────────────────────────────────

function mkRect(ns, x, y, w, h, cls) {
  const r = document.createElementNS(ns, 'rect');
  r.setAttribute('x',      String(x));
  r.setAttribute('y',      String(y));
  r.setAttribute('width',  String(w));
  r.setAttribute('height', String(h));
  r.setAttribute('class',  cls);
  return r;
}

function mkCircle(ns, cx, cy, r, cls) {
  const c = document.createElementNS(ns, 'circle');
  c.setAttribute('cx',    String(cx));
  c.setAttribute('cy',    String(cy));
  c.setAttribute('r',     String(r));
  c.setAttribute('class', cls);
  return c;
}

/**
 * Pentagon-shaped arrow hit zone pointing outward from a centre point.
 * Used for the four d-pad directions: a small flat-tail / pointed-tip
 * shape that sits over one arm of the cross sprite so the bound
 * direction tints with its own outline and the press-ring animation
 * follows the same arrow silhouette.
 *
 * Layout (for `up`):
 *
 *        apex
 *         /\
 *        /  \
 *       |    |    shoulders
 *        \__/     base
 *
 * @param {string} ns
 * @param {number} cx   - D-pad centre X
 * @param {number} cy   - D-pad centre Y
 * @param {string} dir  - 'up' | 'down' | 'left' | 'right'
 * @param {string} cls
 * @returns {SVGPathElement}
 */
function mkArrow(ns, cx, cy, dir, cls) {
  // "Label" pentagon: flat outer base + two parallel sides + a tapered
  // apex pointing INWARD toward the d-pad centre. The apex sides are
  // strictly 45°, and the parametrisation enforces
  // `R_shoulder = R_inner + half_w`. Under this constraint the apex
  // side of UP and the adjacent apex side of RIGHT lie on parallel
  // lines (y = -x ± R_inner), so the gap between every pair of
  // adjacent pentagons is uniformly `R_inner * √2`.
  //
  //         ┌──────┐     <- outer base (R_outer from d-pad centre)
  //         │      │     <- parallel vertical sides (R_outer → R_shoulder)
  //          \    /      <- 45° apex sides
  //            v         <- apex tip (R_inner)
  const R_inner    = 3;    // apex tip, closest point to d-pad centre
  const half_w     = 5;    // half-width of the body
  const R_shoulder = R_inner + half_w; // = 8, enforces 45° apex sides
  const R_outer    = 14;   // flat base far edge, from d-pad centre

  const up = [
    [-half_w, -R_outer],     // outer base left
    [+half_w, -R_outer],     // outer base right
    [+half_w, -R_shoulder],  // shoulder right
    [0,       -R_inner],     // apex (toward centre)
    [-half_w, -R_shoulder],  // shoulder left
  ];
  const map = {
    up:    ([x, y]) => [x,  y],
    down:  ([x, y]) => [x, -y],
    left:  ([x, y]) => [y,  x],
    right: ([x, y]) => [-y, x],
  }[dir];

  const pts = up
    .map(map)
    .map(([dx, dy]) => `${cx + dx} ${cy + dy}`)
    .join(' L ');
  const d = `M ${pts} Z`;

  const p = document.createElementNS(ns, 'path');
  p.setAttribute('d',     d);
  p.setAttribute('class', cls);
  return p;
}

/**
 * Trapezoid hit zone around a stick well. Outer base is flat
 * (perpendicular to the direction), inner base is flat and closer to
 * the stick well, the two side edges are strictly 45° lines. Adjacent
 * trapezoids' diagonal edges lie on parallel lines, so the gap
 * between every pair of adjacent quarters is uniformly `g * √2`
 * along the diagonal — same parallel-gap geometry the d-pad pentagon
 * uses (`mkArrow`).
 *
 * @param {string} ns
 * @param {number} cx   - Stick centre X
 * @param {number} cy   - Stick centre Y
 * @param {string} dir  - 'up' | 'down' | 'left' | 'right'
 * @param {string} cls
 * @returns {SVGPathElement}
 */
function mkQuarter(ns, cx, cy, dir, cls) {
  const r_in  = 10;   // inner base, just outside the stick-well ring (r=9)
  const r_out = 17;   // outer base, far edge
  const g     = 2;    // perpendicular gap half-width along the diagonals
  const k     = g * Math.SQRT2 / 2;  // chord offset to push the diagonals in

  // Canonical "up" trapezoid in stick-local coordinates.
  // Outer corners sit at x = ±(r_out - k); inner corners at x = ±(r_in - k).
  // The side from (r_out - k, -r_out) to (r_in - k, -r_in) has slope −1
  // (since Δy = r_out − r_in, Δx = −(r_out − r_in)), i.e. 45°.
  const up = [
    [-(r_out - k), -r_out],  // outer-left
    [ (r_out - k), -r_out],  // outer-right
    [ (r_in  - k), -r_in ],  // inner-right
    [-(r_in  - k), -r_in ],  // inner-left
  ];
  const map = {
    up:    ([x, y]) => [x,  y],
    down:  ([x, y]) => [x, -y],
    left:  ([x, y]) => [y,  x],
    right: ([x, y]) => [-y, x],
  }[dir];

  const pts = up
    .map(map)
    .map(([dx, dy]) => `${cx + dx} ${cy + dy}`)
    .join(' L ');
  const d = `M ${pts} Z`;

  const p = document.createElementNS(ns, 'path');
  p.setAttribute('d',     d);
  p.setAttribute('class', cls);
  return p;
}
