# Feature Analysis: Model Transformations

- **Status:** Implemented
- **Branch:** `feat/csg`
- **Date:** 2026-10-17

## Layers affected

- **Rust core** (`solid_euclid`) — `transform.rs` exists with `Transformation` and `Transform<T>`; the generic `Transform<T>` cannot be deserialized from JSON at the bridge boundary; `GeometryHash` is not implemented for transformed geometry
- **C bridge** (`c_api.rs`, `include/solid_euclid.h`) — no `create_transform_c` function, no header declaration
- **Web API** (`web_api.rs`, `pkg`) — no `create_transform` function
- **Web examples** (`examples/web`) — no transformation example page, no vitest file
- **C++ examples** (`examples/cpp`) — no doctest file for transformations
- **Living reference** (`.agents/skills/`, `.agents/architecture.md`) — skill files and architecture doc do not mention the transform module

## What was asked and why

The `feat/csg` branch introduces `solid_euclid/src/transform.rs` with a `Transformation` enum (8 variants: matrix, quaternion, translation, uniform scale, XYZ scale, rotation X/Y/Z) and a generic `Transform<T: GeometryModel>` wrapper that applies the transformation to the inner model's polygon mesh. The stated purpose is to support CSG operations (union, difference, intersection), which require positioning operands in space before combining them.

The module is complete at the Rust core level — it compiles, all 794 tests pass, and the `GeometryModel` implementation correctly transforms vertex positions, vertex normals (via inverse-transpose), and polygon normals. But the feature is not usable from either bridge: there is no way to deserialize a `Transform` from JSON because `T` is a generic type parameter with no concrete type at the deserialization site.

This analysis answers: **how should the kernel make `Transform` deserializable from JSON so that both bridges can expose it, and what are the implications of each approach?**

## How the area works today

### The geometry model pattern

Every geometry type in the kernel follows the same shape. A serde-serializable struct with public fields, a `new()` constructor, convenience constructors, and an `impl GeometryModel` that builds a `PolygonMesh` in `to_polygon_mesh()`:

```rust
// platonic_solid.rs:29-33
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub geometry: PlatonicSolid,
    pub material: Material,
}
```

```rust
// sweeps.rs:75-85 (Torus)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torus {
    pub major_radius: f64,
    pub minor_radius: f64,
    pub major_segments: u32,
    pub minor_segments: u32,
    pub material: Material,
}
```

The seven concrete geometry types are:

| Type            | Module              | Struct                                                                                            |
| --------------- | ------------------- | ------------------------------------------------------------------------------------------------- |
| Platonic solids | `platonic_solid.rs` | `Model { geometry: PlatonicSolid, material: Material }`                                           |
| Sphere          | `sphere.rs`         | `Model { geometry: Sphere, material: Material }`                                                  |
| Torus           | `sweeps.rs`         | `Torus { major_radius, minor_radius, major_segments, minor_segments, material }`                  |
| Capsule         | `sweeps.rs`         | `Capsule { radius, height, segments, material }`                                                  |
| Cylinder        | `sweeps.rs`         | `Cylinder { radius, height, segments, bottom_edge, top_edge, material }`                          |
| Conical frustum | `sweeps.rs`         | `ConicalFrustum { bottom_radius, top_radius, height, segments, bottom_edge, top_edge, material }` |
| Extrude         | `sweeps.rs`         | `Extrude { profile: PlanarProfile, height, sample_distance, material }`                           |

### The bridge pattern

Both bridges follow the identical shape for each geometry type:

```
Web API (web_api.rs):
  create_<name>(model: JsValue) -> Result<WasmTriangularMesh, JsError>
    → serde_wasm_bindgen::from_value::<ConcreteType>(model)
    → .to_triangular_mesh()
    → triangular_mesh_to_wasm()

C API (c_api.rs):
  create_<name>_c(model_json: *const c_char) -> CTriangularMesh
    → null check → CStr::from_ptr → serde_json::from_str::<ConcreteType>(json)
    → .to_triangular_mesh()
    → triangular_mesh_to_c()
```

