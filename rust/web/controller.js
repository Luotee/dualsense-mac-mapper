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
// Gap between adjacent touchpad quadrants — matches the visual spacing
// pattern used by the d-pad pentagons and stick wedges. Tune here if a
// designer wants a tighter or wider cross-line.
const TOUCHPAD_QUAD_GAP = 1.5;

// Issue 7: subtle rounded corners for D-pad wedges + stick donut quarters.
// 0 = sharp (v2.0.0 behaviour). Tuned to ~0.8 to match L1 button feel.
export const CORNER_RADIUS = {
  dpad: 0.8,
  stickSlice: 0.7,
};

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

    // Touchpad — 4 quadrant hit zones with a centre-cross gap matching
    // the d-pad / stick-wedge spacing convention. Each quadrant shrinks
    // by GAP/2 along the centre axes; the decorative pad rect drawn in
    // render() shows through the gap so the visual centre-line reads.
    ...touchpadQuadElements(),

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

function touchpadQuadElements() {
  const g = TOUCHPAD_QUAD_GAP;
  const r = TOUCHPAD.rx;
  const cx = TOUCHPAD.x + TOUCHPAD.w / 2;
  const cy = TOUCHPAD.y + TOUCHPAD.h / 2;
  const w = TOUCHPAD.w / 2 - g / 2;
  const h = TOUCHPAD.h / 2 - g / 2;
  const leftX  = TOUCHPAD.x;
  const rightX = cx + g / 2;
  const topY   = TOUCHPAD.y;
  const botY   = cy + g / 2;
  // `corner` names which of the 4 corners is the outer rounded one;
  // the other three corners are sharp (inner edges meeting the gap).
  return [
    el(25, 'touchpad_quad', { x: leftX,  y: topY, w, h, r, corner: 'tl' }, 'TP-TL'),
    el(26, 'touchpad_quad', { x: rightX, y: topY, w, h, r, corner: 'tr' }, 'TP-TR'),
    el(27, 'touchpad_quad', { x: leftX,  y: botY, w, h, r, corner: 'bl' }, 'TP-BL'),
    el(28, 'touchpad_quad', { x: rightX, y: botY, w, h, r, corner: 'br' }, 'TP-BR'),
  ];
}

// ─── Binding → CSS class ──────────────────────────────────────────────────────

