# SolidEuclid Geometry Model Skill

The core contract of the kernel: the `GeometryModel` trait, the mesh pipeline behind it, and the
conventions a new geometry type must follow. Load this skill for any change to the Rust core, a
new geometry type, or mesh/material behaviour.

## The trait

[`geometry_model.rs`](../../solid_euclid/src/geometry_model.rs) defines the whole contract:

```rust
pub trait GeometryModel {
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError>;

    fn to_triangular_mesh(&self) -> Result<TriangularMesh, GeometryError> {
        let polygon_mesh = self.to_polygon_mesh()?;
        Ok(TriangularMesh::from_polygon_mesh(&polygon_mesh))
    }
}
```

- `to_polygon_mesh()` is the only required method — it is the line every geometry type implements.
- `to_triangular_mesh()` tessellates the polygon mesh by default; override it only when a type has
  a cheaper triangulation than the generic polygon path.
- Implementors today: `platonic_solid::Model`, `sphere::Model`, `sweeps::{Torus, Capsule,
Cylinder, ConicalFrustum, Extrude}`, `planar_profile::PlanarProfile`,
  `planar_contour::ContourSet`, plus the wrappers `transform::Transform<T>` and `model::AnyModel`
  (match-and-delegate over every geometry type, with a `Transform` variant for nesting).

## The mesh pipeline

```
GeometryModel ──to_polygon_mesh()──> PolygonMesh ──TriangularMesh::from_polygon_mesh()──> TriangularMesh
 (f64 geometry)                      (f64, polygons)                                        (f32, per material)
```

1. **`PolygonMesh`** ([`polygon_mesh.rs`](../../solid_euclid/src/polygon_mesh.rs)) — the
   interchange mesh. Vertices are `Vertex` values: position, normal and uv as `glam::DVec3`/`DVec2`
   (f64). A `Polygon` is an index list, a normal (`DVec3`) and a `geometry_id` (i32). One material
   id per geometry id (`set_material` / `get_material`).
2. **`TriangularMesh::from_polygon_mesh`**
   ([`triangular_mesh.rs`](../../solid_euclid/src/triangular_mesh.rs#L148)) — tessellates every
   polygon and groups the result into one `TriangularGeometry` per material id: flat f32
   `vertices`, `normals` and `uvs`, and `indices: Vec<Index>` with `Index = i32`
   ([`vertex.rs`](../../solid_euclid/src/vertex.rs#L4)). Only the C and WASM bridges convert the
   indices to `u32`. This per-material grouping is what the GPU and both bridges consume.
3. **`TriangularMesh`** — a map from material id to `TriangularGeometry` (`insert`, `get`,
   `get_mut`, `len`, `is_empty`).

## Conventions

- **Precision** — f64 inside the kernel, f32 at the mesh boundary. Do not widen the mesh data or
  narrow the geometry data to make a conversion easier.
- **Normals** — every vertex carries its normal; polygon normals come from the cross product of
  the first three vertices (`(p1 - p0) × (p2 - p0)`), so the winding of the polygon fixes the
  facing. Keep front faces counter-clockwise seen from outside.
- **Convex polygons** — `add_convex_polygon_from_vertices` assumes a convex polygon; concave
  outlines with holes go through the earcut path in
  [`planar_contour.rs`](../../solid_euclid/src/planar_contour.rs) (triangulate, then stitch).
- **Materials** — [`material.rs`](../../solid_euclid/src/material.rs): `Material::Default`,
  `Uniform(String)`, `Map { default, face_map }` and `Array(Vec<String>)` decide the material id
  of every face (`get_material_for_face`). The triangular mesh is grouped by that id.
- **Errors** — fallible construction returns
  [`GeometryError`](../../solid_euclid/src/geometry_error.rs) (`InsufficientVertices`,
  `NotYetImplemented`, `InvalidParameter`). Propagate it through `Result`; never panic on
  caller-controlled input.
- **Hashing** — [`geometry_hash.rs`](../../solid_euclid/src/geometry_hash.rs) implements
  `GeometryHash` (SHA256, values rounded to 3 decimals) for `PolygonMesh`, `TriangularMesh`, and —
  by delegating to their polygon mesh — for `Transform<T>` and `AnyModel`.
- **Transformations** — [`transform.rs`](../../solid_euclid/src/transform.rs) wraps any model in a
  `Transform<T>`: positions are transformed with the matrix (w=1), normals with the inverse
  transpose (then normalized), UVs stay in texture space. A negative determinant (mirroring)
  reverses the polygon indices so faces keep their outward winding. `Transformation::validate()`
  rejects NaN/Inf parameters with `GeometryError::InvalidParameter`; zero and negative scales are
  allowed. [`model.rs`](../../solid_euclid/src/model.rs) defines `AnyModel`, the serde enum the
  bridges deserialize `Transform<AnyModel>` from (externally tagged, like `Material::Uniform`);
  the `Transform` variant nests transformations.

## Adding a geometry type

1. New `snake_case` module in `solid_euclid/src/`, registered in
   [`lib.rs`](../../solid_euclid/src/lib.rs).
2. A serde-serializable model struct (`Serialize`/`Deserialize`) so both bridges can parse it.
3. `impl GeometryModel` — build the `PolygonMesh` in `to_polygon_mesh()`.
4. Unit tests beside the module (`#[cfg(test)]`): vertex/polygon/triangle counts, normals,
   material grouping, error cases.
5. One `create_*` function per bridge — see
   [solid-euclid-web-api.md](./solid-euclid-web-api.md) and
   [solid-euclid-c-api.md](./solid-euclid-c-api.md).

## File map

| File                                     | Role                                                                  |
| ---------------------------------------- | --------------------------------------------------------------------- |
| `solid_euclid/src/geometry_model.rs`     | The `GeometryModel` trait                                             |
| `solid_euclid/src/polygon_mesh.rs`       | `PolygonMesh`, `Polygon`                                              |
| `solid_euclid/src/triangular_mesh.rs`    | `TriangularMesh`, `TriangularGeometry`                                |
| `solid_euclid/src/vertex.rs`             | `Vertex` (position, normal, uv)                                       |
| `solid_euclid/src/material.rs`           | `Material` assignment strategies                                      |
| `solid_euclid/src/geometry_error.rs`     | `GeometryError`                                                       |
| `solid_euclid/src/geometry_hash.rs`      | `GeometryHash` (SHA256)                                               |
| `solid_euclid/src/model.rs`              | `AnyModel` (serde enum of all geometry types + Transform)             |
| `solid_euclid/src/transform.rs`          | `Transform<T>`, `Transformation` (matrix/quat/translate/scale/rotate) |
| `solid_euclid/src/plane.rs`              | `Plane`                                                               |
| `solid_euclid/src/planar_contour.rs`     | `PlanarContour`, `ContourSet` (earcut path)                           |
| `solid_euclid/src/planar_nurbs_curve.rs` | `PlanarNurbsCurve`                                                    |
| `solid_euclid/src/planar_profile.rs`     | `PlanarProfile` (extrude outlines)                                    |
| `solid_euclid/src/platonic_solid.rs`     | Platonic solids (`Model`)                                             |
| `solid_euclid/src/sphere.rs`             | Sphere (`Model`)                                                      |
| `solid_euclid/src/sweep_model.rs`        | `SweepModel` (profile swept along a path)                             |
| `solid_euclid/src/sweeps.rs`             | `Torus`, `Capsule`, `Cylinder`, `ConicalFrustum`, `Extrude`           |
