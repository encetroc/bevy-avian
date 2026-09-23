# 68: Add the prototype evaluation scenarios and observation log

**What to build:** Provide repeatable evaluation scenarios and a developer-facing record of interaction, stacking, composition, performance, and physics-style observations.

**Blocked by:** 35, 51, 56, 58, 65, 66, 67

**Status:** ready-for-human

## Acceptance criteria

- [x] The scenarios cover picking, dragging, throwing, stacking, composition, sleeping, and stress.
- [x] The record captures FPS, physics time, bodies, sleeping bodies, contacts, and qualitative notes; unavailable render-only and unsampled metrics are marked explicitly.

## Tests

- [x] Run each scenario from a clean reset using isolated headless tests.
- [x] Verify spawn/clear at 100, 500, and 1,000 bodies; see `.scratch/avian-physics-sandbox/prototype-evaluation.md` for the observation log and measurement limits.
