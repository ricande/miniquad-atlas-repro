# Why this implementation

## Separate testing from writing

#670 occurs because the original GL backend treats `depth_write=false` as a reason
to disable depth testing. The proposed fix enables testing when comparison is not
Always or writing is requested, then sets the write mask independently. Always
with writes enabled needs testing enabled with GL_ALWAYS; disabling testing would
also prevent writes.

## Apply offset and clear it when absent

#597 occurs because the GL backend ignores depth_write_offset. Some(factor, units)
enables GL_POLYGON_OFFSET_FILL and sets its values; None disables it. State from a
previous pipeline must not silently leak into the next pipeline.

## Clear depth independently, then restore actual mask

glClear respects the depth write mask. Once read-only pipelines genuinely disable
writes, an ordinary clear could otherwise stop clearing depth. For depth=Some,
the implementation reads the actual mask, enables writes for the clear and restores
the original mask. A color-only clear does not touch it. This also handles a mask
changed by direct GL calls and supports drawing after clear without reapplying a
pipeline. Use depth=None when the clear must leave depth untouched.

## Why there is no new depth-state cache

We tried caching these states to avoid redundant driver calls and the clear-time
query. This was withdrawn: external GL code can change actual state without
updating Miniquad's cache, after which reapplying the same pipeline may incorrectly
skip required calls. Direct GL integration is not inherently a workaround or a bug.

A safe cached design would require a supported ownership/invalidation boundary;
arbitrary external calls cannot be detected automatically. Introducing such a new
contract is not necessary to fix these two issues. The current patch explicitly
applies pipeline state and reads actual clear-time mask instead. No compatibility
switch, fallback mode or extra ownership API is introduced.

The abandoned cache reduced certain individual-operation microbenchmark medians
by roughly 4–11% on one M2, with variable measurements. There was no demonstrated
whole-rendering or FPS improvement. That experiment is not this proposal and its
performance numbers must not be attributed to the current patch. Correct behavior
with external GL state changes takes precedence over that unproven application gain.

## Intended behavioral changes versus unmodified upstream

- A pipeline's declared write mask replaces a manually set mask when applied.
- A pipeline with no offset disables a manually enabled polygon offset.
- Direct glClear after a read-only pipeline needs its caller to enable depth writes;
  otherwise OpenGL does not clear depth. Miniquad's clear handles this itself.
- An external false write mask no longer suppresses Miniquad's requested depth clear;
  the actual mask is restored afterwards.
- Existing workarounds for ignored offsets or disabled depth testing may need removal.

Manual overrides made after apply_pipeline still affect draws as ordinary GL calls.
Reapplying the pipeline explicitly sets these depth/offset states again. No guarantee
is made about unrelated state caches already present in Miniquad.

## Browser deployment

The patch includes glDepthMask and glPolygonOffset in js/gl.js plus the depth-mask
query constant in native and WASM bindings. Updating Rust alone while shipping an
old JavaScript bundle is insufficient. Macroquad distributions must update their
bundle/imports too. This package does not claim to rebuild or publish that bundle.
