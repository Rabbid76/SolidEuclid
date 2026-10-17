# Digital Brain Index

## Purpose

SolidEuclid is a high-performance, cross-platform geometric modeling kernel written in Rust. It
generates precise 3D primitives (platonic solids, spheres), sweep bodies (torus, capsule, cylinder,
conical frustum) and extrusions from planar profiles, and performs the mesh conversion every
consumer needs. The kernel is exposed through two bridges: a C bridge for C++ (`c_api.rs`,
`include/solid_euclid.h`) and a WebAssembly bridge for TypeScript/JavaScript (`web_api.rs`, built
into `pkg/` by wasm-pack). Every geometry type funnels through one contract — the `GeometryModel`
trait — which converts a model into a polygon mesh and, by default, into a triangular mesh.

Every markdown document in this folder, in one place. Add a line here whenever you add a document —
see **Where Documentation Goes** in [`../AGENTS.md`](../AGENTS.md#where-documentation-goes).

The split below matters when you are using these documents as context:

- **Living reference** — describes how the code works _now_. Keep it accurate; update it when the
  behaviour it describes changes.
- **Historical records** — point-in-time write-ups (a bug as it was analysed, a feature as it was
  studied, a refactoring as it was carried out). They are deliberately _not_ updated. **Treat them
  as evidence of what was true on the date shown, not as a description of current behaviour** —
  always confirm against the code before acting on one.

Decisions about _why_ the code is shaped the way it is live in `.agents/decisions/` (created with
the first ADR).

---

## Decisions

Architecture Decision Records — why the code is shaped the way it is, and which approaches were
deliberately rejected. Read these before proposing a change to an area they cover. Create numbered
ADR documents in `.agents/decisions/` as needed, following the
[RoomleCore pattern](https://github.com/roomle-internal/RoomleCore/blob/master/documentation/decisions/README.md).

| ADR                                                                                                                                  | Status   |
| ------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| [0001 — GeometryModel is the single mesh-conversion contract](decisions/0001-geometry-model-as-the-mesh-conversion-contract.md)      | Accepted |
| [0002 — Model transformations: AnyModel enum, bridge exposure, and design decisions](decisions/0002-model-transformations-design.md) | Accepted |

---

## Living Reference

### Skills (On-Demand Domain Knowledge)

Skills provide deep domain knowledge for AI agents. Load them when the task matches their domain.

| Skill                                                                     | Description                                                                                                                   | Load When                                                                    |
| ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| [solid-euclid-geometry-model.md](./skills/solid-euclid-geometry-model.md) | The `GeometryModel` trait, the polygon-to-triangular mesh pipeline, mesh and material conventions, how to add a geometry type | Any change to the Rust core, a new geometry type, mesh or material behaviour |
| [solid-euclid-web-api.md](./skills/solid-euclid-web-api.md)               | The WASM web API: wasm-bindgen ABI, `pkg/` build, the example pages, vitest tests, adding a web API function                  | Web API, `examples/web`, wasm-pack, vitest                                   |
| [solid-euclid-c-api.md](./skills/solid-euclid-c-api.md)                   | The C bridge: `extern "C"` ABI, ownership and free functions, the header, CMake build, doctest tests, adding a C API function | C API, `include/solid_euclid.h`, `examples/cpp`, CMake, doctest              |
| [solid-euclid-pr-resolution.md](./skills/solid-euclid-pr-resolution.md)   | Resolving a pull request: verifying suggested changes, applying them, replying to every review comment, resolving threads     | Resolving a PR; never merge it                                               |

### Scripts

| Script                                                       | Description                                                                                             |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| [check-markdown-links.js](./scripts/check-markdown-links.js) | Checks the relative links of markdown files: `node .agents/scripts/check-markdown-links.js <file.md> …` |

### Architecture and Development

High-level living reference for navigating the codebase and running its toolchain.

| Document                                       | Description                                                                                                     |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| [architecture.md](./architecture.md)           | Codebase map: module index, geometry types, mesh pipeline, bridge surfaces, data flow, invariants, dependencies |
| [development-guide.md](./development-guide.md) | Build, test, lint, debug and deployment commands; test suite patterns; project layout; common pitfalls          |

### User-Facing Documentation

Living reference for users and developers is the [README.md](../README.md) and the rustdoc of the
public API (`cargo doc`). Agent-facing domain knowledge lives in the skill files above.

---

## Historical Records

**Point-in-time. May not reflect current behaviour.**

### Bug Analyses

One document per bug: root-cause analysis written **before** the fix, closed out **after** it.

| Document | Status | Last touched |
| -------- | ------ | ------------ |

### Feature Analyses

One document per feature question: a proposed feature, a change to an existing one, or _why_ a
feature behaves the way it does. Written **before** the work, closed out **after** it.

| Document                                                           | Status      | Last touched |
| ------------------------------------------------------------------ | ----------- | ------------ |
| [Model transformations](feature-analysis/model-transformations.md) | Implemented | 2026-10-17   |

### Refactoring Analyses

One document per refactoring, written **before** the work and carrying the report once the work is
done. The analysis and the report are the same document — the report is appended at close-out.

| Document | Status | Last touched |
| -------- | ------ | ------------ |

### Backlog

Outstanding defects, optimizations and follow-ups for SolidEuclid.

| Document                                     | Last touched |
| -------------------------------------------- | ------------ |
| [Backlog](backlog/README.md) — the open work | 2026-10-17   |

---

## Analysis Workflow

### Trigger Phrases

The following phrases trigger the analysis workflow automatically:

- **"analyse the bug"** / **"analyze the bug"** / **"bug analysis"** → Creates document in `.agents/bug-analysis/`
- **"analyse the feature"** / **"analyze the feature"** / **"feature analysis"** → Creates document in `.agents/feature-analysis/`
- **"analyse the refactoring"** / **"analyze the refactoring"** / **"refactoring analysis"** → Creates document in `.agents/refactoring-analysis/`

### Document Lifecycle

1. **Write analysis** — Before any code changes, write the complete analysis document
2. **Do the work** — Implement the fix, feature, or refactoring
3. **Close out** — Update the document with the results, set status, promote durable knowledge
4. **Delete once landed** — when the work is on `main`, promote the durable outcome into the living
   reference and delete the document; git history and the pull request keep the record. A solved
   bug keeps no analysis

### Status Values

| Analysis Type        | Status Values                     |
| -------------------- | --------------------------------- |
| Bug Analysis         | `Open`, `Fixed`                   |
| Feature Analysis     | `Open`, `Implemented`, `Rejected` |
| Refactoring Analysis | `Open`, `Done`                    |

### Naming Convention

Use kebab-case for document filenames:

- `wrong-normal-on-conical-frustum-cap.md`
- `sweep-model-simplification.md`
- `c-api-without-free-function.md`

### Document Structure

Every analysis document should include:

**Bug Analysis:**

- Symptom and reproduction
- Investigation trace with file:line references
- Exact root cause
- Why current code is wrong
- Proposed clean fix

**Feature Analysis:**

- What was asked and why
- How the area works today with code references
- The gap or question answered
- Proposed design or change
- Alternatives considered and rejected
- Code and documents the work would touch

**Refactoring Analysis:**

- What current code does and why it's a problem
- Full scope with file:line references
- Proposed target shape
- Tests covering affected behaviour
- Output changes to expect
- For performance: benchmark to run before/after

---

## Where Documentation Goes

| What you produced                                        | Where it goes                                                                                      | Lifecycle                                                                                                                   |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| **Living reference** — how things work now               | `.agents/skills/`, `.agents/architecture.md`, `.agents/development-guide.md`, `README.md`, rustdoc | Updated when behaviour changes; says what holds today — never what was wrong, when something was solved, or a ticket number |
| **Historical record** — analysis, benchmark, refactoring | `.agents/<analysis-type>/`                                                                         | Never updated after close-out; deleted once the work has landed                                                             |
| **Decision** — why code is shaped this way               | `.agents/decisions/` (create folder if needed)                                                     | Living reference                                                                                                            |
| **Open work**                                            | `.agents/backlog/README.md`                                                                        | Removed when the fix is in the code                                                                                         |

## Adding New Documentation

### For AI Agents (Digital Brain)

1. **Skills**: Add to `.agents/skills/` for on-demand domain knowledge
2. **Bug Analysis**: Add to `.agents/bug-analysis/` before fixing
3. **Feature Analysis**: Add to `.agents/feature-analysis/` before implementing
4. **Refactoring Analysis**: Add to `.agents/refactoring-analysis/` before refactoring
5. **Update this index**: Add a line to the appropriate table

### For End Users

Update `README.md` and the rustdoc of the public items the change touches.

## See Also

- [AGENTS.md](../AGENTS.md) — AI assistant instructions and workflow
- [.github/copilot-instructions.md](../.github/copilot-instructions.md) — GitHub Copilot specific guidance
