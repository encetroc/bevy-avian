# 63: Add current-station and full-sandbox reset controls

**What to build:** Implement R for current-station reset and Shift+R for full-sandbox reset.

**Blocked by:** 07, 61, 62

**Status:** completed

## Acceptance criteria

- [x] R resets only the current station.
- [x] Shift+R resets the complete sandbox without restarting the application.

## Tests

- [x] Disturb multiple stations and invoke R.
- [x] Invoke Shift+R and verify all resettable state returns to baseline.
