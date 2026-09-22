# 73: Add optional physics validation mode

**What to build:** Expose a development-only Avian validation configuration for diagnosing invalid physics setup.

**Blocked by:** 56

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Validation can be enabled without changing normal defaults.
- [ ] Validation output is visible to developers without crashing the sandbox.

## Tests

- [ ] Run the application with validation enabled.
- [ ] Exercise representative stations and confirm diagnostics remain available.
