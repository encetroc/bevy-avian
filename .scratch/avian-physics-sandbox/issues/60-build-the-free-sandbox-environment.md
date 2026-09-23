# 60: Build the free-sandbox environment

**What to build:** Add floors, ramps, stairs, walls, shelves, tables, and platforms to the free area.

**Blocked by:** 03, 06

**Status:** complete

## Acceptance criteria

- [x] Floors, ramps, stairs, walls, shelves, tables, and platforms are present with static collision; headless tests verify the player can cross the expanded course and dynamic objects land on the ramp, stairs, platform, table, and upper shelf.
- [x] The expanded area provides open space for unrestricted object experimentation.

## Tests

- [x] Headless player movement test crosses the full free-sandbox course without a visible window.
- [x] Headless physics tests drop objects onto the ramp, stairs, platform, table, and upper shelf; all course structures are verified as static colliders.
