# 0002 — Model transformations: AnyModel enum, bridge exposure, and design decisions

Status: Accepted
Date: 2026-10-17

## Context

The `feat/csg` branch adds `solid_euclid/src/transform.rs` with a `Transformation` enum (matrix,
quaternion, translation, uniform scale, XYZ scale, rotation X/Y/Z) and a generic `Transform<T:
GeometryModel>` wrapper. The wrapper correctly transforms vertex positions, vertex normals
(inverse-transpose), and polygon normals, and all 794 Rust unit tests pass.

But the feature is unusable from either bridge: `Transform<T>` is generic, and the bridges
deserialize JSON into concrete types. There is no `T` to substitute at the deserialization site.
Additionally, the current implementation has a known limitation with negative-scale (mirroring)
transforms, does not implement `GeometryHash`, and leaves UV coordinates unchanged.

The transformations feature is the prerequisite for CSG operations (union, difference,
intersection), which need to position operands in space. CSG is out of scope for now; this ADR
covers only the transformations design. The `AnyModel` enum introduced here is designed so CSG can
reuse it later.

## Decision

### AnyModel enum in a new model.rs

A new module `solid_euclid/src/model.rs` defines `AnyModel`, a serde-serializable enum wrapping
every concrete geometry type plus a `Transform` variant for nesting:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

`AnyModel` implements `GeometryModel` by delegating to the inner variant. `Transform<AnyModel>`
is the bridge-facing type: the web API's `create_transform` and the C API's `create_transform_c`
deserialize JSON into `Transform<AnyModel>`.

The enum uses serde's default externally-tagged representation, consistent with `Material::Uniform`
(`{"Uniform": "default"}`). The `Transform` variant enables nested transformations — a user can
wrap a transformed model in another transformation, and the JSON representation nests naturally.

The enum lives in `model.rs` (not `transform.rs`) because it is shared infrastructure: it is not
specific to transformations, and CSG operators will reuse it as the operand type. Placing it in a
dedicated module keeps `transform.rs` focused on the transformation logic and makes the type
discoverable.

### Bridge exposure

- **Web API** (`web_api.rs`): `create_transform(model: JsValue) -> Result<WasmTriangularMesh,
JsError>` — parses `Transform<AnyModel>` via `serde_wasm_bindgen::from_value`, converts via
  `to_triangular_mesh()`, wraps with `triangular_mesh_to_wasm`.
- **C API** (`c_api.rs` + `include/solid_euclid.h`):
  `create_transform_c(model_json: *const c_char) -> CTriangularMesh` — null check, parse via
  `serde_json::from_str::<Transform<AnyModel>>`, convert, wrap with `triangular_mesh_to_c`. Header
  declaration with the standard ownership doc.

### Negative-scale winding fix (in scope)

When the transformation matrix has a negative determinant (mirroring), polygon winding must be
flipped to keep outward-facing normals. `Transform<T>::to_polygon_mesh()` checks
`mat.determinant() < 0.0` and reverses polygon index order for affected polygons. Without this fix,
mirrored geometry has inverted normals on the GPU.

### UV coordinates unchanged

UVs are texture-space coordinates, not world-space. They are not transformed by
`Transform<T>::to_polygon_mesh()`. The texture stretches with the mesh — standard behavior in
three.js, Blender, and every mainstream 3D engine.

### GeometryHash for Transform

`Transform<T>` implements `GeometryHash` by delegating to the resulting `PolygonMesh`'s
`geometry_hash()`. A transformed geometry has different vertex positions than the original, so its
hash differs — as expected.

### Validation

- **NaN or Inf values** in `Matrix`, `ScaleXYZ`, `UniformScale`, or rotation angles →
  `GeometryError::InvalidParameter`. Garbage in, error out.
- **Zero or negative scale** → allowed. Mathematically valid, just degenerate (zero scale collapses
  all vertices to a point; negative scale mirrors).

### Scope

Transformations only. CSG operators (union, difference, intersection) are out of scope. The
`AnyModel` enum is designed for CSG reuse: when CSG is implemented, its operand type will be
`AnyModel` without changes to the enum.

## Alternatives

- **`Box<dyn GeometryModel>` with custom serde** — rejected. Trait objects are not
  `Serialize`/`Deserialize` by default; requires custom serde impls, loses `PartialEq`, and
  violates the project's simplicity-first rule.
- **Separate `Transformed*` types per geometry** — rejected. 7+ near-identical struct definitions,
  massive code duplication, does not compose.
- **Flattening `transformation` into each geometry struct** — rejected. Duplicates logic across 7
  types, breaks separation of shape vs. placement, changes working code.
- **Post-hoc mesh transformation at the bridge (f32)** — rejected. Loses f64 precision, cannot
  support CSG (which operates on `PolygonMesh`), transformation logic belongs in the kernel.
- **Transforming UVs** — rejected. UVs are texture-space; transforming them would make textures
  behave unexpectedly. Standard behavior (texture stretches with mesh) is correct.
- **`AnyModel` in `transform.rs`** — rejected. The enum is shared infrastructure, not
  transformation-specific. A dedicated `model.rs` is cleaner and discoverable.
- **`AnyModel` in `geometry_model.rs`** — rejected. That file defines the trait contract; adding 8
  concrete enum variants would bloat it.

## Consequences

- The bridge functions deserialize `Transform<AnyModel>` — a concrete, serde-derivable type.
- Nested transformations work: `AnyModel::Transform(Box<Transform<AnyModel>>)` recurses naturally.
- Adding a new geometry type requires one variant in `AnyModel` and one match arm in its
  `GeometryModel` impl. A compile error catches a missing variant — safe failure mode.
- The negative-scale winding fix changes `Transform<T>::to_polygon_mesh()` behavior for mirroring
  transforms. Existing tests for negative scale (`test_transform_negative_scale`) verify positions
  are negated; new tests verify winding is flipped.
- `GeometryHash` for `Transform<T>` produces a different hash than the untransformed original.
- When CSG is implemented later, its operand type is `AnyModel` — no changes to the enum needed.

The public contract is maintained in
[`../skills/solid-euclid-geometry-model.md`](../skills/solid-euclid-geometry-model.md) and the
feature analysis at [`../feature-analysis/model-transformations.md`](../feature-analysis/model-transformations.md).
