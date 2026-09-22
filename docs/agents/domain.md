# Domain documentation

This repository uses a single-context domain documentation layout.

## Layout

- `CONTEXT.md` at the repository root contains the domain model and terminology.
- `docs/adr/` contains architecture decision records.

## Consumer rules

Before making domain-level changes, read `CONTEXT.md` when it exists. Read relevant ADRs in `docs/adr/` before changing an area governed by an existing decision. Keep domain terminology consistent with `CONTEXT.md`, and record material architectural changes as new ADRs rather than rewriting history.

These files may not exist yet; create them when the repository has domain decisions to document.
