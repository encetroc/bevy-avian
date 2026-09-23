# 38: Add contact inspection

**What to build:** Show contact partner, position, normal, relative speed, and impulse for a selected colliding object.

**Blocked by:** 36, 13

**Status:** complete

## Acceptance criteria

- [x] Contact data updates while the collision is active.
- [x] The inspector identifies the other entity.

## Tests

- [x] Headless test selects an object resting against a named floor and reports its contact.
- [x] Headless test verifies contact position, normal, relative speed, impulse, and live refresh.
