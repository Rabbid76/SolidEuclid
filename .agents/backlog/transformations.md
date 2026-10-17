# Backlog: Model Transformations

## Layers affected

- **Rust core** (`solid_euclid`) — `transform.rs` exists with the `Transformation` enum and `Transform<T>` wrapper; bridge exposure is missing
- **C bridge** (`c_api.rs`, `include/solid_euclid.h`) — no `create_transform_c` function, no header declaration
- **Web API** (`web_api.rs`, `pkg`) — no `create_transform` function
- **Web examples** (`examples/web`) — no transformation example page, no vitest file
- **C++ examples** (`examples/cpp`) — no doctest file for transformations

## Current state

`solid_euclid/src/transform.rs` (867 lines) is the only file the `feat/csg` branch adds beyond
`main`. It contains:

- **`Transformation` enum** — 8 variants: `Matrix(DMat4)`, `Quaternion(DQuat)`, `Translation(DVec3)`,
  `UniformScale(f64)`, `ScaleXYZ(DVec3)`, `RotationX(f64)`, `RotationY(f64)`, `RotationZ(f64)`.
  Derives `Debug, Clone, PartialEq, Serialize, Deserialize`. Provides `to_matrix()`,
  `transform_position()`, and `transform_normal()` (inverse-transpose for normals).
- **`Transform<T>` struct** — generic wrapper pairing a `Transformation` with a `T: GeometryModel`.
  Derives `Debug, Clone, PartialEq, Serialize, Deserialize`. Provides convenience constructors
  (`with_matrix`, `with_quaternion`, `with_translation`, `with_uniform_scale`, `with_scale_xyz`,
  `with_rotation_x/y/z`). Implements `GeometryModel` by delegating to the inner model's
  `to_polygon_mesh()` and then transforming every vertex position, vertex normal, and polygon
  normal.
- **Unit tests** — ~40 tests covering every `Transformation` variant, position/normal transforms,
  identity/translation/scale/rotation, edge cases (zero scale, negative scale, 180/360 degree
  rotation, combined matrix), serialization round-trips, and mesh-level checks (material
  preservation, polygon normal updates, triangular mesh output).

The module is registered in `lib.rs` (`pub mod transform;`). All 794 Rust unit tests pass,
including the transform tests.

## What is missing

### 1. Bridge deserialization: AnyModel enum in model.rs

`Transform<T>` derives `Serialize`/`Deserialize`, but `T` is a generic type parameter. The bridges
parse JSON into a concrete type — `serde_json::from_str::<Transform<???>>` has no concrete `T` to
substitute. The unit tests in `transform.rs` only test serialization with a private
`TestTriangleModel`, which is not reachable from JSON.

**Decision (ADR [0002](../decisions/0002-model-transformations-design.md)):** a new
`solid_euclid/src/model.rs` module defines `AnyModel`, a serde-serializable enum wrapping all 7
concrete geometry types plus a `Transform(Box<Transform<AnyModel>>)` variant for nested
transformations:

```rust
pub enum AnyModel {
    PlatonicSolid(platonic_solid::Model),
    Sphere(sphere::Model),
    Torus(Torus),
    Capsule(Capsule),
    Cylinder(Cylinder),
    ConicalFrustum(ConicalFrustum),
    Extrude(Extrude),
    Transform(Box<Transform<AnyModel>>),
}
```

`AnyModel` implements `GeometryModel` by delegating to the inner variant. The bridge functions
deserialize `Transform<AnyModel>`. The enum uses serde's default externally-tagged representation,
consistent with `Material::Uniform`. The `Transform` variant enables nested transformations in JSON.

The JSON contract must be documented in rustdoc on the bridge functions, matching the pattern of
every existing `create_*` function.

### 2. Web API function

No `create_transform(model: JsValue) -> Result<WasmTriangularMesh, JsError>` in `web_api.rs`.
Following the pattern of every other geometry type (platonic solid, sphere, torus, capsule,
cylinder, conical frustum, extrude), the function must:

- Parse the `JsValue` with `serde_wasm_bindgen::from_value`
- Convert via `to_triangular_mesh()`
- Wrap with `triangular_mesh_to_wasm`
- Map errors to `JsError`
- Carry a rustdoc comment with the accepted JSON shape and a TypeScript example

### 3. C API function

No `create_transform_c(model_json: *const c_char) -> CTriangularMesh` in `c_api.rs`, and no
declaration in `include/solid_euclid.h`. Following the existing pattern:

- Null check → parse JSON with `serde_json::from_str` → `to_triangular_mesh()` →
  `triangular_mesh_to_c`
- Errors via `create_error_mesh`
- Header declaration with the ownership doc comment (caller frees with `triangular_mesh_free`)

### 4. Web example page

No `examples/web/transform/` directory. Following the pattern of `platonic_solid/`, `sphere/`,
`sweeps/`, `extrude/`:

- `examples/web/transform/index.html` — page with UI controls for transformation type and
  parameters, plus a geometry selector