function kindClass(kind) {
  switch (kind) {
    case 'key':     return 'binding-key';
    case 'macro':   return 'binding-macro';
    case 'mouse':   return 'binding-mouse';
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
      case 'touchpad_quad': {
        // Each quadrant fills the touchpad rounded rect with ONE outer
        // corner curved (matching the pad's rx) and three inner corners
        // sharp, so the four quadrants together rebuild the rounded
        // touchpad outline with a cross-shaped gap in the middle.
        shape = mkTouchpadQuad(ns, e.geo, `hit ${cls}`);
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

// ─── Touchpad debug dot ────────────────────────────────────────────────────────
//
// Renders a small dot on the touchpad SVG at the proportional position
// of the most recent click. Lets the user verify the raw coords being
// captured match where they actually touched — if they don't, the
// `touchpad_midpoint_x` / `touchpad_midpoint_y` settings can be tuned
// to match the user's physical pad coordinate range.

const TOUCHPAD_RAW_MAX_X = 1919;
const TOUCHPAD_RAW_MAX_Y = 1079;
let _debugDot = null;
let _debugDotTimer = null;

/**
 * Show a debug dot at the touchpad position corresponding to (raw_x,
 * raw_y). The dot fades by being removed after a short timeout.
 *
 * @param {SVGElement} svg
 * @param {number} raw_x  - 0..1919 (or whatever the pad reports)
 * @param {number} raw_y  - 0..1079
 */
export function showTouchpadDot(svg, raw_x, raw_y) {
  if (!svg) return;
  if (_debugDot && _debugDot.parentNode) _debugDot.remove();
  if (_debugDotTimer) clearTimeout(_debugDotTimer);
  const ns = 'http://www.w3.org/2000/svg';
  const dot = document.createElementNS(ns, 'circle');
  const cx = TOUCHPAD.x + (raw_x / TOUCHPAD_RAW_MAX_X) * TOUCHPAD.w;
  const cy = TOUCHPAD.y + (raw_y / TOUCHPAD_RAW_MAX_Y) * TOUCHPAD.h;
  dot.setAttribute('cx', String(cx));
  dot.setAttribute('cy', String(cy));
  dot.setAttribute('r',  '1.2');
  dot.setAttribute('class', 'touchpad-debug-dot');
  svg.appendChild(dot);
  _debugDot = dot;
  _debugDotTimer = setTimeout(() => {
    if (_debugDot === dot && dot.parentNode) dot.remove();
    if (_debugDot === dot) _debugDot = null;
    _debugDotTimer = null;
  }, 1200);
}

// ─── Touchpad hover preview ───────────────────────────────────────────────────
//
// Issue 3: continuous hover preview. While the finger is active, the engine
// emits 'touchpad-hover' per frame on quadrant change (dedupe-on-change),
// and on lift emits a sentinel `quadrant=255`. The frontend highlights the
// active quadrant and renders a persistent debug dot at the raw position.
// Distinct from `showTouchpadDot` (transient post-click) — this one updates
// live and persists while the finger is down.

let _hoverDot = null;

/**
 * Highlight quadrant `quadrantId` (25..=28) and move a persistent debug dot
 * to the proportional position of (rawX, rawY) inside the touchpad bbox.
 */
export function showTouchpadHover(svg, quadrantId, rawX, rawY) {
  if (!svg) return;
  // Highlight: add `hover` class to the matching quadrant, remove from others.
  for (const id of [25, 26, 27, 28]) {
    const el = svg.querySelector(`[data-id="${id}"]`);
    if (!el) continue;
    if (id === quadrantId) el.classList.add('hover');
    else                   el.classList.remove('hover');
  }
  // Debug dot: live-updated while finger is down.
  const ns = 'http://www.w3.org/2000/svg';
  if (!_hoverDot || !_hoverDot.parentNode) {
    _hoverDot = document.createElementNS(ns, 'circle');
    _hoverDot.setAttribute('r', '1.0');
    _hoverDot.setAttribute('class', 'touchpad-hover-dot');
    svg.appendChild(_hoverDot);
  }
  const cx = TOUCHPAD.x + (rawX / TOUCHPAD_RAW_MAX_X) * TOUCHPAD.w;
  const cy = TOUCHPAD.y + (rawY / TOUCHPAD_RAW_MAX_Y) * TOUCHPAD.h;
  _hoverDot.setAttribute('cx', String(cx));
  _hoverDot.setAttribute('cy', String(cy));
}

/**
 * Clear the hover highlight and remove the debug dot. Called on finger
 * lift (engine sentinel `quadrant=255`).
 */
export function clearTouchpadHover(svg) {
  if (!svg) return;
  for (const id of [25, 26, 27, 28]) {
    const el = svg.querySelector(`[data-id="${id}"]`);
    if (el) el.classList.remove('hover');
  }
  if (_hoverDot && _hoverDot.parentNode) {
    _hoverDot.remove();
  }
  _hoverDot = null;
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
 * Rectangle with exactly one rounded corner (the "outer" corner of a
 * touchpad quadrant). The other three corners are sharp so adjacent
 * quadrants share straight edges across the centre gap and the four
 * quadrants together rebuild the pad's rounded outer outline.
 *
 * `geo` shape:
 *   { x, y, w, h, r, corner: 'tl' | 'tr' | 'bl' | 'br' }
 */
function mkTouchpadQuad(ns, geo, cls) {
  const { x, y, w, h, r, corner } = geo;
  const path = (() => {
    const x1 = x + w, y1 = y + h;
    switch (corner) {
      case 'tl':
        return `M ${x} ${y + r} A ${r} ${r} 0 0 1 ${x + r} ${y} L ${x1} ${y} L ${x1} ${y1} L ${x} ${y1} Z`;
      case 'tr':
        return `M ${x} ${y} L ${x1 - r} ${y} A ${r} ${r} 0 0 1 ${x1} ${y + r} L ${x1} ${y1} L ${x} ${y1} Z`;
      case 'bl':
        return `M ${x} ${y} L ${x1} ${y} L ${x1} ${y1} L ${x + r} ${y1} A ${r} ${r} 0 0 1 ${x} ${y1 - r} Z`;
      case 'br':
      default:
        return `M ${x} ${y} L ${x1} ${y} L ${x1} ${y1 - r} A ${r} ${r} 0 0 1 ${x1 - r} ${y1} L ${x} ${y1} Z`;
    }
  })();
  const p = document.createElementNS(ns, 'path');
  p.setAttribute('d',     path);
  p.setAttribute('class', cls);
  return p;
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
  // Tuned via tools/controller_tuner.html on 2026-05-17: tighter
  // cluster + narrower gap aligned with the 2-unit L1↔L2 spacing.
  // gap = R_inner · √2 ≈ 3.82, length × width = 10.5 × 6.8 (ratio 1.54).
  const R_inner    = 2.70;
  const half_w     = 3.40;
  const R_shoulder = R_inner + half_w; // = 6.10, enforces 45° apex sides
  const R_outer    = 13.20;

  const up = [
    [-half_w, -R_outer],     // outer base left
    [+half_w, -R_outer],     // outer base right
    [+half_w, -R_shoulder],  // shoulder right
    [0,       -R_inner],     // apex (toward centre)
    [-half_w, -R_shoulder],  // shoulder left
  ];
  // Proper rotations (not reflections) so arc sweep direction is
  // preserved when this same map is reused inside `mkQuarter`. Down was
  // previously `(x, -y)` (reflection across x-axis) which flipped the
  // path winding — see v1.1.3 release notes.
  const map = {
    up:    ([x, y]) => [ x,  y],   // 0°
    right: ([x, y]) => [-y,  x],   // 90° CW
    down:  ([x, y]) => [-x, -y],   // 180°
    left:  ([x, y]) => [ y, -x],   // 90° CCW
  }[dir];

  const rotated = up.map(map).map(([dx, dy]) => [cx + dx, cy + dy]);
  const d = buildRoundedPolygonPath(rotated, CORNER_RADIUS.dpad, [0, 1]);

  const p = document.createElementNS(ns, 'path');
  p.setAttribute('d',     d);
  p.setAttribute('class', cls);
  return p;
}

/**
 * Donut quarter hit zone around a stick well — outer + inner arcs,
 * with the two SIDE boundaries being parallel chord-lines instead of
 * radii. Adjacent quarters' side chords are offset on opposite sides
 * of the same diagonal, so the gap between every pair of adjacent
 * quarters is uniformly `g · √2` along the perpendicular.
 *
 * Geometry (local stick-centre coords, "up" canonical):
 *   - Right side line: y = -x - d   (parallel to 135° diagonal)
 *   - Left  side line: y =  x - d   (parallel to 45° diagonal)
 *   - Outer arc:       x² + y² = r_out²
 *   - Inner arc:       x² + y² = r_in²
 *   where d > 0 pushes both side lines toward UP.
 *
 * Vertices for "up" come out symmetric in x:
 *   outer-right = ( x_out, -x_out - d ),
 *   outer-left  = (-x_out, -x_out - d ),
 *   inner-left  = (-x_in,  -x_in  - d ),
 *   inner-right = ( x_in,  -x_in  - d ),
 * where x = (-d + √(2r² - d²)) / 2 for radius r.
 *
 * @param {string} ns
 * @param {number} cx   - Stick centre X
 * @param {number} cy   - Stick centre Y
 * @param {string} dir  - 'up' | 'down' | 'left' | 'right'
 * @param {string} cls
 * @returns {SVGPathElement}
 */
function mkQuarter(ns, cx, cy, dir, cls) {
  // Tuned via tools/controller_tuner.html on 2026-05-17: thinner donut
  // ring (3.5 wide instead of 7) with a narrower 1.7·√2 ≈ 2.4 gap, to
  // match the L1↔L2 spacing the user picked as the visual reference.
  const r_in  = 11.60;  // inner arc, just outside the stick-well ring (r=9)
  const r_out = 15.10;  // outer arc, reach into the live-press area
  const d     = 1.70;   // perpendicular offset of the side chord from origin

  // x-coordinate where the line y = -x - d meets a circle of radius r.
  const xAt = (r) => (-d + Math.sqrt(2 * r * r - d * d)) / 2;
  const x_out = xAt(r_out);
  const x_in  = xAt(r_in);

  // Canonical "up" quarter vertices (start at outer-right, go CCW so
  // the outer arc travels through the top of the circle).
  const upVerts = [
    [ x_out, -x_out - d],  // outer-right (on right chord ∩ outer arc)
    [-x_out, -x_out - d],  // outer-left  (on left chord  ∩ outer arc)
    [-x_in,  -x_in  - d],  // inner-left  (on left chord  ∩ inner arc)
    [ x_in,  -x_in  - d],  // inner-right (on right chord ∩ inner arc)
  ];
  // Proper rotations (not reflections) so SVG arc sweep direction stays
  // valid after transform. Same fix as `mkArrow`'s map — see v1.1.4
  // release notes. Reflection-based maps for `down`/`left` flipped the
  // arc direction and produced visibly broken quarters.
  const map = {
    up:    ([x, y]) => [ x,  y],   // 0°
    right: ([x, y]) => [-y,  x],   // 90° CW
    down:  ([x, y]) => [-x, -y],   // 180°
    left:  ([x, y]) => [ y, -x],   // 90° CCW
  }[dir];
  const [vO_R, vO_L, vI_L, vI_R] = upVerts.map(map).map(
    ([dx, dy]) => [cx + dx, cy + dy]);

  // SVG arc sweep flag picks the short way round. Outer arc travels
  // through the direction's outer tip (e.g. straight up for `up`);
  // inner arc travels the opposite sense back.
  const rr = CORNER_RADIUS.stickSlice;
  const pathD = buildRoundedQuarterPath(vO_R, vO_L, vI_L, vI_R, r_out, r_in, rr);

  const p = document.createElementNS(ns, 'path');
  p.setAttribute('d',     pathD);
  p.setAttribute('class', cls);
  return p;
}

// ─── Rounded corner helpers ───────────────────────────────────────────────────

/**
 * Build an SVG path for an arbitrary closed polygon with selected corners
 * rounded via quadratic Bezier inset.
 *
 * @param {Array<[number, number]>} verts - vertex coordinates in order
 * @param {number} rr                     - corner radius (0 = sharp)
 * @param {Array<number>} roundIdx        - indexes (into verts) of corners to round
 * @returns {string} SVG path d-attribute
 */
function buildRoundedPolygonPath(verts, rr, roundIdx) {
  const n = verts.length;
  if (rr <= 0 || roundIdx.length === 0) {
    return 'M ' + verts.map(([x, y]) => `${x} ${y}`).join(' L ') + ' Z';
  }
  const shouldRound = new Set(roundIdx);
  // For each vertex P, compute the inset points along P→prev and P→next
  // (only if P is in roundIdx). Otherwise the segment ends/begins at P itself.
  const segs = [];
  for (let i = 0; i < n; i++) {
    const prev = verts[(i - 1 + n) % n];
    const cur  = verts[i];
    const next = verts[(i + 1) % n];
    if (!shouldRound.has(i)) {
      segs.push({ start: cur, end: cur, control: null });
      continue;
    }
    // Inset toward prev and toward next by rr.
    const towardPrev = insetAlong(cur, prev, rr);
    const towardNext = insetAlong(cur, next, rr);
    segs.push({ start: towardPrev, end: towardNext, control: cur });
  }
  // Build path: start at first segment's start (if rounded) or first vertex.
  // Then for each vertex: L to seg.start (the inset-from-prev), Q cur seg.end (if rounded).
  let d = `M ${formatPt(segs[0].start)}`;
  for (let i = 0; i < n; i++) {
    const s = segs[i];
    if (s.control !== null) {
      // Round: Q from current point through control (corner) to seg.end (inset toward next).
      d += ` Q ${formatPt(s.control)} ${formatPt(s.end)}`;
    }
    // L to next segment's start.
    const nextSeg = segs[(i + 1) % n];
    if (nextSeg.start !== s.end || s.control === null) {
      d += ` L ${formatPt(nextSeg.start)}`;
    }
  }
  d += ' Z';
  return d;
}

function insetAlong(from, to, rr) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const len = Math.hypot(dx, dy);
  if (len < rr * 2) return [...from];  // edge too short to inset cleanly
  return [from[0] + (dx / len) * rr, from[1] + (dy / len) * rr];
}

function formatPt(p) {
  return `${p[0].toFixed(3)} ${p[1].toFixed(3)}`;
}

/**
 * Build an SVG path for a rounded donut quarter slice.
 * Vertices in CCW order: outer-right (vOR), outer-left (vOL), inner-left (vIL), inner-right (vIR).
 * Outer arc connects vOR → vOL through the outer tip.
 * Inner arc connects vIL → vIR through the inner tip.
 *
 * For each corner, the inset along an arc edge uses chord direction
 * (approximation valid for rr <= 5 visually).
 */
function buildRoundedQuarterPath(vOR, vOL, vIL, vIR, rOut, rIn, rr) {
  if (rr <= 0) {
    return [
      `M ${vOR[0]} ${vOR[1]}`,
      `A ${rOut} ${rOut} 0 0 0 ${vOL[0]} ${vOL[1]}`,
      `L ${vIL[0]} ${vIL[1]}`,
      `A ${rIn}  ${rIn}  0 0 1 ${vIR[0]} ${vIR[1]}`,
      'Z',
    ].join(' ');
  }
  // Inset each corner along its two incident edges.
  // vOR neighbours: outer-arc toward vOL (chord-approx), and line-edge toward vIR.
  // vOL neighbours: outer-arc toward vOR, and line-edge toward vIL.
  // vIL neighbours: line-edge toward vOL, and inner-arc toward vIR.
  // vIR neighbours: inner-arc toward vIL, and line-edge toward vOR.
  const vOR_toOL  = insetAlong(vOR, vOL, rr);
  const vOR_toIR  = insetAlong(vOR, vIR, rr);
  const vOL_toOR  = insetAlong(vOL, vOR, rr);
  const vOL_toIL  = insetAlong(vOL, vIL, rr);
  const vIL_toOL  = insetAlong(vIL, vOL, rr);
  const vIL_toIR  = insetAlong(vIL, vIR, rr);
  const vIR_toIL  = insetAlong(vIR, vIL, rr);
  const vIR_toOR  = insetAlong(vIR, vOR, rr);
  // Path: start at vOR's outer-arc-side inset, arc to vOL's outer-arc-side inset (smaller sweep since insets shorten the arc).
  return [
    `M ${formatPt(vOR_toOL)}`,
    `A ${rOut} ${rOut} 0 0 0 ${formatPt(vOL_toOR)}`,
    `Q ${formatPt(vOL)} ${formatPt(vOL_toIL)}`,
    `L ${formatPt(vIL_toOL)}`,
    `Q ${formatPt(vIL)} ${formatPt(vIL_toIR)}`,
    `A ${rIn}  ${rIn}  0 0 1 ${formatPt(vIR_toIL)}`,
    `Q ${formatPt(vIR)} ${formatPt(vIR_toOR)}`,
    `L ${formatPt(vOR_toIR)}`,
    `Q ${formatPt(vOR)} ${formatPt(vOR_toOL)}`,
    'Z',
  ].join(' ');
}
