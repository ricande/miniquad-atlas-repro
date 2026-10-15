# Miniquad depth-state reproductions and proposed fix

Independent verification of [#670](https://github.com/not-fl3/miniquad/issues/670)
and [#597](https://github.com/not-fl3/miniquad/issues/597). This package is separate
from the atlas reproduction at the repository root.

**Current proposal:** [combined-depth.patch](combined-depth.patch), without a
depth/offset cache. See [DESIGN.md](DESIGN.md) for the reasoning and compatibility
tradeoffs, and [UPSTREAM.md](UPSTREAM.md) for the checked upstream revision/status.
No cache experiment is included as a recommended alternative.

## Reproduce locally

Requires Python 3, Git, `patch`, Rust/Cargo, and an interactive graphical session.
The recorded native runs explicitly use macOS OpenGL, not Miniquad's Metal backend.
The driver reports a Metal implementation underneath OpenGL; that does not change
which Miniquad backend was tested. Other platforms are not verified here.

From the repository root:

```sh
python3 depth/run.py
```

The runner clones upstream, checks out pinned commit
`39c928fb7e09fd44e45f37fe9a98503968bca412`, makes separate baseline/fixed copies,
applies the patch only to the fixed copy, builds with the included Cargo.lock,
and runs each native test three times with a 45-second per-process timeout.
It does not modify a Cargo registry source or any external checkout.

To reuse an existing **clean Git checkout of that exact commit** and cached Cargo
dependencies:

```sh
python3 depth/run.py --source /path/to/miniquad --offline
```

Generated sources, builds, environment and logs live under a new ignored
`depth/.work/run-*` directory printed by the runner. Baseline reproduction tests
intentionally exit 1; the runner succeeds only when baseline/fixed outcomes match
expectations. A reported PASS for baseline means the expected bug was reproduced,
not that the baseline library is correct. No headless-CI claim is made.

## Tests and expected behavior

- `src/depth-670.rs`: opaque foreground plus transparent rear geometry. The rear
  geometry must remain occluded with depth writes disabled. Controls cover writes
  enabled, the direct-mask workaround, and visible foreground transparency.
- `src/offset-597.rs`: coplanar quads with positive/negative offset, comparison
  controls and direct-GL controls. API offsets must change visibility like the
  corresponding direct GL settings.
- `src/regressions.rs`: 115 pixel/state/error assertions, including depth
  comparisons, writing with Always, clear/restore, offset transitions and repeated
  application of the same pipeline after external GL changes. Counts are
  assertions, not 115 distinct bugs. This is not an exhaustive comparison matrix.
- `src/macroquad-integration.rs`: 70 checks over ten frames covering 3D occlusion,
  2D overlays, depth clearing, render targets, alpha blending, text and the default
  framebuffer. Both baseline and fixed should pass this integration test.

## Recorded upstream results

2026-10-15, Apple M2, arm64, macOS 26.5.2; OpenGL renderer `Apple M2`, version
`4.1 Metal - 90.5`; rustc 1.99.1; Macroquad 0.4.16. Machine identity and architecture
were checked locally before testing; personal hostnames and absolute home paths
are intentionally not included in the shareable logs.

| Test | Unmodified pinned master | Same master with patch |
| --- | --- | --- |
| #670 reproduction | reproduced, 3/3 | absent, 3/3 |
| #597 reproduction | reproduced, 3/3 | absent, 3/3 |
| Regression suite | 37 failures / 115 checks, each run | 0 / 115, each run |
| Macroquad integration | 0 / 70, each run | 0 / 70, each run |

See [results/upstream](results/upstream) for all 24 native logs, environment and
runner outcomes. A further one-run-per-test smoke check of the final runner with
the packaged lockfile also passed; its logs are in [results/locked-smoke](results/locked-smoke).
These are runtime tests against the Git checkout, not just comparison of a release
crate's source with master.

## Optional WebGL builds and browser test

```sh
rustup target add wasm32-unknown-unknown
python3 depth/run.py --wasm
python3 depth/serve.py depth/.work/run-REPLACE-WITH-PRINTED-DIRECTORY
```

Open http://127.0.0.1:8765 and click **Run browser suite**. The loopback-only server
saves browser JSON beside the run logs. Stop it with Ctrl-C when finished.
Both variants use the patched JavaScript bridge so this comparison isolates the
Rust behavior. Baseline expects failed assertions; fixed expects zero failures.

The upstream baseline and fixed WebGL1/2 release builds succeeded. After an initial
localhost connection timeout, the browser test was rerun successfully before
publication: both unmodified variants failed 32 of 107 checks, and both patched
variants passed 107/107. All four outcomes matched expectations. The browser
reported Chrome 155 and an ANGLE Metal renderer on Apple M2. See
[results/upstream/browser-webgl.json](results/upstream/browser-webgl.json) for the
complete log. The WebGL2 framebuffer limitation below still applies.

Historical release-crate browser evidence is preserved in
[results/release-webgl.json](results/release-webgl.json): patched Miniquad 0.4.11
passed 107/107 in WebGL1 and WebGL2; negative controls failed as expected. That run
also includes an older Macroquad-bundle adapter case, not a full Macroquad game in
the browser. It is separate evidence, not a rerun of this upstream package.

Important test limitations: WebGL2 uses a test-created DEPTH_COMPONENT16 renderbuffer
because the original depth-texture setup produced an incomplete framebuffer. This
does not fix or validate Miniquad's ordinary WebGL2 depth-texture path. The test
page also supplies a real glGetError import so GL errors are observable. Neither
test accommodation is part of the library patch. Existing deployed JavaScript
bundles must include the two new GL imports when upgrading the Rust library.

## Scope

No Metal, native GLES, Windows/Linux driver, multisampling/stencil combination or
large-game verification is claimed. This proposal addresses depth state, not every
raw-GL interoperability issue. The atlas patch is not applied. No FPS improvement
is claimed. This is a verification package and proposed patch, not an upstream-merged fix.
