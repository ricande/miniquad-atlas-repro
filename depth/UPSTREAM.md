# Upstream status checked 2026-10-15

- Repository: https://github.com/not-fl3/miniquad
- Default branch fetched: master.
- HEAD: [39c928fb7e09fd44e45f37fe9a98503968bca412](https://github.com/not-fl3/miniquad/commit/39c928fb7e09fd44e45f37fe9a98503968bca412).
- [#670](https://github.com/not-fl3/miniquad/issues/670): open.
- [#597](https://github.com/not-fl3/miniquad/issues/597): open.
- Both issue discussions were read, including existing verification comments.
- The patch applies without fuzz to this revision. Runtime comparisons against
  unmodified and patched copies are recorded in README.md and results/upstream.

## Overlapping work search

GitHub PR searches (all states, first 100 results, API reported complete results):

- `repo:not-fl3/miniquad is:pr depth`: 11 results.
- `repo:not-fl3/miniquad is:pr ("670" OR "597" OR "polygon")`: 1 result, closed #87.
- `repo:not-fl3/miniquad is:pr glDepthMask`: 1 result, closed #48.

No direct replacement for this combined fix was identified in these searches.
This is a bounded keyword search, not proof that no overlapping branch exists.
[PR #683](https://github.com/not-fl3/miniquad/pull/683) touches the same GL file for
transient multisampled attachments. Its file patch was inspected: it does not add
the depth-mask/offset pipeline fix. It may still affect adjacent render-pass code
and should be considered when rebasing a future submission. Other depth-related
results largely concern Metal/render targets, not this GL pipeline behavior.

The default branch and issue status may change after this check. Recheck before
submission. No comments, commits, pushes or PRs were created during preparation.