- `examples/web/transform/main.ts` — three.js scene rendering a transformed geometry, using
  `examples/web/util/three-util.ts` helpers
- Register in `vite.config.ts` `rollupOptions.input`
- Add a link in `examples/web/main.ts`

### 5. Web vitest file

No `examples/web/__tests__/transform.test.ts`. Following the pattern of `torus.test.ts`,
`capsule.test.ts`, etc.:

- `beforeAll` loads `pkg/solid_euclid_bg.wasm` and calls `initSync`
- Tests for each transformation type (translation, rotation, scale, matrix, quaternion)
- Tests for transformed geometry correctness (vertex positions, normals)
- Tests for material preservation through transformation
- Tests for error handling (invalid input, null input)
- Each test calls `mesh.free()`

### 6. C++ doctest file

No `examples/cpp/tests/test-transform.cpp`. Following the pattern of `test-sweeps.cpp`:

- Tests for each transformation type via JSON
- Tests for transformed geometry correctness
- Tests for error handling (null JSON, invalid JSON)
- Register in `examples/cpp/CMakeLists.txt` `add_executable(solid_euclid_tests …)`

### 7. Rust core unit tests at the bridge boundary

The existing unit tests in `transform.rs` test the `Transform<T>` wrapper with private test models.
Once the bridge deserialization approach is chosen, additional unit tests are needed:

- Deserialization of a `Transform` wrapping each concrete geometry type from JSON
- Round-trip serialization of transformed real geometry (e.g., a transformed hexahedron)
- Error cases for invalid transformation parameters (e.g., zero scale, NaN in matrix)

## Definition of done

The feature is complete when all of the following hold:

1. **Rust core** — `solid_euclid/src/model.rs` defines `AnyModel` (7 geometry variants + `Transform`
   variant for nesting), implements `GeometryModel`, and is registered in `lib.rs`. Unit tests cover
   deserialization from JSON for at least two different inner geometry types, plus error cases.
   `transform.rs` has the negative-scale winding fix, `GeometryHash` impl, and NaN/Inf validation.

2. **Web API** — `create_transform` is exported from `web_api.rs` with a rustdoc comment
   documenting the accepted JSON and a TypeScript example. The function is covered by Rust unit
   tests in `web_api.rs` (geometry counts, material grouping, data consistency, error propagation)
   following the pattern of every existing `create_*` function.

3. **C API** — `create_transform_c` is exported from `c_api.rs` and declared in
   `include/solid_euclid.h` with the ownership doc comment. Covered by Rust unit tests in
   `c_api.rs` (null JSON, invalid JSON, geometry data consistency, normals normalized, indices
   within bounds).

4. **Web example** — `examples/web/transform/{index.html,main.ts}` renders a transformed
   geometry with three.js, registered in `vite.config.ts`, linked from `examples/web/main.ts`.

5. **Web tests** — `examples/web/__tests__/transform.test.ts` passes, covering all
   transformation types, geometry correctness, material preservation, and error handling.

6. **C++ tests** — `examples/cpp/tests/test-transform.cpp` passes, registered in
   `examples/cpp/CMakeLists.txt`, covering all transformation types and error handling.

7. **All suites green** — `cargo test`, `npm test`, and `npm run test:cpp` all pass.

8. **Lint and format** — `npm run lint`, `npm run format:check`, `cargo fmt --check` all pass.

9. **Documentation** — the `solid-euclid-geometry-model.md` skill file is updated to mention
   `transform::Transform` and `model::AnyModel` as implementors of `GeometryModel`. The
   `solid-euclid-web-api.md` and `solid-euclid-c-api.md` skill files list the new
   `create_transform` / `create_transform_c` functions in their function surface tables.
   `.agents/architecture.md` includes `model.rs` and `transform.rs` in the module map.

## Known design considerations

All resolved — see ADR [0002](../decisions/0002-model-transformations-design.md) and the
[feature analysis](../feature-analysis/model-transformations.md).

- **Normal transformation** — `transform_normal` uses the inverse-transpose of the matrix, which
  is correct for non-uniform scaling. Non-uniform scale with skew (via `Matrix`) is handled by the
  same code path but is not explicitly tested — worth adding a test.

- **Negative scale / mirroring** — in scope. When the determinant is negative, polygon winding
  must be flipped to keep outward-facing normals. `Transform<T>::to_polygon_mesh()` checks
  `mat.determinant() < 0.0` and reverses polygon index order.

- **UV coordinates** — unchanged by design. UVs are texture-space coordinates; the texture
  stretches with the mesh (standard behavior in three.js, Blender, etc.).

- **GeometryHash** — `Transform<T>` implements `GeometryHash` by delegating to the resulting
  `PolygonMesh`'s hash. A transformed geometry hashes differently from the original.

- **Validation** — NaN or Inf values in transformation parameters return
  `GeometryError::InvalidParameter`. Zero or negative scale is allowed (mathematically valid,
  just degenerate).

- **Scope** — transformations only. CSG operators are out of scope. `AnyModel` is designed so CSG
  can reuse it as its operand type later.
