# 07: Add station reset infrastructure

**What to build:** Give stations a resettable initial state for their objects and controls.

**Blocked by:** 03

**Status:** ready-for-agent

## Acceptance criteria

- [ ] A station can restore its initial object set and transforms.
- [ ] Resetting one station does not reset unrelated stations.

## Tests

- [ ] Disturb a station, reset it, and compare against its initial layout.
- [ ] Verify another station remains unchanged.