The critical detail: **both bridges deserialize into a concrete type**. `serde_json::from_str::<T>` requires `T` to be a known, concrete type at compile time. This is why every bridge function names its concrete model type in the deserialization call (e.g., `serde_json::from_str::<Torus>(json_str)` at `c_api.rs:339`).

### The current transform.rs implementation

`Transform<T>` at `transform.rs:97-105`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform<T>
where
    T: GeometryModel,
{
    pub transformation: Transformation,
    pub model: T,
}
```

The `Serialize`/`Deserialize` derives work because `glam` types (`DMat4`, `DQuat`, `DVec3`) have the `serde` feature enabled (`Cargo.toml:11`). The unit tests at `transform.rs:635-660` verify round-trip serialization — but only with a private `TestTriangleModel` that is not reachable from JSON.

The `GeometryModel` implementation at `transform.rs:155-174` delegates to the inner model, then transforms every vertex position, vertex normal, and polygon normal in place. It does not touch UVs, does not flip polygon winding for negative-determinant transforms, and does not implement `GeometryHash`.

## The gap

The gap has two parts:

1. **No concrete deserialization target.** `Transform<T>` is generic. At the bridge, there is no `T` to substitute. `serde_json::from_str::<Transform<???>>` does not compile.

2. **No bridge functions, examples, or tests.** Even if the deserialization problem were solved, the feature has no `create_transform` in `web_api.rs`, no `create_transform_c` in `c_api.rs`, no header declaration, no web example, no vitest file, and no C++ doctest file.

Part 2 is mechanical — it follows the exact pattern of the seven existing geometry types. Part 1 is a design decision that this analysis addresses.

## The design question: how to make Transform deserializable from JSON

Five approaches were considered. Each is evaluated against the kernel's constraints: serde-derive compatibility, composition (can a transformed model be transformed again?), extensibility (what happens when a new geometry type is added?), and minimal diff to existing code.

### Option A: Enum-based `AnyModel` type (recommended)

Introduce a serde-tagged enum that wraps every concrete geometry type, then make `Transform<AnyModel>` the bridge-facing type.

```rust
// New module or in transform.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnyModel {
    PlatonicSolid(platonic_solid::Model),
    Sphere(sphere::Model),
    Torus(Torus),
    Capsule(Capsule),
    Cylinder(Cylinder),
    ConicalFrustum(ConicalFrustum),
    Extrude(Extrude),
}

