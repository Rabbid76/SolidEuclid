# Backlog: CSG Operators (Union, Difference, Intersection)

## Layers affected

- **Rust core** (`solid_euclid`) — no CSG module exists; a new `csg` module is needed
- **C bridge** (`c_api.rs`, `include/solid_euclid.h`) — no CSG functions
- **Web API** (`web_api.rs`, `pkg`) — no CSG functions
- **Web examples** (`examples/web`) — no CSG example page, no vitest file
- **C++ examples** (`examples/cpp`) — no doctest file for CSG
- **Transformations** (`.agents/backlog/transformations.md`) — CSG operators are the primary consumer
  of the `Transform` wrapper; a user will want to position two solids before combining them

## Current state

There is no CSG code anywhere in the repository. A grep for `csg`, `boolean`, `union`,
`intersection`, `difference` across all Rust, TypeScript, C++, and header files finds no
implementation. The `.agents/backlog/README.md` already lists "CSG operations (union, difference,
intersection)" as a high-priority open item with the note: "The kernel has no boolean operations;
the plan is a `csg` module behind the `GeometryModel` contract, developed on `feat/csg`."

The `feat/csg` branch currently contains only the `transform.rs` module (see
[transformations.md](./transformations.md)). The branch name signals the intent: CSG operators are
the goal, and transformations are the prerequisite.

## What is missing

### 1. Algorithm research

No research has been done on which CSG algorithm to use. The kernel works with `PolygonMesh`
(f64 vertices, polygons with arbitrary vertex counts) as its interchange representation, and the
output must be a `PolygonMesh` that flows through the existing `TriangularMesh` pipeline. The
research should evaluate:

- **BSP-tree CSG** (classic, e.g. csg.js) — works on triangle soups, splits polygons along
  partition planes. Robust for convex inputs but can be slow for complex meshes and produces
  many coplanar-face artifacts.
- **Nef polyhedra** — exact, handles all cases, but complex to implement and heavy.
- **Mesh-based boolean (e.g. Manifold, CGAL corefine-and-compute)** — robust mesh booleans that
  produce watertight output. Manifold is a modern C++ library; a Rust port or reimplementation
  would be significant work.
- **Voxel/ SDF-based approaches** — approximate, not suitable for a precision kernel.

The research must conclude with a recommendation that fits the kernel's constraints:

- f64 precision in the kernel
- Input and output are `PolygonMesh` (polygons, not just triangles)
- The existing `GeometryModel` contract (`to_polygon_mesh()` → `PolygonMesh`)
- No new heavy dependencies without strong justification (see AGENTS.md hard rule)

### 2. Rust core CSG module

A new `csg.rs` (or `csg/` directory) module in `solid_euclid/src/`, registered in `lib.rs`. It
must define:

- **Operation enum** — `CsgOp::Union`, `CsgOp::Difference`, `CsgOp::Intersection` (serde-serializable).
- **CSG model struct(s)** — one or more structs implementing `GeometryModel` that take two (or
  more) inner geometry models and an operation. The structs must be serde-serializable so both
  bridges can parse them from JSON. This requires the same "deserialize any geometry from JSON"
  mechanism that the transformations backlog item needs (see
  [transformations.md](./transformations.md) item 1).
- **`impl GeometryModel`** — `to_polygon_mesh()` performs the boolean operation on the two
  inner models' polygon meshes and returns the result as a `PolygonMesh`.
- **Error handling** — degenerate inputs (empty meshes, non-manifold geometry, tolerance
  failures) must return `GeometryError` (likely `InvalidParameter` or a new variant).
- **Unit tests** — beside the module, covering:
  - Basic operations: two overlapping hexahedra (union, difference, intersection)
  - Disjoint solids (union = both, intersection = empty, difference = first unchanged)
  - Contained solids (one inside the other)
  - Vertex/polygon/triangle counts of the result
  - Normal correctness (outward-facing)
  - Material assignment on the result
  - Error cases (empty mesh, invalid parameters)
  - Transformed inputs (a `Transform` wrapping each operand)

### 3. Tolerance model

CSG operations are numerically sensitive. The module must define and document:

- An epsilon/tolerance for point-coincidence, coplanarity, and edge-intersection tests
- How the tolerance scales with the geometry's bounding box
- Behavior at tolerance boundaries (documented, not necessarily resolved perfectly)

### 4. Material assignment on the result

The result of a boolean operation has faces that come from either operand A or operand B. The
material model must decide:

- Faces from operand A keep operand A's material id
- Faces from operand B keep operand B's material id
- New faces created by the intersection (cut faces) get a material — either from a configurable
  "cut material" or from operand A

This must be documented in the rustdoc and reflected in the per-material grouping of the
`TriangularMesh`.

### 5. Web API function

No `create_csg(model: JsValue) -> Result<WasmTriangularMesh, JsError>` in `web_api.rs`. Following
the pattern of every other geometry type:

- Parse with `serde_wasm_bindgen::from_value`
- Convert via `to_triangular_mesh()`
- Wrap with `triangular_mesh_to_wasm`
- Map errors to `JsError`
- Rustdoc with JSON shape and TypeScript example

### 6. C API function

No `create_csg_c(model_json: *const c_char) -> CTriangularMesh` in `c_api.rs`, and no declaration
in `include/solid_euclid.h`. Following the existing pattern (null check → parse → convert → wrap,
errors via `create_error_mesh`, header declaration with ownership doc).

### 7. Web example page

No `examples/web/csg/` directory. The example should demonstrate:

