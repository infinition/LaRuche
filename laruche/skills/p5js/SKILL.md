---
type: skill
name: p5js
description: "p5.js creative coding: generative art, WebGL, interactive, export."
version: 1.0.0
platforms: [linux, macos, windows]
tools: [shell_exec, file_write, browser_navigate, browser_screenshot]
scripts:
  - scripts/serve.sh
  - scripts/render.sh
  - scripts/export-frames.js
  - scripts/setup.sh
metadata:
  laruche:
    tags: [creative-coding, generative-art, p5js, canvas, interactive, visualization, webgl, shaders, animation]
---

# p5.js Production Pipeline

## When to use

p5.js sketches, creative coding, generative art, interactive visualizations, canvas animations, browser-based visual art, data viz, shader effects.

## Modes

| Mode | Output | Reference |
|------|--------|-----------|
| Generative art | Procedural still or animated composition | `references/visual-effects.md` |
| Data visualization | Interactive charts / custom data display | `references/interaction.md` |
| Interactive experience | Mouse/keyboard/touch-driven sketch | `references/interaction.md` |
| Animation / motion graphics | Timed sequences, kinetic typography | `references/animation.md` |
| 3D scene | WebGL geometry, lighting, camera, materials | `references/webgl-and-3d.md` |
| Image processing | Pixel manipulation, filters, mosaic, pointillism | `references/visual-effects.md` § Pixel Manipulation |
| Audio-reactive | Sound-driven generative visuals | `references/interaction.md` § Audio Input |

## Stack

Single self-contained HTML file per project. No build step.

| Layer | Tool | Notes |
|-------|------|-------|
| Core | p5.js 1.11.3 (CDN) | Default - stable, broadest library compat |
| 3D | p5.js WebGL mode | 3D geometry, camera, lighting, GLSL |
| Audio | p5.sound.js (CDN) | FFT analysis, amplitude, mic, oscillators |
| Export | `saveCanvas()` / `saveGif()` / `saveFrames()` | PNG, GIF, frame sequence |
| Capture | CCapture.js (optional) | Deterministic framerate video (WebM, GIF) |
| Headless | Puppeteer + Node.js (optional) | Automated high-res render, MP4 via ffmpeg |
| SVG | p5.js-svg 1.6.0 (optional) | Vector output - requires p5.js 1.x |
| Natural media | p5.brush (optional) | Watercolor, charcoal, pen - requires p5.js 2.x + WEBGL |
| Texture | p5.grain (optional) | Film grain overlays |

**p5.js 2.x** (2.2+): `async setup()` replaces `preload()`, OKLCH/OKLAB color modes, `splineVertex()`, shader `.modify()`, `textToContours()`. Required for p5.brush. See `references/core-api.md` § p5.js 2.0.

## Pipeline

```
CONCEPT → DESIGN → CODE → PREVIEW → EXPORT → VERIFY
```

### Step 1: Creative Vision

Before any code, decide: mood/atmosphere, color world, shape language, motion vocabulary, what makes this unique. A "relaxing generative background" demands different everything from "glitch data visualization." Interpret the user's prompt with creative ambition - deliver at least one visual detail they didn't request but will appreciate.

**Non-negotiables per project:**
- Custom color palette (3-7 colors, never raw `fill(255, 0, 0)`)
- Non-trivial background (textured, gradient, or layered - never plain `background(0)`)
- Motion variety (primary 1x, secondary 0.3x, ambient 0.1x)
- Seeded randomness for reproducibility

### Step 2: Technical Design

- **Mode** - pick from the table above
- **Canvas** - 1920×1080 (landscape), 1080×1920 (portrait), 1080×1080 (square), or `windowWidth/windowHeight` (responsive)
- **Renderer** - `P2D` (default) or `WEBGL` (3D, shaders, advanced blend modes)
- **Frame rate** - 60fps (interactive), 30fps (ambient), `noLoop()` (static generative)
- **Export target** - browser, PNG, GIF, MP4, SVG
- **Viewer UI** - for interactive generative art with seed navigation + sliders, start from `templates/viewer.html`. For animations/video/simple sketches, use bare HTML.

### Step 3: Code the Sketch

For **interactive generative art**: read `templates/viewer.html` first, keep the seed nav/actions sections, replace the algorithm and parameter controls.