impl GeometryModel for AnyModel {
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        match self {
            AnyModel::PlatonicSolid(m) => m.to_polygon_mesh(),
            AnyModel::Sphere(m) => m.to_polygon_mesh(),
            // ...
        }
    }
}
```

JSON shape:

```json
{
  "transformation": { "Translation": [1.0, 2.0, 3.0] },
  "model": {
    "type": "PlatonicSolid",
    "geometry": "Hexahedron",
    "material": { "Uniform": "default" }
  }
}
```

Or with externally-tagged enum (serde default, no `tag` attribute):

```json
{
  "transformation": { "Translation": [1.0, 2.0, 3.0] },
  "model": {
    "PlatonicSolid": {
      "geometry": "Hexahedron",
      "material": { "Uniform": "default" }
    }
  }
}
```

**Pros:**

- Works with serde derive out of the box — no custom deserializers
- Explicit: the set of transformable types is visible in one place
- Composes: `Transform<AnyModel>` can wrap any supported type; nesting `Transform<Transform<AnyModel>>` works because `Transform<AnyModel>` itself implements `GeometryModel`
- The enum's `GeometryModel` impl is a trivial match-and-delegate (7 arms)
- Adding a new geometry type means adding one enum variant and one match arm

**Cons:**

- Every new geometry type requires updating the enum (but this is a one-line change in two places)
- The enum must be kept in sync with the set of geometry types

### Option B: `Box<dyn GeometryModel>` with custom serde

Make `Transform` hold a `Box<dyn GeometryModel>` and write a custom deserializer that reads a type tag and dispatches to the correct concrete type.

**Pros:**

- Extensible without modifying a central enum

**Cons:**

- `dyn GeometryModel` is not `Serialize` or `Deserialize` — requires custom `Serialize` and `Deserialize` implementations for `Transform<Box<dyn GeometryModel>>`
- Trait objects lose the concrete type; `PartialEq` on `Transform<Box<dyn GeometryModel>>` is not derivable
- More complex serde code, harder to maintain
- The existing `Transform<T>` derive-based approach would need to be replaced, not extended
- Against the AGENTS.md "Simplicity First" rule — significantly more code for a marginal benefit

**Verdict: rejected.** The complexity of custom serde for trait objects outweighs the extensibility benefit. The enum approach (Option A) is simpler and the "add a variant" cost is trivial.

### Option C: Separate concrete Transform types per geometry

Instead of a generic `Transform<T>`, define `TransformedPlatonicSolid`, `TransformedSphere`, etc. — seven structs, each pairing a `Transformation` with one concrete model type.

**Pros:**

- No enum needed
- Each type is self-contained and serde-derivable

**Cons:**

- 7+ near-identical struct definitions — massive code duplication
- Does not compose: `TransformedPlatonicSolid` cannot wrap another transformed model
- Every new geometry type requires a new transformed variant
- The `GeometryModel` impl for each is identical except for the inner type — the duplication is in the struct definition, not just the impl

**Verdict: rejected.** Violates "Simplicity First" — 7x code duplication for no benefit over the enum approach.

### Option D: Flatten transformation into each geometry model

Add an optional `transformation: Option<Transformation>` field to each of the 7 geometry model structs (`platonic_solid::Model`, `sphere::Model`, `Torus`, `Capsule`, `Cylinder`, `ConicalFrustum`, `Extrude`). Each struct's `to_polygon_mesh()` applies the transformation after building the mesh.

**Pros:**

- No wrapper type needed
- Each model is self-contained
- Serde works without any enum

**Cons:**

- Duplicates the `transformation` field and the post-processing logic across 7 structs
- Changes every existing struct — larger diff, touches working code
- Does not compose: you cannot transform a transformed model (would need `Vec<Transformation>` or nested structs)
- The transformation logic (vertex/normal/polygon transformation) would be duplicated or extracted to a helper — if extracted, it is essentially the same as `Transform<T>` but inlined
- Breaks the clean separation: geometry definition vs. spatial placement

**Verdict: rejected.** The wrapper approach (`Transform<T>`) is already implemented and tested. Flattening would mean deleting working code and duplicating logic across 7 types. The wrapper is the right abstraction — it separates "what the shape is" from "where it is in space".

### Option E: Post-hoc mesh transformation at the bridge

The bridge creates the inner model's mesh via `to_triangular_mesh()`, then applies the transformation to the resulting `TriangularMesh` (transforming f32 vertex positions and normals).

**Pros:**

- No changes to the Rust core needed
- Works with any existing type without an enum

**Cons:**

- Transforms f32 mesh data, not f64 geometry — loses precision (the kernel is f64 internally, f32 at the mesh boundary; transforming at the mesh level means the transformation math happens in f32 or requires upcasting)
- The transformation is not part of the model — cannot be serialized as a model, cannot be inspected
- Does not compose: the result is a mesh, not a `GeometryModel`
- The bridge would need transformation logic that belongs in the kernel
- CSG operators need transformed `PolygonMesh` operands (f64), not transformed `TriangularMesh` — this approach cannot support CSG

**Verdict: rejected.** Transforming at the mesh level loses the f64 precision that is the kernel's core design principle, and it cannot support CSG (which operates on `PolygonMesh`). The transformation must happen in the kernel, on f64 data, before tessellation.

### Recommendation

**Option A (enum-based `AnyModel`)** is the recommended approach. It is the simplest solution that works with serde derive, composes correctly, and follows the existing codebase patterns. The enum is a natural fit for the bridge boundary: it is the point where the kernel's type system meets the untyped JSON world.

The enum lives in a new `solid_euclid/src/model.rs` (not `transform.rs`) because it is shared infrastructure — both transformations and future CSG operators need it. It derives `Serialize`/`Deserialize` with serde's default externally-tagged representation, matching how `Material::Uniform(String)` serializes as `{"Uniform": "default"}` throughout the existing codebase.

The `AnyModel` enum includes a `Transform(Box<Transform<AnyModel>>)` variant so that nested transformations are expressible in JSON. Since `Transform<AnyModel>` implements `GeometryModel`, the recursion is well-typed and the `GeometryModel` impl delegates correctly.

## Implications of the recommended approach

### Rust core changes

1. **New `model.rs` module** defining the `AnyModel` enum:
   - 7 variants wrapping each concrete geometry type, plus `Transform(Box<Transform<AnyModel>>)` for nesting
   - `impl GeometryModel` delegating to the inner variant
   - `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`
   - Registered in `lib.rs` as `pub mod model;`

2. **`Transform<AnyModel>`** becomes the bridge-facing type. The existing generic `Transform<T>` stays — it is the general-purpose wrapper. `AnyModel` is the bridge-facing specialization.

3. **Unit tests** for the enum:
   - Each variant's `to_polygon_mesh()` produces the same mesh as the unwrapped type
   - `Transform<AnyModel>` round-trips through JSON for each variant
   - Deserialization error cases (unknown type tag, missing fields)

4. **Negative scale / winding fix** (known issue from the backlog): when the transformation matrix has a negative determinant, polygon winding must be flipped to keep outward-facing normals. This affects `Transform<T>::to_polygon_mesh()` at `transform.rs:155-174`. The fix: check `mat.determinant() < 0.0` and reverse polygon index order for affected polygons.

5. **`GeometryHash` for `Transform<T>`**: the transformed mesh has different vertex positions than the original, so the hash should differ. Implementing `GeometryHash` for `Transform<T>` by hashing the resulting `PolygonMesh` (delegating to `PolygonMesh::geometry_hash()`) is straightforward and correct.

### Bridge changes

6. **Web API** (`web_api.rs`): add `create_transform(model: JsValue) -> Result<WasmTriangularMesh, JsError>` following the exact pattern of `create_platonic_solid` at `web_api.rs:163`. Parse as `Transform<AnyModel>`, convert, wrap.

7. **C API** (`c_api.rs`): add `create_transform_c(model_json: *const c_char) -> CTriangularMesh` following the pattern of `create_platonic_solid_c` at `c_api.rs:206`. Declare in `include/solid_euclid.h`.

8. **Rust unit tests** in both `web_api.rs` and `c_api.rs` following the existing test patterns (geometry counts, material grouping, data consistency, normals normalized, indices within bounds, error handling).

### Example and test changes

9. **Web example** (`examples/web/transform/`): interactive page with:
   - Geometry selector (platonic solid type, sphere, torus, etc.)
   - Transformation type selector (translation, rotation, scale, matrix)
   - Parameter inputs (sliders for translation/rotation/scale)
   - Three.js rendering with solid + wireframe view modes
   - Registered in `vite.config.ts`, linked from `examples/web/main.ts`

10. **Web vitest** (`examples/web/__tests__/transform.test.ts`):
    - Each transformation type tested against a known geometry
    - Vertex position correctness (e.g., translated hexahedron vertices are offset by the translation vector)
    - Normal correctness (rotated normals match expected directions)
    - Material preservation through transformation
    - Error handling (invalid JSON, null input, unknown model type)

11. **C++ doctest** (`examples/cpp/tests/test-transform.cpp`):
    - Each transformation type via JSON
    - Error handling (null JSON, invalid JSON)
    - Registered in `examples/cpp/CMakeLists.txt`

### Documentation changes

12. **Skill files**:
    - `solid-euclid-geometry-model.md` — add `transform::Transform` to the implementors list; mention `AnyModel` as the bridge-facing enum
    - `solid-euclid-web-api.md` — add `create_transform` to the function surface table
    - `solid-euclid-c-api.md` — add `create_transform_c` to the function surface

13. **Architecture doc** (`.agents/architecture.md`): add `transform.rs` to the module map; add `Transform` and `AnyModel` to the geometry types table.

14. **README.md**: mention transformations as a capability.

## Shared prerequisite with CSG

The CSG operators backlog item ([`csg-operators.md`](../backlog/csg-operators.md)) has the same dependency: CSG operands must be deserializable from JSON, which requires the same `AnyModel` enum. The enum should be designed to serve both features:

- `Transform<AnyModel>` — wrap any geometry in a transformation
- `CsgOperation { op: CsgOp, a: AnyModel, b: AnyModel }` — combine two geometries

This means the `AnyModel` enum is defined once, in `solid_euclid/src/model.rs`, and used by both
features. The CSG module would not need its own enum — it reuses `AnyModel` as its operand type.

## Risks and open questions

All open questions from the analysis have been resolved — see **Decisions** below and ADR
[`0002-model-transformations-design.md`](../decisions/0002-model-transformations-design.md).

Remaining risks:

1. **Enum maintenance** — every new geometry type requires adding a variant to `AnyModel`. This is a one-line change in two places (the enum definition and the `GeometryModel` impl), but it is a maintenance obligation. A compile error will catch a missing variant, so this is a safe failure mode.

## Decisions

All design questions are resolved. The authoritative record is ADR
[`0002-model-transformations-design.md`](../decisions/0002-model-transformations-design.md).
Summary:

| #   | Question                                    | Decision                                                                                            |
| --- | ------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | How to deserialize `Transform<T>` from JSON | `AnyModel` enum wrapping all 7 geometry types + a `Transform` variant for nesting                   |
| 2   | Where `AnyModel` lives                      | New `solid_euclid/src/model.rs` — shared infrastructure, not transformation-specific                |
| 3   | Serde tag format                            | Externally-tagged (serde default), consistent with `Material::Uniform`                              |
| 4   | Negative-scale winding fix                  | In scope — check `determinant() < 0.0`, reverse polygon winding                                     |
| 5   | Nested transformation                       | Required — `AnyModel::Transform(Box<Transform<AnyModel>>)` recurses naturally                       |
| 6   | UV transformation                           | UVs unchanged — texture-space coordinates, standard behavior (texture stretches with mesh)          |
| 7   | `GeometryHash` for `Transform<T>`           | Implement — delegate to the resulting `PolygonMesh`'s hash; transformed geometry hashes differently |
| 8   | Scope                                       | Transformations only; CSG is out of scope; `AnyModel` designed for CSG reuse                        |
| 9   | NaN/Inf in transformation parameters        | `GeometryError::InvalidParameter` — garbage in, error out                                           |
| 10  | Zero or negative scale                      | Allowed — mathematically valid, just degenerate                                                     |

## Code and documents the work touches

| File                                                     | Change                                                                                          |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `solid_euclid/src/model.rs`                              | New module: `AnyModel` enum + `GeometryModel` impl + unit tests for deserialization             |
| `solid_euclid/src/transform.rs`                          | Fix negative-scale winding; add `GeometryHash` impl; add validation for NaN/Inf; add unit tests |
| `solid_euclid/src/web_api.rs`                            | Add `create_transform` function + Rust unit tests                                               |
| `solid_euclid/src/c_api.rs`                              | Add `create_transform_c` function + Rust unit tests                                             |
| `include/solid_euclid.h`                                 | Declare `create_transform_c` with ownership doc                                                 |
| `examples/web/transform/index.html`                      | New example page                                                                                |
| `examples/web/transform/main.ts`                         | New example page logic                                                                          |
| `examples/web/__tests__/transform.test.ts`               | New vitest file                                                                                 |
| `examples/cpp/tests/test-transform.cpp`                  | New doctest file                                                                                |
| `examples/cpp/CMakeLists.txt`                            | Register `test-transform.cpp`                                                                   |
| `vite.config.ts`                                         | Register `transform` page in `rollupOptions.input`                                              |
| `examples/web/main.ts`                                   | Add link to transform example                                                                   |
| `.agents/skills/solid-euclid-geometry-model.md`          | Add `Transform` and `AnyModel`                                                                  |
| `.agents/skills/solid-euclid-web-api.md`                 | Add `create_transform` to function table                                                        |
| `.agents/skills/solid-euclid-c-api.md`                   | Add `create_transform_c` to function table                                                      |
| `.agents/architecture.md`                                | Add `model.rs` and `transform.rs` to module map                                                 |
| `README.md`                                              | Mention transformations capability                                                              |
| `.agents/decisions/0002-model-transformations-design.md` | ADR recording all design decisions                                                              |
