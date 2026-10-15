# miniquad atlas repro

A minimal reproduction of
[not-fl3/macroquad#1006](https://github.com/not-fl3/macroquad/issues/1006),
"Text rendering corrupted – rendering as black rects", and its cause in
miniquad 0.4.11.

## The bug

In `miniquad` 0.4.11, `GlContext::delete_texture` (`src/graphics/gl.rs`)
deletes the GL texture but does not clear miniquad's texture-binding cache;
the line that would do it is commented out:

```rust
fn delete_texture(&mut self, texture: TextureId) {
    //self.cache.clear_texture_bindings();
    ...
    glDeleteTextures(1, raw as *const _);
```

The line was added in miniquad commit
[f08f001](https://github.com/not-fl3/miniquad/commit/f08f001) ("graphics/gl:
Fix glBindTexture with already deleted textures bug", 2024-05-30) and
commented out four days later, without a stated reason, in
[65f1da6](https://github.com/not-fl3/miniquad/commit/65f1da6) ("native/wasm:
optional WebGL2 support"). It is the same in miniquad master today.

macroquad's font atlas starts at 512 × 512. When it is full, macroquad
deletes the atlas texture and creates a bigger one (`Atlas::texture` in
`src/text/atlas.rs`). If the atlas texture is the cached binding of texture
unit 0 at that moment – which it is right after a flush that ended with
text, for example when switching away from a render-target camera – then:

1. `glDeleteTextures` unbinds it in GL, but the cache still says it is bound.
2. GL hands the new texture the same name (the driver here reuses names).
3. `bind_texture` compares with the stale cache entry, thinks the texture is
   already bound and skips `glBindTexture`.
4. The new atlas's pixels are uploaded to whatever is really bound (nothing)
   instead of the new texture, which is left without any image.
5. Sampling a texture without an image gives opaque black, so every glyph
   is drawn as a black rectangle, regardless of the text colour. It stays
   that way until the atlas happens to be uploaded again.

At the start of each frame miniquad's `commit_frame` clears the cache, so a
growth that happens before the first flush of a frame is harmless. That is
why the bug looks random: it depends on where in a frame the atlas grows.

## The reproduction

`src/main.rs` (about 100 lines) does, every frame:

1. draws text into a 1080 × 1920 render target and reads it back;
2. draws text into the window;

with font sizes that change every frame, so new glyphs keep filling the
atlas. It exits with code 1 and prints `RESULT: black text …` when the
render target's text has no white pixels left. Which frame makes the atlas
grow depends on the order in which the atlas re-packs its glyphs (a
`HashMap`, randomly seeded), so some runs fail and some do not.

```bash
cargo run --release              # one run
./scripts/run.sh 10              # 10 runs, counts the failures
./scripts/run.sh 10 --rect       # with the workaround below
./scripts/with-fix.sh 10         # against miniquad with the line restored
```

`with-fix.sh` copies miniquad 0.4.11 from Cargo's registry into
`./patched`, applies `miniquad-forget-deleted-texture.patch` and builds
against it with a `[patch]`; nothing outside this folder is changed.

Runs are slow (tens of seconds) once the atlas is large: macroquad uploads
the whole atlas, up to 8192 × 8192, for every new glyph.

## Results

Linux 7.0, Mesa 26.0.8, Intel UHD Graphics 630 (OpenGL 4.6), macroquad
0.4.16 and miniquad 0.4.11 (the latest releases), 100 frames per run:

| Build | Runs with black text |
| --- | --- |
| unchanged | 6 of 10 |
| `--rect` workaround | 1 of 10 (and 0 of 15 in a second batch) |
| miniquad with `clear_texture_bindings()` restored | 0 of 10 (and 0 of 12 in a second batch) |
| miniquad with `miniquad-forget-deleted-texture.patch` (`./scripts/with-fix.sh`) | 0 of 10 (and 0 of 12 instrumented runs) |

With a temporarily instrumented macroquad (logging in `Atlas::texture`;
not part of this repository), every one of 8 runs showed the pattern above:

- the atlas was recreated 2–3 times per run, and the new texture always got
  the same GL name as the old one (4);
- the text turned black **exactly** in the runs where the atlas was deleted
  while GL had the atlas itself bound (5 of 8 runs), and in none of the
  others;
- with the miniquad line restored, 0 of 12 runs failed although 5 of them
  deleted the atlas while it was bound;
- with `miniquad-forget-deleted-texture.patch`, 0 of 12 runs failed
  although 3 of them deleted the atlas while it was bound.

These results are from this one machine. They show that the fix removes the
failure reproduced here; other reports in macroquad#1006 (other GPUs and
systems) have not been verified with it.

## Workarounds

Without changing miniquad:

- **End every batch with a draw that does not use the atlas** before it is
  flushed (camera change, end of frame), e.g. a 1 × 1 transparent
  rectangle, so the atlas is not the cached binding when it grows. This is
  what `--rect` does. It makes the failure much rarer but did **not**
  remove it: 1 of 25 runs still drew black text, and that case could not be
  caught with logging (15 instrumented runs all passed), so its path is not
  known.
- Fill the atlas at startup (as suggested in the issue), so it does not grow
  later. This only works for the text sizes cached in advance.

## Suggested fix

Proposed upstream in
[not-fl3/miniquad#691](https://github.com/not-fl3/miniquad/pull/691).

`miniquad-forget-deleted-texture.patch`: after `glDeleteTextures`,
`delete_texture` calls a new `GlCache::forget_texture`, which sets every
cache entry that records the deleted texture as bound (and the stored
binding used by `store`/`restore_texture_binding`) to 0. GL itself rebinds
those units to 0 when a texture is deleted, so this only makes the cache
match GL, with no GL calls. That is narrower than restoring
`clear_texture_bindings()`, which binds 0 on every unit, including units
whose textures are still in use – a possible reason it was commented out.

## License

MIT.