For **animations, video, or simple sketches** - bare HTML template:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Project Name</title>
  <script>p5.disableFriendlyErrors = true;</script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/p5.js/1.11.3/p5.min.js"></script>
  <!-- <script src="https://cdnjs.cloudflare.com/ajax/libs/p5.js/1.11.3/addons/p5.sound.min.js"></script> -->
  <!-- <script src="https://unpkg.com/p5.js-svg@1.6.0"></script> -->
  <!-- <script src="https://cdn.jsdelivr.net/npm/ccapture.js-npmfixed/build/CCapture.all.min.js"></script> -->
  <style>
    html, body { margin: 0; padding: 0; overflow: hidden; }
    canvas { display: block; }
  </style>
</head>
<body>
<script>
const CONFIG = { seed: 42 };
const PALETTE = { bg: '#0a0a0f', primary: '#e8d5b7' };
let particles = [];

function preload() { /* loadFont, loadImage, loadSound */ }

function setup() {
  createCanvas(1920, 1080);
  pixelDensity(1);
  randomSeed(CONFIG.seed);
  noiseSeed(CONFIG.seed);
  colorMode(HSB, 360, 100, 100, 100);
}

function draw() { /* render frame */ }

class Particle {
  constructor() {}
  update() {}
  display() {}
}

function keyPressed() {
  if (key === 's' || key === 'S') saveCanvas('output', 'png');
  if (key === 'g' || key === 'G') saveGif('output', 5);
  if (key === 'r' || key === 'R') { randomSeed(millis()); noiseSeed(millis()); }
  if (key === ' ') CONFIG.paused = !CONFIG.paused;
}
function windowResized() { resizeCanvas(windowWidth, windowHeight); }
</script>
</body>
</html>
```

Key patterns:
- **Seeded randomness**: `randomSeed()` + `noiseSeed()` in every generative sketch
- **Color mode**: `colorMode(HSB, 360, 100, 100, 100)` - rotate hue, scale sat/bri procedurally
- **Layers**: `createGraphics()` offscreen buffers for trails, masks, compositing
- **Classes**: Particles/agents with `update()` + `display()` methods

### Step 4: Preview & Iterate

- Open HTML directly in browser (no server needed for CDN-only sketches)
- Local assets (fonts, images) require a server: `shell_exec: bash scripts/serve.sh` or `python3 -m http.server 8080` then `http://localhost:8080/sketch.html`
- Use `browser_navigate` + `browser_screenshot` to capture output from LaRuche
- Chrome DevTools Performance tab to verify fps
- Test at target export resolution

### Step 5: Export

| Format | Method | Command |
|--------|--------|---------|
| PNG | `saveCanvas('output', 'png')` | Press 's' |
| High-res PNG | Puppeteer headless | `shell_exec: node scripts/export-frames.js sketch.html --width 3840 --height 2160 --frames 1` |
| GIF | `saveGif('output', 5)` | Press 'g' |
| Frame sequence | `saveFrames('frame', 'png', 10, 30)` | Then `shell_exec: ffmpeg -i frame-%04d.png -c:v libx264 output.mp4` |
| MP4 | Puppeteer + ffmpeg | `shell_exec: bash scripts/render.sh sketch.html output.mp4 --duration 30 --fps 30` |
| SVG | `createCanvas(w, h, SVG)` + p5.js-svg | `save('output.svg')` |

### Step 6: Verify

- Does output match the creative concept? If it looks generic, rethink Step 1.
- Sharp at target display size? No aliasing?
- Holds target fps? Colors work together? Edge cases handled (resize, 10-min run)?
- Use `browser_screenshot` to capture a reference frame for the user.

## Critical Implementation Notes

### Performance

Disable FES in every production sketch:

```javascript
p5.disableFriendlyErrors = true;  // BEFORE setup()
// pixelDensity(1) in setup() prevents 2x-4x overdraw on retina
```

In hot loops, use `Math.*` - measurably faster than p5 wrappers:

```javascript
Math.sin(t)            // not sin(t)
Math.sqrt(dx*dx+dy*dy) // not dist() - or skip sqrt, compare magSq
Math.random()          // not random() - when seed not needed
Math.min(a, b)         // not min(a, b)
```

Never `console.log()` inside `draw()`. Never manipulate DOM in `draw()`.

### Vectorize for Large Particle Counts

```javascript
// Slow: individual ellipse() per particle
// Fast: single beginShape(POINTS)
beginShape(POINTS);
for (let p of particles) vertex(p.x, p.y);
endShape();

// Fastest: pixel buffer
loadPixels();
for (let p of particles) {
  let idx = 4 * (floor(p.y) * width + floor(p.x));
  pixels[idx] = r; pixels[idx+1] = g; pixels[idx+2] = b; pixels[idx+3] = 255;
}
updatePixels();
```

Targets: 5k-10k particles (P2D shapes), 50k-100k (pixel buffer) at 60fps.

