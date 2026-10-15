# Miniquad #633: stale cache versus deleted application handle

Date: 2026-10-16 (Europe/Stockholm). Results from local Apple M2 testing.
This package publishes the reproducer and logs; it does not add a new library fix.

## Conclusion

**A concrete library-internal route to the deleted-texture message is reproduced,
and PR #691 removes it. The full mobile crash reported in #633 is NOT verified.**

The application can delete a texture through Miniquad, then update a different
live texture, without ever passing the deleted handle to another operation.
Unmodified Miniquad restores its stale cached binding and emits the deleted-texture
message. The proposed #691 change eliminates that message in this sequence.

Deliberately passing a deleted handle from application code produces the same
message even with #691. Consequently the message alone cannot distinguish stale
library cache from use of a dead application handle. This is not evidence that
#691 fixes every occurrence of #633.

## Sources and versions

- Issue: https://github.com/not-fl3/miniquad/issues/633 (open at check).
- PR: https://github.com/not-fl3/miniquad/pull/691 (open at check).
- Baseline: master 39c928fb7e09fd44e45f37fe9a98503968bca412, still remote HEAD at check.
- Fix: current PR diff, head fbc59729b24dd6c3c4052a729431fa54ffe42b0d, applied to a
  separate copy of that baseline. Only src/graphics/gl.rs and src/graphics/gl/cache.rs
  differ. No depth-state patch is included.
- Exact tested PR diff: pr691.patch.

## Local environment

hostname/pwd/uname -m verified at start: MacBook-Air-som-tillhor-Richard.local,
local project workspace, arm64. Apple M2, macOS 26.5.2, Rust 1.99.1 with
wasm32-unknown-unknown. In-app browser user agent Chrome 155; renderer:
ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version).

These were real WebGL1 and WebGL2 executions, not a simulated GL implementation.
The bridge was the unmodified upstream js/gl.js for both variants. The only
test-only import addition was glGetError forwarding to gl.getError, so the checks
do not use a missing-function stub. No error suppression or validator modification
was used. No offscreen depth attachment was needed.

## Test cases

Each iframe creates a fresh WASM instance/context. Textures A and B are allocated
before deletion, so the cache test does not rely on reusing a numeric texture ID.
All cases first draw A as red and assert the correct pixel and no GL error.

1. **Live control:** keep A alive; update B to green while A is cached as bound.
   Restoring A is valid; no deleted-texture message is expected.
2. **Cache-only case:** bind/draw A, delete A using Miniquad, then update only B.
   Miniquad's texture-update path stores the cached A binding, binds B, uploads,
   and restores A. The application does not pass A again after deletion.
   Original is expected to emit the diagnostic; #691 should not.
3. **Invalid application handle:** delete A, explicitly bind live B, then explicitly
   bind deleted A. Both variants are expected to emit the diagnostic. This is a
   deliberate misuse control, not a proposed valid rendering sequence.

After each sequence, update/draw live B and check that the center pixel is green,
the GL error is zero, and execution reaches RESULT. The second update makes the
stale restore occur twice where applicable; two messages are not two distinct bugs.
In the invalid-handle case, explicit reuse reintroduces A into the cache even with
#691, causing the second message during the final update.

## Results

Three complete rounds; 12 cases per round (three scenarios, two library variants,
two WebGL versions), 36 completed instances. Each version and round gave:

| Scenario | Original: deleted-binding messages | #691: messages |
| --- | ---: | ---: |
| A remains alive | 0 | 0 |
| Delete A, update only live B | 2 | 0 |
| Application explicitly reuses deleted A | 2 | 2 |

All instances continued, produced the final green pixel [0,255,0,255], and reported
GL error 0. **No fatal WASM abort or browser crash occurred on this machine.**
PASS in the test UI means the expected diagnostic/control outcome occurred; it
does not mean the unpatched library has no bug.

Raw logs in this directory:

- browser-depth-1792101781432547000.json
- browser-depth-1792101813786823000.json
- browser-depth-1792101831552180000.json
- results/build-{baseline,fixed}-{1,2}-{cache,live,invalid}.log records all 12 original builds.
  Local absolute workspace prefixes are replaced with <TEST_ROOT>. Browser log
  contents are unchanged; a final newline was added during packaging.

Packaging validation: all 12 configurations rebuilt successfully with the
committed lockfiles. The browser results above are from the original three runs,
not an additional browser run of the packaged build.

## Why the linkage is credible, and what remains unknown

Source inspection matches the observed cache-only sequence: delete_texture leaves
the cached texture ID; Texture::update_texture_part uses store/bind/restore;
restore_texture_binding binds the recorded deleted ID. PR #691 removes that ID
from cached and stored bindings. This connects the mechanism to the previously
tested atlas cache bug, without needing the atlas or Macroquad to trigger it.

The current JS validator uses console.error for a deleted ID; it does not itself
throw or abort. Our execution confirms continuation on this browser. That does
not disprove a mobile failure elsewhere, but the report's message is not by itself
proof of a fatal driver error.

The original report names Macroquad 0.4.15 / Miniquad 0.4.10 and mobile browsers.
Its example leaves Assets undefined and keeps the assets alive while switching
which one is drawn; it does not itself show the claimed drop/reload. This test
therefore uses a minimal explicit Miniquad lifetime sequence rather than claiming
to run that incomplete original example unchanged.

Not verified: iOS Safari, Android Chrome, a real Macroquad scene-transition crash,
the reported older-version combination, or whether the original reporter's game
hits this exact cache path. A reproducer from that game or mobile execution is
needed before saying #633 is fully resolved by #691.

## Reproduce

Requirements: Git, patch, tar, Python 3, Rust/Cargo and the wasm32-unknown-unknown
target. Network access is needed to fetch the pinned source and dependencies.
From this repository's root:

```sh
rustup target add wasm32-unknown-unknown
python3 texture-633/prepare.py
python3 texture-633/build.py
python3 depth/serve.py texture-633
```

Open http://127.0.0.1:8765 and click Run texture tests. Repeat after completion to
record another round. Stop the server after testing. A new browser-depth-*.json
file is saved for each round. Generated sources, binaries and new logs are ignored
by Git; the published original logs remain tracked.

To prepare without fetching upstream, pass an existing checkout containing the
pinned commit: `python3 texture-633/prepare.py --source /path/to/miniquad`.
The checkout is read only. Preparation refuses to overwrite existing source copies.
The build uses committed lockfiles. The browser bridge is copied unchanged from
the pinned upstream source. This runner is desktop-loopback only; mobile testing
requires a separately arranged reachable host and is not claimed here.
