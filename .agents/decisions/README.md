# Decisions (Architecture Decision Records)

One numbered document per architecture decision: why the code is shaped the way it is, and which
approaches were deliberately rejected. Read the ADRs before proposing a change to an area they
cover; the index of all ADRs is the **Decisions** table in [`../README.md`](../README.md#decisions).

## Format

Follow the [RoomleCore pattern](https://github.com/roomle-internal/RoomleCore/blob/master/documentation/decisions/README.md):

```
# NNNN — Title

Status: Proposed | Accepted | Superseded by NNNN
Date: YYYY-MM-DD

## Context

## Decision

## Alternatives

## Consequences
```

- **Numbering** — four digits, next free number (`0002-…`), kebab-case slug.
- **Status** — `Proposed` while under discussion, `Accepted` once decided; a replaced decision is
  marked `Superseded by NNNN` and stays for the record.
- **Living reference** — an ADR describes why the code is shaped this way _now_; it is updated
  when a later ADR supersedes it, never rewritten to hide the earlier decision.

## Current Documents

| ADR                                                                                                                        | Status   |
| -------------------------------------------------------------------------------------------------------------------------- | -------- |
| [0001 — GeometryModel is the single mesh-conversion contract](0001-geometry-model-as-the-mesh-conversion-contract.md)      | Accepted |
| [0002 — Model transformations: AnyModel enum, bridge exposure, and design decisions](0002-model-transformations-design.md) | Accepted |