### Noise - Layer Octaves, Not Raw

```javascript
function fbm(x, y, octaves = 4) {
  let val = 0, amp = 1, freq = 1, sum = 0;
  for (let i = 0; i < octaves; i++) {
    val += noise(x * freq, y * freq) * amp;
    sum += amp; amp *= 0.5; freq *= 2;
  }
  return val / sum;
}
```

For organic flow: domain warping (feed noise output back as input coordinates). See `references/visual-effects.md`.

### Layers - createGraphics()

```javascript
let bgLayer, fgLayer, trailLayer;
function setup() {
  bgLayer = createGraphics(width, height);
  fgLayer = createGraphics(width, height);
  trailLayer = createGraphics(width, height);
}
function draw() {
  renderBackground(bgLayer);
  renderTrails(trailLayer);  // persistent, fading
  renderForeground(fgLayer); // cleared each frame
  image(bgLayer, 0, 0); image(trailLayer, 0, 0); image(fgLayer, 0, 0);
}
```

### WebGL Gotchas

- Origin is center, not top-left. Y-axis inverted (positive Y = up). Use `translate(-width/2, -height/2)` for P2D-like coords.
- `push()`/`pop()` around every transform - matrix stack overflows silently.
- `texture()` must come before `rect()`/`plane()`.
- Test custom shaders (`createShader(vert, frag)`) across browsers.

### Headless Video Export

Sketch **must** use `noLoop()` + `_p5Ready` signal. Without `noLoop()`, the draw loop races ahead of Puppeteer screenshots:

```javascript
function setup() {
  createCanvas(1920, 1080);
  pixelDensity(1);
  noLoop();
  window._p5Ready = true;  // signals capture script to start
}
```

`scripts/export-frames.js` detects `_p5Ready` and calls `redraw()` once per frame for exact 1:1 correspondence. For multi-scene videos: one HTML per scene, stitch with `ffmpeg -f concat`. See `references/export-pipeline.md`.

### Instance Mode (Multiple Sketches)

```javascript
const sketch = (p) => {
  p.setup = () => p.createCanvas(800, 800);
  p.draw  = () => { p.background(0); p.ellipse(p.mouseX, p.mouseY, 50); };
};
new p5(sketch, 'canvas-container');
```

Required when embedding multiple sketches on one page or integrating with frameworks.

### Generative Art Platforms (fxhash / Art Blocks)

```javascript
const SEED = $fx.hash;
const rng  = $fx.rand;
$fx.features({ palette: 'warm', complexity: 'high' });
// In setup():
randomSeed(SEED); noiseSeed(SEED);
let x = rng() * width;  // replace random() with rng() for platform determinism
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Frame rate (interactive) | 60fps sustained |
| Frame rate (animated export) | 30fps minimum |
| Canvas resolution (export) | Up to 3840×2160 |
| HTML file size | < 100KB (excluding CDN) |
| Load time | < 2s to first frame |

## References

| File | Contents |
|------|----------|
| `references/core-api.md` | Canvas setup, draw loop, `push()`/`pop()`, offscreen buffers, `pixelDensity()`, responsive design |
| `references/shapes-and-geometry.md` | Primitives, `beginShape()`, Bezier/Catmull-Rom, `p5.Vector`, SDFs, SVG path conversion |
| `references/visual-effects.md` | Noise (Perlin, fractal, domain warp, curl), flow fields, particles, pixel manipulation, texture gen, feedback loops |
| `references/animation.md` | Easing, `lerp()`/`map()`, spring physics, state machines, timeline sequencing, `millis()`-based timing |
| `references/typography.md` | `text()`, `loadFont()`, `textToPoints()`, kinetic typography, font metrics |
| `references/color-systems.md` | `colorMode()`, HSB/HSL/RGB, `lerpColor()`, procedural palettes, `blendMode()`, gradient rendering |
| `references/webgl-and-3d.md` | WEBGL renderer, 3D primitives, camera, lighting, GLSL shaders, framebuffers, post-processing |
| `references/interaction.md` | Mouse/keyboard/touch, DOM elements, p5.sound FFT/amplitude, scroll-driven animation |
| `references/export-pipeline.md` | `saveCanvas/Gif/Frames`, headless capture, ffmpeg, CCapture.js, SVG, per-clip architecture, fxhash |
| `references/troubleshooting.md` | Performance profiling, WebGL debugging, font loading, pixel density traps, memory leaks, CORS |
| `templates/viewer.html` | Seed navigation (prev/next/random/jump), parameter sliders, PNG download - start here for explorable gen art |
