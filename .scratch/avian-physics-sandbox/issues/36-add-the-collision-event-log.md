# 36: Add the collision event log

**What to build:** Display entity enter, contact, and exit events in a clear on-screen log with a clear action.

**Blocked by:** 06

**Status:** complete

## Acceptance criteria

- [x] The log records collision event types and participating entities.
- [x] The log can be cleared without affecting simulation.

## Tests

- [x] Drop an object onto the floor and inspect the event sequence.
- [x] Clear the log and verify new events still appear.