- Two or more primitive solids (e.g., a hexahedron and a sphere) with interactive transformation
  controls (position, rotation) — this is where the `Transform` wrapper is exercised
- Operation selector (union, difference, intersection)
- Real-time re-computation of the boolean result
- Rendering with three.js via `examples/web/util/three-util.ts`
- Registered in `vite.config.ts`, linked from `examples/web/main.ts`

### 8. Web vitest file

No `examples/web/__tests__/csg.test.ts`. Following the pattern of the other test files:

- `beforeAll` loads the wasm module
- Tests for each operation (union, difference, intersection)
- Tests with overlapping, disjoint, and contained solids
- Tests for material grouping on the result
- Tests for transformed operands
- Tests for error handling (invalid input, null input, degenerate geometry)
- Each test calls `mesh.free()`

### 9. C++ doctest file

No `examples/cpp/tests/test-csg.cpp`. Following the pattern of `test-sweeps.cpp`:

- Tests for each operation via JSON
- Tests for overlapping, disjoint, contained solids
- Tests for error handling (null JSON, invalid JSON)
- Register in `examples/cpp/CMakeLists.txt`

### 10. Documentation updates

- `solid-euclid-geometry-model.md` skill — add the CSG model types to the implementors list
- `solid-euclid-web-api.md` skill — add `create_csg` to the function surface table
- `solid-euclid-c-api.md` skill — add `create_csg_c` to the function surface
- `README.md` — mention CSG as a capability (the project overview already lists "Constructive
  Solid Geometry (CSG) operations" as a purpose)
- `.agents/architecture.md` — add the `csg` module to the module map

## Definition of done

The feature is complete when all of the following hold:

1. **Algorithm decision** — a research document (`.agents/feature-analysis/csg-algorithm.md`)
   records the evaluated approaches and the chosen algorithm, with the rationale.

2. **Rust core** — a `csg` module exists in `solid_euclid/src/`, registered in `lib.rs`, with
   serde-serializable model struct(s) implementing `GeometryModel`. The three operations (union,
   difference, intersection) work on at least platonic solids and spheres. Unit tests cover
   overlapping, disjoint, contained, and transformed inputs, plus error cases. A tolerance model
   is defined and documented.

3. **Bridge deserialization** — the CSG model struct can be deserialized from JSON, which
   requires the same "deserialize any geometry from JSON" mechanism as the transformations
   backlog item. This dependency must be resolved first or in parallel.

4. **Web API** — `create_csg` is exported from `web_api.rs` with rustdoc and TypeScript
   example. Covered by Rust unit tests in `web_api.rs`.

5. **C API** — `create_csg_c` is exported from `c_api.rs` and declared in
   `include/solid_euclid.h` with ownership doc. Covered by Rust unit tests in `c_api.rs`.

6. **Web example** — `examples/web/csg/{index.html,main.ts}` demonstrates CSG with interactive
   transformation of operands, registered in `vite.config.ts`, linked from `examples/web/main.ts`.

7. **Web tests** — `examples/web/__tests__/csg.test.ts` passes.

8. **C++ tests** — `examples/cpp/tests/test-csg.cpp` passes, registered in
   `examples/cpp/CMakeLists.txt`.

9. **All suites green** — `cargo test`, `npm test`, and `npm run test:cpp` all pass.

10. **Lint and format** — `npm run lint`, `npm run format:check`, `cargo fmt --check` all pass.

11. **Documentation** — skill files, `README.md`, and `.agents/architecture.md` are updated.

## Dependencies

- **Transformations** (`.agents/backlog/transformations.md`) — CSG operators need to position
  operands in space. Without the transformation bridge functions, CSG is limited to
  axis-aligned primitives at the origin. The transformation work should land first or in
  parallel.

- **Bridge deserialization of generic models** — both transformations and CSG need a way to
  deserialize "any geometry model" from JSON at the bridge boundary. This is a shared
  prerequisite. The design decision (enum-based model, `Box<dyn GeometryModel>`, or per-type
  variants) should be made once and applied to both features.

## Known design considerations

- **Polygon vs triangle input** — the kernel's `PolygonMesh` uses polygons (n-gons), but most
  CSG algorithms operate on triangles. The implementation must either triangulate first (via the
  existing `TriangularMesh` pipeline) and work in triangle space, or handle n-gons directly. The
  simpler path is: triangulate both operands, run the boolean in triangle space, then the result
  flows through `TriangularMesh::from_polygon_mesh` naturally. However, the `GeometryModel`
  contract requires `to_polygon_mesh()`, so the result may need to be converted back to a
  `PolygonMesh`. This round-trip should be lossless for triangle-only meshes.

- **Precision** — f64 throughout the kernel. CSG intersection computations (ray-plane, segment-segment)
  must use f64. The tolerance must be relative to the bounding box, not absolute.

- **Performance** — BSP-tree CSG is O(n^2) in the worst case. For the example meshes (platonic
  solids, low-segment spheres) this is fine. For high-segment meshes, performance will degrade.
  The tessellation parameters (`segments`, `slices`) on the input geometry types already let the
  caller control mesh density, which indirectly controls CSG performance.

- **Coplanar faces** — when two operands share a face (e.g., two hexahedra stacked exactly on
  top of each other), the boolean must handle coplanar polygons. This is a classic hard case for
  mesh booleans. The tolerance model and the chosen algorithm must address it, or document it as
  a known limitation.

- **No new dependencies without reason** — if the chosen algorithm requires a crate (e.g., a
  robust mesh-boolean library), the justification must be strong and the crate must be
  well-maintained. A pure-Rust implementation within `solid_euclid` is preferred.
