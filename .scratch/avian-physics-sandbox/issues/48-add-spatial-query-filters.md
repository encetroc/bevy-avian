# 48: Add spatial-query filters

**What to build:** Add query controls for including and excluding collision layers.

**Blocked by:** 39, 42

**Status:** ready-for-agent

## Acceptance criteria

- [ ] The UI exposes the required layer filters.
- [ ] Raycasts and shape casts respect the selected filters.

## Tests

- [ ] Toggle one layer at a time and repeat a query.
- [ ] Verify excluded objects cannot become the reported hit.
