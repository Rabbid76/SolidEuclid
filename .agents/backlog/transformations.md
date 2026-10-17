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

### 1. Bridge deserialization is not possible with the generic `Transform<T>`

`Transform<T>` derives `Serialize`/`Deserialize`, but `T` is a generic type parameter. The bridges
parse JSON into a concrete type — `serde_json::from_str::<Transform<???>>` has no concrete `T` to
substitute. The unit tests in `transform.rs` only test serialization with a private
`TestTriangleModel`, which is not reachable from JSON.

To make transformations usable from the bridges, the kernel needs a way to deserialize a
`Transform` wrapping any geometry model from JSON. Options (not prescribed here — this is a
design decision for the implementation):

- An enum-based `Model` type that covers all geometry types (platonic solid, sphere, torus,
  capsule, cylinder, conical frustum, extrude) and can be wrapped in `Transform<ModelEnum>`.
- A `Box<dyn GeometryModel>` approach with a custom deserializer.
- Separate `Transform` variants per geometry type.

Whatever approach is chosen, the JSON contract must be documented in rustdoc on the bridge
functions, matching the pattern of every existing `create_*` function.

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

1. **Rust core** — a JSON-deserializable `Transform` wrapper exists (or an equivalent mechanism)
   that can wrap any existing geometry type. Unit tests cover deserialization from JSON for at
   least two different inner geometry types, plus error cases.

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
   `transform::Transform` as an implementor of `GeometryModel`. The `solid-euclid-web-api.md` and
   `solid-euclid-c-api.md` skill files list the new `create_transform` / `create_transform_c`
   functions in their function surface tables.

## Known design considerations

- **Normal transformation** — `transform_normal` uses the inverse-transpose of the matrix, which
  is correct for non-uniform scaling. The unit tests verify this for uniform scale and rotation.
  Non-uniform scale with skew (via `Matrix`) is handled by the same code path but is not
  explicitly tested — worth adding a test.

- **Negative scale / mirroring** — a negative scale factor flips the winding of polygons. The
  current implementation transforms normals but does not flip polygon winding. This means a
  mirrored geometry will have inverted normals on the GPU. This is a known limitation that should
  be addressed (either by flipping winding when the determinant is negative, or by documenting
  the limitation).

- **UV coordinates** — the current `to_polygon_mesh` implementation transforms positions and
  normals but leaves UVs unchanged. For most transformations this is correct (UVs are
  parameter-space, not world-space), but for completeness the behavior should be documented.

- **GeometryHash** — `transform.rs` does not implement `GeometryHash`. If the hash is used for
  caching or identity, transformed geometry should probably hash differently from the untransformed
  original. This is worth investigating when the bridge functions are added.
