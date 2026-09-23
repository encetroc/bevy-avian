# 48: Add spatial-query filters

**What to build:** Add query controls for including and excluding collision layers.

**Blocked by:** 39, 42

**Status:** complete

## Acceptance criteria

- [x] The UI exposes the required layer filters.
- [x] Raycasts and shape casts respect the selected filters.

## Tests

- [x] Toggle one layer at a time and repeat a query.
- [x] Verify excluded objects cannot become the reported hit.
