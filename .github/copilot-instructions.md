# GitHub Copilot Instructions — SolidEuclid

> Open-standard "README for AI" — structured context for GitHub Copilot, Copilot Workspace, coding agents, and LLM-based dev tools.

## Always Do This First

1. **Load matching skills.** Check the [On-Demand Skills](#on-demand-skills) catalog and read every skill file whose domain matches the task _before_ taking action. Multiple skills may apply to one task.
2. **Read the matching skill for the layer you touch.** The Rust core and the mesh pipeline: [`.agents/skills/solid-euclid-geometry-model.md`](../.agents/skills/solid-euclid-geometry-model.md). The WASM web API and `examples/web`: [`.agents/skills/solid-euclid-web-api.md`](../.agents/skills/solid-euclid-web-api.md). The C bridge and `examples/cpp`: [`.agents/skills/solid-euclid-c-api.md`](../.agents/skills/solid-euclid-c-api.md). Read [`README.md`](../README.md) before answering architecture or domain questions.
3. **Read [`.agents/README.md`](../.agents/README.md)** for the complete digital brain index, separating living reference from historical records.
4. **Treat documentation as part of the task, not a follow-up.** Every analysis produces a document, and every change to productive code updates one — see [Where Documentation Goes](#where-documentation-goes).
5. **This file mirrors [AGENTS.md](../AGENTS.md).** AGENTS.md is the canonical source; when it changes, this file changes in the same commit.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:

- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:

- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:

- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:

- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:

```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## Identity

- **Project**: SolidEuclid
- **Repository**: `Rabbid76/SolidEuclid`
- **Default branch**: `main`
- **Language**: Rust (edition 2024) for the kernel; TypeScript for the web examples; C++ (C++23) for the C-bridge examples
- **Purpose**: High-performance, cross-platform geometric modeling kernel — 3D primitives, sweep bodies and extrusions converted to renderable meshes — exposed to C++ through a C bridge and to TypeScript/JavaScript through WebAssembly

## Project Overview

SolidEuclid is a geometric modeling kernel written in Rust. It provides a robust foundation for
generating precise 3D primitives, applying advanced edge treatments, and performing Constructive
Solid Geometry (CSG) operations. Designed with portability in mind, it offers first-class support
for C++ via a C bridge and for TypeScript/JavaScript through WebAssembly.

1. **Rust core** (`solid_euclid/`) — the only place geometry logic lives. Primitives (platonic
   solids, sphere), sweep bodies (torus, capsule, cylinder, conical frustum), extrusions from
   planar profiles and NURBS contours, and the mesh pipeline: `PolygonMesh` (f64, polygons) →
   `TriangularMesh` (f32, per-material triangle soups). Every geometry type implements the
   `GeometryModel` trait.
2. **C bridge** (`solid_euclid/src/c_api.rs` + `include/solid_euclid.h`) — `extern "C"` functions
   that take a model as a JSON string and return a `CTriangularMesh` by value; the caller frees
   every allocation with the matching `*_free` function.
3. **WASM web API** (`solid_euclid/src/web_api.rs` → `pkg/`) — `#[wasm_bindgen]` functions that
   take a model as a `JsValue` and return a `WasmTriangularMesh` with zero-copy typed-array
   getters, built by `npm run build:wasm` (wasm-pack).

The examples demonstrate every geometry type at both bridges: `examples/web/` (vite + three.js,
TypeScript) and `examples/cpp/` (CMake, C++23).

## Repository Structure

```
.
├── solid_euclid/                 # The Rust crate (workspace member, edition 2024)
│   ├── Cargo.toml                # crate-type: cdylib + staticlib + rlib
│   └── src/
│       ├── lib.rs                # Crate root: the module list
│       ├── geometry_model.rs     # The GeometryModel trait: to_polygon_mesh / to_triangular_mesh
│       ├── polygon_mesh.rs       # PolygonMesh: f64 vertices (position/normal/uv) + polygons + materials
│       ├── triangular_mesh.rs    # TriangularMesh: per-material GPU-ready triangle soups (f32)
│       ├── vertex.rs             # Vertex: position, normal, uv
│       ├── material.rs           # Material assignment: Default, Uniform, Map, Array
│       ├── geometry_error.rs     # GeometryError: the crate's error type
│       ├── geometry_hash.rs      # GeometryHash: SHA256 over geometry (rounded to 3 decimals)
│       ├── plane.rs              # Plane
│       ├── planar_contour.rs     # PlanarContour, ContourSet (earcut triangulation path)
│       ├── planar_nurbs_curve.rs # PlanarNurbsCurve
│       ├── planar_profile.rs     # PlanarProfile (extrude outlines)
│       ├── platonic_solid.rs     # Platonic solids (Model)
│       ├── sphere.rs             # Sphere (Model)
│       ├── sweep_model.rs        # SweepModel: a profile swept along a path
│       ├── sweeps.rs             # Torus, Capsule, Cylinder, ConicalFrustum, Extrude
│       ├── web_api.rs            # WASM bindings (wasm-bindgen) -> pkg/
│       └── c_api.rs              # C bridge (extern "C") -> include/solid_euclid.h
├── include/
│   └── solid_euclid.h            # The C header (mirrors c_api.rs)
├── examples/
│   ├── web/                      # TypeScript/three.js example pages (vite)
│   │   ├── __tests__/            # vitest tests of the web API, one file per geometry type
│   │   └── util/three-util.ts    # Shared three.js helpers
│   └── cpp/                      # C++ example and doctest tests (CMake)
├── pkg/                          # wasm-pack output (git-ignored, never edited)
├── CMakeLists.txt                # Builds the Rust staticlib + the C++ examples
├── package.json                  # npm scripts: build:wasm, dev, build, test, test:cpp, test:all, lint, format, tidy-up
├── Cargo.toml                    # Workspace root
├── README.md                     # Project overview
├── AGENTS.md                     # Primary AI assistant instructions (canonical source of this file)
├── CLAUDE.md                     # Redirect to AGENTS.md (for Claude Code)
├── copilot-instructions.md       # Redirect to this file
├── .agents/                      # Digital Brain - AI agent knowledge base
│   ├── README.md                 # Digital brain index
│   ├── backlog/                  # Open work only
│   ├── bug-analysis/             # Bug root-cause analyses
│   ├── decisions/                # Architecture decision records (ADRs)
│   ├── feature-analysis/         # Feature investigations
│   ├── refactoring-analysis/     # Refactoring analyses with reports
│   ├── scripts/                  # Utility scripts (JavaScript only)
│   └── skills/                   # On-demand domain knowledge
├── .github/
│   └── copilot-instructions.md   # This file - GitHub Copilot specific instructions
├── .internal/                    # Private working notes
└── .temp/                        # Scratch space, git-ignored
```

## On-Demand Skills

Skills provide deep domain knowledge. Load them by reading the file when the task matches their domain. Skill files are located in `.agents/skills/`.

**Skills belong in `.agents/skills/`.** Every skill of this repository is a Markdown file there with a row in the table below. **Never create a `.claude/` folder in this repository** — no `.claude/skills/`, no `SKILL.md` wrappers and no other tool-specific copy of a skill.

| Skill                                                                                               | Load when the task involves                                                                                                                   |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| [`.agents/skills/solid-euclid-geometry-model.md`](../.agents/skills/solid-euclid-geometry-model.md) | The `GeometryModel` trait, the polygon-to-triangular mesh pipeline, mesh and material conventions, adding a geometry type in the Rust core    |
| [`.agents/skills/solid-euclid-web-api.md`](../.agents/skills/solid-euclid-web-api.md)               | The WASM web API (`web_api.rs`, `pkg/`), the example pages, vitest tests, adding a web API function                                           |
| [`.agents/skills/solid-euclid-c-api.md`](../.agents/skills/solid-euclid-c-api.md)                   | The C bridge (`c_api.rs`, `include/solid_euclid.h`), ownership and free functions, CMake build, doctest tests, adding a C API function        |
| [`.agents/skills/solid-euclid-pr-resolution.md`](../.agents/skills/solid-euclid-pr-resolution.md)   | Resolving a pull request: verifying suggested changes, applying them, replying to every review comment, resolving threads. Never merge the PR |

## Key Architecture Patterns

### The GeometryModel Contract

1. **One trait, one conversion path** — every geometry type implements `GeometryModel`
   (`solid_euclid/src/geometry_model.rs`): `to_polygon_mesh()` builds a `PolygonMesh` (f64
   vertices with position/normal/uv, polygons with a normal and a geometry id, one material id
   per geometry id); `to_triangular_mesh()` tessellates it by default into a `TriangularMesh`
2. **Meshes are the interchange** — the C bridge and the WASM web API never expose geometry
   internals; they convert through `to_triangular_mesh()` and hand out mesh data only (per-material
   triangle soups of f32 position/normal/uv + indices)
3. **Materials** — `Material` (`Default`, `Uniform`, `Map`, `Array`) decides the material id of
   every face; the triangular mesh is grouped by material id, so each material becomes one GPU
   geometry
4. **Errors** — fallible conversions return `GeometryError` (`InsufficientVertices`,
   `NotYetImplemented`, `InvalidParameter`); the bridges convert it at their edge (a `JsError` in
   the web API, an error mesh in the C API)
5. **Precision** — f64 (`glam`) inside the kernel, f32 at the mesh boundary

### The Three API Layers

1. **Rust core** (`solid_euclid/src/`) — the only place geometry logic lives; unit tests beside
   the code (`#[cfg(test)] mod tests`)
2. **C bridge** (`c_api.rs` + `include/solid_euclid.h`) — `extern "C"` functions taking JSON in
   (`*const c_char`) and returning `CTriangularMesh` by value; every allocation has a matching
   `*_free` function owned by the caller; C++ example and doctest tests in `examples/cpp/`
3. **WASM web API** (`web_api.rs` → `pkg/` via wasm-pack) — `#[wasm_bindgen]` functions taking
   `JsValue` in and returning `WasmTriangularMesh` with zero-copy typed-array getters; TypeScript
   example pages in `examples/web/` (vite + three.js), vitest tests in `examples/web/__tests__/`;
   `pkg/` is git-ignored and never edited

### The Pattern Every Geometry Type Follows

Platonic solid → sphere → torus → capsule → cylinder → conical frustum → extrude: a model struct in
the core with `impl GeometryModel`, a `create_*` function in `web_api.rs`, a `create_*_c` function
in `c_api.rs` with a header declaration, a vitest file, a doctest file, and an example page.

## Hard Rules

### Code Style

- **Rust** — `cargo fmt` formats the crate; `npm run tidy-up` runs the full pass (prettier, eslint
  --fix, `cargo fix`, `cargo fmt`, clang-format). Match the existing style; keep public items
  documented with rustdoc (`///`)
- **TypeScript** — Prettier (`npm run format`, `npm run format:check`; settings in `.prettierrc`,
  excluded paths in `.prettierignore`) and ESLint (`npm run lint`, `npm run lint:fix`; settings in
  `eslint.config.js`)
- **C++** — clang-format with the repo's `.clang-format` (`npm run format:cpp`); `.clang-tidy` for
  static analysis
- **No new dependencies without reason** — the crate's dependencies are locked in `Cargo.lock`,
  the web tooling in `package-lock.json`; do not add to either without need
- **Error handling** — return `GeometryError` from the core and propagate it through `Result`;
  never panic on caller-controlled input

### Naming Conventions

- **Rust** — files and directories `snake_case` (`geometry_model.rs`); functions and methods
  `snake_case`; types (structs, enums, traits) `PascalCase`; constants `SCREAMING_SNAKE_CASE`;
  macros `snake_case!`
- **TypeScript** — filenames `kebab-case` (`platonic-solid.test.ts`); variables and functions
  `camelCase`; classes and interfaces `PascalCase`; constants `SCREAMING_SNAKE_CASE`
- **C++** — filenames `kebab-case` (`test-platonic-solid.cpp`); types (classes, structs)
  `PascalCase`; variables, functions and methods `snake_case`; constants `SCREAMING_SNAKE_CASE`

### Comments

> "Every time you write a comment, you should grimace and feel the failure of your ability of expression." — Robert C. Martin (Uncle Bob)

- **Minimal comments** — Code must express its intent through naming and structure
- **Rustdoc for public APIs** — public items carry `///` doc comments (the bridges' JSON contract and ownership rules included)
- **No header comments** — No file-level author/license headers
- **No section dividers** — No `// ===` or `// ---` lines in Rust; the C++ code uses them sparingly where it already does

## Where Documentation Goes

Documentation lives in markdown files under `.agents/`, in `README.md`, and in rustdoc comments — never only in code comments.

### Digital Brain Structure

The `.agents/` folder serves as the "digital brain" for SolidEuclid, containing:

```
.agents/
├── README.md                     # Digital brain index
├── backlog/                      # Open work only
├── bug-analysis/                 # Bug root-cause analyses (written before fix)
├── decisions/                    # Architecture decision records (ADRs)
├── feature-analysis/             # Feature investigations and decisions
├── refactoring-analysis/         # Refactoring analyses with reports
├── scripts/                      # Utility scripts (JavaScript only)
└── skills/                       # On-demand domain knowledge (skills)
```

**IMPORTANT**: All scripts in `.agents/scripts/` MUST be JavaScript (Node.js), not Python, Rust or shell. This keeps the agent tooling consistent with the repository's npm-based tooling.

### Analysis Documents

Every change to productive code should be accompanied by analysis documents:

| Analysis Type        | Folder                          | When to Create                                                               | Status Values               |
| -------------------- | ------------------------------- | ---------------------------------------------------------------------------- | --------------------------- |
| Bug Analysis         | `.agents/bug-analysis/`         | "analyse the bug", "analyze the bug", "bug analysis"                         | Open, Fixed                 |
| Feature Analysis     | `.agents/feature-analysis/`     | "analyse the feature", "analyze the feature", "feature analysis"             | Open, Implemented, Rejected |
| Refactoring Analysis | `.agents/refactoring-analysis/` | "analyse the refactoring", "analyze the refactoring", "refactoring analysis" | Open, Done                  |

Each analysis document follows the same lifecycle: written **before** the work, closed out **after** the work.

**Every analysis names the affected layers first.** A bug, feature or refactoring analysis starts, right after its header block, with a short list of the layers the work changes, one line each saying what changes there: the Rust core (`solid_euclid`), the C bridge (`c_api.rs`, `include/solid_euclid.h`), the web API (`web_api.rs`, `pkg`), the web examples (`examples/web`), the C++ examples (`examples/cpp`). A layer the work leaves unchanged is named only where a reader would expect a change.

| What you produced                  | Where it goes                                                                                 |
| ---------------------------------- | --------------------------------------------------------------------------------------------- |
| Bug analysis                       | new file in `.agents/bug-analysis/` with kebab-case slug                                      |
| Feature analysis                   | new file in `.agents/feature-analysis/` with kebab-case slug                                  |
| Refactoring analysis               | new file in `.agents/refactoring-analysis/` with kebab-case slug                              |
| Benchmark and performance analysis | new file in `.agents/benchmarks/` with kebab-case slug (create the folder with the first one) |
| New feature capability             | `README.md`, or the rustdoc of the public items, plus the matching skill file                 |
| Architecture decisions             | Create a numbered ADR in `.agents/decisions/`                                                 |
| Open work                          | `.agents/backlog/README.md`                                                                   |
| Living reference                   | `README.md` and rustdoc for users, `.agents/skills/` for agent-facing domain knowledge        |

**Important**: Analysis documents in `.agents/bug-analysis/`, `.agents/feature-analysis/`, and `.agents/refactoring-analysis/` are **historical records** — they document the analysis process and decisions, but the durable knowledge should be promoted to living-reference documentation in the skill files or the rustdoc.

**The living reference describes today.** Everything outside the analyses and the backlog — `.agents/skills/`, the READMEs, the rustdoc — says how the code works now:

- When an analysis is deleted, its valuable content goes there first: the invariants, the decisions and the rejected approaches someone could re-propose, the guarding tests, and how the geometry behaves as the investigation found it.
- It never says what was wrong in the past: no defect, no earlier behaviour, no "found in" or "found with", no "previously", "no longer" or "was fixed".
- It does not record when something was solved, fixed, verified or confirmed.
- It carries no ticket numbers. They belong to the analyses, the backlog items and the pull requests.

**A solved bug keeps no analysis.** Once its fix has landed on `main`, the valuable content is promoted and the document deleted, and the bug's item leaves the backlog.

### Analysis Workflow

When investigating issues, follow this workflow:

#### Bug Analysis ("analyse the bug")

1. **Reproduce** — Identify exact steps to trigger the issue
2. **Investigate** — Trace data flow, identify root cause with file:line references
3. **Document** — Create analysis in `.agents/bug-analysis/<slug>.md`
4. **Fix** — Implement clean fix addressing root cause
5. **Validate** — Test fix thoroughly
6. **Close out** — Update analysis with results, promote durable knowledge

#### Feature Analysis ("analyse the feature")

1. **Question** — Clearly state what needs investigation
2. **Current State** — Document how it works today with code references
3. **Gap/Question** — Identify what's missing or unclear
4. **Proposed** — Design or change proposal
5. **Alternatives** — Considered and rejected approaches
6. **Document** — Create analysis in `.agents/feature-analysis/<slug>.md`

#### Refactoring Analysis ("analyse the refactoring")

1. **Current** — What the code does and why it's a problem
2. **Scope** — Full scope with file:line references
3. **Target** — Proposed target shape
4. **Tests** — Coverage of affected behaviour
5. **Benchmark** — Performance measurements if applicable
6. **Document** — Create analysis in `.agents/refactoring-analysis/<slug>.md`

### Documentation Structure

For AI agents:

- **[AGENTS.md](../AGENTS.md)** — Primary AI assistant instructions (canonical source of this file)
- **[.agents/README.md](../.agents/README.md)** — Digital brain index
- **[.agents/skills/](../.agents/skills/)** — On-demand domain knowledge
- **[.agents/bug-analysis/](../.agents/bug-analysis/)** — Bug analyses
- **[.agents/feature-analysis/](../.agents/feature-analysis/)** — Feature investigations
- **[.agents/refactoring-analysis/](../.agents/refactoring-analysis/)** — Refactoring analyses
- **[.agents/decisions/](../.agents/decisions/)** — Architecture decision records

For humans:

- **[README.md](../README.md)** — Project overview
- The rustdoc of the public API (`cargo doc`)

## Development Workflow

### Checks Before Committing

```bash
cargo test                  # Rust unit tests of the core
npm test                    # wasm-pack build + vitest (the web API)
npm run test:cpp            # CMake build + doctest (the C API)
npm run test:all            # all three suites
npm run lint                # ESLint (TypeScript)
npm run format:check        # Prettier
cargo fmt --check           # rustfmt
node .agents/scripts/check-markdown-links.js <changed .md files>
```

`npm run tidy-up` runs the full format/lint pass (prettier, eslint --fix, `cargo fix`, `cargo fmt`, clang-format) — run it before committing a larger change.

### Running the Web Example

```bash
npm install
npm run dev                 # wasm-pack build + vite dev server for examples/web, browser opens
npm run build               # wasm-pack build + tsc + vite build into dist/
npm run preview             # serve dist/
```

### Building the C++ Example

```bash
cmake -B build              # configure (builds the Rust staticlib via CMake)
npm run test:cpp            # build the doctest target and run it
```

### Common Workflows

**Adding a geometry type (the full pattern):**

```
1. solid_euclid/src/<name>.rs — model struct + impl GeometryModel + unit tests, registered in lib.rs
2. web_api.rs — create_<name>(JsValue), a vitest file, an example page registered in vite.config.ts
3. c_api.rs + include/solid_euclid.h — create_<name>_c(*const c_char) + doctest file
4. npm run test:all — every layer green
```

**Debugging wrong geometry:**

```
1. Reproduce in a Rust unit test at the layer where the wrongness starts
2. Trace the mesh pipeline: PolygonMesh (winding, normals, materials) -> TriangularMesh (tessellation, grouping)
3. Fix the assumption in the algorithm, not the output mesh
```

## Adding New Geometry

The pattern every primitive follows (platonic solid → sphere → torus → capsule → cylinder → conical frustum → extrude):

1. **Rust core** — a new `snake_case` module in `solid_euclid/src/` (registered in `lib.rs`), a serde-serializable model struct, `impl GeometryModel` building the `PolygonMesh` in `to_polygon_mesh()`, and unit tests beside the module — see [`.agents/skills/solid-euclid-geometry-model.md`](../.agents/skills/solid-euclid-geometry-model.md)
2. **Web API** — `create_<name>(JsValue) -> Result<WasmTriangularMesh, JsError>` in `web_api.rs`, a vitest file in `examples/web/__tests__/`, and an example page in `examples/web/<name>/` registered in `vite.config.ts` — see [`.agents/skills/solid-euclid-web-api.md`](../.agents/skills/solid-euclid-web-api.md)
3. **C API** — `create_<name>_c(*const c_char) -> CTriangularMesh` in `c_api.rs`, the declaration in `include/solid_euclid.h` with the ownership doc, doctest cases in `examples/cpp/tests/` (registered in `examples/cpp/CMakeLists.txt`) — see [`.agents/skills/solid-euclid-c-api.md`](../.agents/skills/solid-euclid-c-api.md)
4. **Verify** — `npm run test:all`, then the lint/format checks above

## Suggested Change Workflow

This workflow is a suggestion and a guideline, not a requirement. It describes how a feature, a bug
fix, or a refactoring moves from an idea to a merged pull request when a human and an agent work on
it together. Small changes may skip steps; large or risky changes benefit from every one of them.
The steps alternate between agent work and human review, and every step builds on the reviewed
result of the step before it.

| #   | Step                                                                 | Who                                                    | Result                                                                                                                                                                                            |
| --- | -------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Describe the feature, bug, or refactoring and its definition of done | human, supported by the agent                          | a clear problem statement and acceptance criteria                                                                                                                                                 |
| 2   | Analyse the feature, bug, or refactoring                             | agent                                                  | the analysis document in the matching folder — see [Analysis Documents](#analysis-documents)                                                                                                      |
| 3   | Review the analysis                                                  | human                                                  | the root cause, the gap, or the scope is confirmed                                                                                                                                                |
| 4   | Plan the implementation and the unit tests                           | agent                                                  | the implementation plan                                                                                                                                                                           |
| 5   | Review the plan                                                      | human                                                  | the approach is approved before any code changes                                                                                                                                                  |
| 6   | Implement the plan                                                   | agent                                                  | code, tests, and documentation on a purpose branch, opened as a pull request only when the human asks for it — see [Hard Rule: NO Automatic Pull Requests](#hard-rule-no-automatic-pull-requests) |
| 7   | Review the code                                                      | human                                                  | review comments on the pull request                                                                                                                                                               |
| 8   | Apply the review suggestions                                         | agent                                                  | new commits, every review thread replied to or resolved — see [Pull Request Resolution](#pull-request-resolution)                                                                                 |
| 9   | Review the code again                                                | agent, preferably a different one than the implementer | review comments on the pull request                                                                                                                                                               |
| 10  | Apply the review suggestions                                         | agent or human                                         | new commits, every review thread replied to or resolved                                                                                                                                           |
| 11  | Final review and merge                                               | human                                                  | the merged pull request — an agent never merges                                                                                                                                                   |

**Every review gates the next step.** Each step trusts the reviewed result of the step before it:
the plan builds on the analysis, the implementation on the plan, the code review on the
implementation. Every review therefore has to be done very carefully — a flaw that passes a review
is carried into every later step.

**A review can send the work back.** If a review identifies a fundamental problem — the analysis
missed the real root cause, the plan does not cover the definition of done, the implementation
contradicts the plan — the work goes back one or more steps instead of being patched at the current
step.

## Git Workflow Rules

### Hard Rule: NO Force Pushes

**NEVER use `git push --force` or `git push -f` on any branch, especially main.** Force pushing rewrites history and is absolutely forbidden. If you need to undo commits, create a new revert commit instead.

### Hard Rule: NO Automatic Pull Requests

**NEVER create a pull request on your own.** Not with `gh pr create`, not through the GitHub API or any other tool, not as the last step of a finished task. A pull request is created only when the user explicitly asks for one in the current conversation. When the work is finished, commit it on a purpose branch and say that a pull request can be opened on request.

**You are only allowed to create a pull request when the user requests it.** No saved note, memory, earlier habit or other repository's workflow makes an exception. A request covers only the pull request it names: a later branch, a follow-up change or a change in another repository needs a request of its own.

### Commit Message Format

Use conventional commits: `type: lowercase description` — no trailing period.

Examples:

- `feat: add torus geometry, web api and example`
- `fix: correct cap normal on conical frustum`
- `docs: describe the mesh pipeline`
- `refactor: simplify sweep triangulation`

**Do not put ticket numbers in commit messages or PR titles.** Reference the ticket in the PR body if needed.

### Pull Requests

- Create a PR only when the user requests it — see [Hard Rule: NO Automatic Pull Requests](#hard-rule-no-automatic-pull-requests)
- Keep PRs focused and small
- Include clear description of changes
- Reference related documentation
- Ensure all tests pass
- Update documentation in same PR as code changes

### Pull Request Resolution

**NEVER merge a PR.** Not with `gh pr merge`, not by enabling auto-merge. Merging is always a human decision performed by maintainers — finish the review resolution and stop.

**Load the skill** [`.agents/skills/solid-euclid-pr-resolution.md`](../.agents/skills/solid-euclid-pr-resolution.md) when resolving a PR.

**Hard Rules:**

- **Never rewrite history.** No `git commit --amend`, no `git push --force`. Every review fix is a **new** conventional commit.
- **Never silently ignore or resolve a comment.** Every handled thread gets a reply comment first, then is marked resolved.
- **Verify before implementing.** A reviewer suggestion is a hypothesis, not an instruction. Confirm it against the actual code before changing anything.

**Workflow:**

1. **Identify the PR** — Check out the PR branch explicitly, do not rely on current branch
2. **Fetch unresolved review threads** — Use GitHub API to get all unresolved threads
3. **Verify each suggestion** — Read the code, classify as valid/invalid/valid-intent-wrong-form
4. **Implement and validate** — Apply changes, test, commit as new conventional commits, push
5. **Reply to every comment** — State applied/applied-differently/not-applied with reasoning
6. **Resolve each thread** — Mark threads as resolved via GitHub API
7. **Final verification** — Confirm all threads are resolved. Stop. Do not merge.

**Completion Checklist:**

- [ ] Every unresolved thread was verified against the actual code
- [ ] Confirmed-valid suggestions are implemented, tested, and formatted
- [ ] Fixes are pushed as new commits (no amend, no force push)
- [ ] Every handled thread has a reply stating applied/applied differently/not applied
- [ ] Every handled thread is marked resolved
- [ ] The PR was **not** merged

## Bug Fixing Guidelines

### Core Principles

- **Root cause analysis is mandatory** — Understand why the bug occurs before fixing
- **Reproduce first** — Verify the bug exists and understand its impact (a failing Rust unit test, a failing vitest or doctest at the bridge)
- **Fix at the source** — Address the root cause, not symptoms: wrong geometry means a wrong assumption in the algorithm (winding, tessellation density, edge treatment), not a wrong output mesh
- **Test the fix** — Verify it resolves the issue without introducing regressions, at the layer where the root cause lives and at the bridges when the bug crossed a layer

### Process

1. **Reproduce** — Identify steps to reproduce the issue
2. **Investigate** — Trace the data flow and identify the root cause
3. **Implement** — Write clean, focused fix
4. **Validate** — Test the fix thoroughly
5. **Document** — Update documentation if needed

## Performance Considerations

- **Precision boundary** — f64 in the kernel, f32 at the mesh boundary; do not widen the mesh data
- **Zero-copy in the web API** — typed-array views over wasm memory; never copy per call
- **Tessellation density is parameterized** — `segments`/`slices` control the mesh size; keep the defaults sane and the error on implausible values a `GeometryError`
- **Ownership in the C bridge** — every allocation crosses the ABI once and is freed exactly once by the caller
- **Large meshes** — build into the mesh with `add_vertex`/`add_polygon` and let the per-material grouping happen once in `TriangularMesh::from_polygon_mesh`

## Copilot-Specific Guidance

### Important Constraints

1. **No New Dependencies Without Reason** — the crate's dependencies are locked in `Cargo.lock`, the web tooling in `package-lock.json`; do not add to either without need
2. **Rust Edition 2024** — the crate targets edition 2024; `unsafe` blocks and `#[unsafe(no_mangle)]` follow the edition's rules
3. **wasm-bindgen ABI** — do not return kernel structs by value across the bridge; wrap them (`WasmTriangularMesh`) and expose typed-array views
4. **C ABI Ownership** — every allocation that crosses the bridge needs a matching `*_free` function, documented in `include/solid_euclid.h`; the caller frees exactly once
5. **Keep the Header in Sync** — `include/solid_euclid.h` mirrors `c_api.rs` one to one
6. **Tests at Every Layer** — a new geometry type gets a Rust unit test, a vitest file and a doctest file before it is done

### Common Pitfalls

1. **Forgotten `lib.rs` registration** — a new module that is not `pub mod`-declared does not compile into the crate
2. **`pkg/` is generated** — never edit it; change `web_api.rs` and run `npm run build:wasm`
3. **Forgotten vite input** — a new `examples/web/<name>/` page is not built until it is added to `rollupOptions.input` in `vite.config.ts`
4. **Forgotten CMake test registration** — a new `examples/cpp/tests/test-*.cpp` is not compiled until it is added to `add_executable(solid_euclid_tests …)` in `examples/cpp/CMakeLists.txt`
5. **Precision mix-ups** — f64 in the kernel, f32 at the mesh boundary; a widened mesh or a narrowed model is a bug
6. **Winding** — polygon normals come from the cross product of the first three vertices; a flipped winding flips the facing

### Testing

The repository has three test suites:

- **Rust**: `cargo test` — unit tests beside the code in `solid_euclid/src/`
- **Web API**: `npm test` — builds the wasm with wasm-pack, then vitest over `examples/web/__tests__`
- **C API**: `npm run test:cpp` — CMake build of the `solid_euclid_tests` doctest target, then runs it
- **All**: `npm run test:all` — all three

Manual testing: `npm run dev` builds the wasm and opens the example pages in the browser.

### Quick Reference

#### Commands

```bash
cargo test                  # Rust unit tests of the core
npm run build:wasm          # wasm-pack build into pkg/
npm test                    # wasm-pack build + vitest (the web API)
npm run test:cpp            # CMake build + doctest (the C API)
npm run test:all            # all three suites
npm run dev                 # wasm build + vite dev server (examples/web)
npm run build               # wasm build + tsc + vite build into dist/
npm run lint                # ESLint
npm run format:check        # Prettier
cargo fmt --check           # rustfmt
npm run tidy-up             # full format/lint pass
node .agents/scripts/check-markdown-links.js <changed .md files>
```

#### Key Paths

- **Rust core**: `solid_euclid/src/` — the only place geometry logic lives
- **Web API**: `solid_euclid/src/web_api.rs` → `pkg/` (generated)
- **C API**: `solid_euclid/src/c_api.rs` + `include/solid_euclid.h`
- **Web examples and tests**: `examples/web/` (tests in `examples/web/__tests__/`)
- **C++ examples and tests**: `examples/cpp/` (tests in `examples/cpp/tests/`)
- **Digital brain**: `.agents/` (index: `.agents/README.md`)

### Related Resources

- [roomle-hi-example](https://github.com/roomle-dev/roomle-hi-example) — Pattern repository for these instructions
- [wasm-bindgen book](https://rustwasm.github.io/docs/wasm-bindgen/) — The WASM bridge
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — Naming and API conventions
- [doctest](https://github.com/doctest/doctest) — The C++ test framework
- [three.js](https://threejs.org/) — The web examples' renderer

## Version

This file mirrors [AGENTS.md](../AGENTS.md): when AGENTS.md changes, this file changes in the same commit.

---

**Last Updated**: 2026-10-17
