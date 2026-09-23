# 70: Experiment with sandbox serialization (optional)

**What to build:** Save and reload object type, transform, material, and gameplay properties.

**Blocked by:** 61, 64, 67

**Status:** complete

## Acceptance criteria

- [x] A sandbox snapshot can be written and loaded.
- [x] Reloaded objects preserve the supported serialized properties.

## Tests

- [x] Save a mixed sandbox, alter it, and reload.
- [x] Compare transforms, materials, and container amounts before and after.
