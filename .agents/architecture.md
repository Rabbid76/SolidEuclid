# SolidEuclid Architecture Overview

High-level map of the codebase: what each module does, how the geometry types connect, and how
data flows from a model to a renderable mesh through both bridges. This is living reference —
describe what is, not what was.

## What the project is

SolidEuclid is a geometric modeling kernel written in Rust (edition 2024). It generates 3D
primitives, sweep bodies and extrusions, and converts them to GPU-ready triangular meshes. The
kernel is consumed through two bridges:

- **C bridge** (`solid_euclid/src/c_api.rs` + `include/solid_euclid.h`) — `extern "C"` functions
  taking JSON in, returning `CTriangularMesh` by value; caller frees every allocation.
- **WASM web API** (`solid_euclid/src/web_api.rs` → `pkg/`) — `#[wasm_bindgen]` functions taking
  `JsValue` in, returning `WasmTriangularMesh` with zero-copy typed-array getters.

Both bridges convert geometry through the same `GeometryModel` trait and hand out mesh data only.

## Module map

All geometry logic lives in `solid_euclid/src/`. Every module is `pub mod`-declared in
[`lib.rs`](../solid_euclid/src/lib.rs).

### Core types

| Module                                  | File                                  | Role                                                                                                                                     |
| --------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `GeometryModel` trait                   | `solid_euclid/src/geometry_model.rs`  | The single conversion contract: `to_polygon_mesh()` (required) and `to_triangular_mesh()` (default tessellation)                         |
| `PolygonMesh` / `Polygon`               | `solid_euclid/src/polygon_mesh.rs`    | f64 interchange mesh: vertices (position/normal/uv), polygons (indices + normal + geometry_id), material map (geometry_id → material_id) |
| `TriangularMesh` / `TriangularGeometry` | `solid_euclid/src/triangular_mesh.rs` | f32 GPU-ready mesh: per-material triangle soups with flat vertex/normal/uv arrays and `Index = i32` indices                              |
| `Vertex` / `Index`                      | `solid_euclid/src/vertex.rs`          | f64 vertex (position, normal, uv); `Index = i32`                                                                                         |
| `Material`                              | `solid_euclid/src/material.rs`        | Material assignment: `Default`, `Uniform(String)`, `Map { default, face_map }`, `Array(Vec<String>)`                                     |
| `GeometryError`                         | `solid_euclid/src/geometry_error.rs`  | The crate error type: `InsufficientVertices`, `NotYetImplemented`, `InvalidParameter`                                                    |
| `GeometryHash`                          | `solid_euclid/src/geometry_hash.rs`   | SHA256 over `PolygonMesh` and `TriangularMesh` (values rounded to 3 decimals)                                                            |
| `Plane`                                 | `solid_euclid/src/plane.rs`           | 3D plane (origin, normal, x-axis) with orthonormalized transformation matrix                                                             |

### Geometry types (implement `GeometryModel`)

| Geometry        | Module                               | Struct                                                                                            | Description                                                                            |
| --------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Platonic solids | `solid_euclid/src/platonic_solid.rs` | `Model { geometry: PlatonicSolid, material: Material }`                                           | Tetrahedron, Hexahedron, Octahedron, Dodecahedron, Icosahedron                         |
| Sphere          | `solid_euclid/src/sphere.rs`         | `Model { geometry: Sphere, material: Material }`                                                  | UV sphere (`Slices { slices, segments }`) and cube-mapped sphere (`Cube { segments }`) |
| Torus           | `solid_euclid/src/sweeps.rs`         | `Torus { major_radius, minor_radius, major_segments, minor_segments, material }`                  | Donut shape via sweep of a circular profile along a circular path                      |
| Capsule         | `solid_euclid/src/sweeps.rs`         | `Capsule { radius, height, segments, material }`                                                  | Cylinder with hemispherical caps; 3 material regions (bottom, body, top)               |
| Cylinder        | `solid_euclid/src/sweeps.rs`         | `Cylinder { radius, height, segments, bottom_edge, top_edge, material }`                          | Special case of `ConicalFrustum` with equal radii; 5 material regions                  |
| Conical frustum | `solid_euclid/src/sweeps.rs`         | `ConicalFrustum { bottom_radius, top_radius, height, segments, bottom_edge, top_edge, material }` | Truncated cone; 5 material regions (bottom cap, bottom edge, body, top edge, top cap)  |
| Extrude         | `solid_euclid/src/sweeps.rs`         | `Extrude { profile: PlanarProfile, height, sample_distance, material }`                           | 2D profile extruded along Y-axis with bottom/top caps                                  |

### Supporting geometry modules

| Module                         | File                                     | Role                                                                                                                                        |
| ------------------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `SweepModel`                   | `solid_euclid/src/sweep_model.rs`        | Generic sweep: a profile swept along a path, both sampled via `SampleContour`                                                               |
| `PlanarProfile`                | `solid_euclid/src/planar_profile.rs`     | 2D profile defined by path commands (`Move`, `Line`, `Ellipse`, `Nurbs`) with corner treatments (`Edge`, `SmoothEdge`, `Chamfer`, `Fillet`) |
| `PlanarContour` / `ContourSet` | `solid_euclid/src/planar_contour.rs`     | Sampled contour representation (points with tangents, segments); earcut triangulation for concave polygons with holes                       |
| `PlanarNurbsCurve`             | `solid_euclid/src/planar_nurbs_curve.rs` | NURBS curve evaluation and sampling                                                                                                         |

### Bridge modules

| Module  | File                          | Role                                                                                                |
| ------- | ----------------------------- | --------------------------------------------------------------------------------------------------- |
| C API   | `solid_euclid/src/c_api.rs`   | `extern "C"` functions: JSON in → `CTriangularMesh` out; `*_free` functions for every allocation    |
| Web API | `solid_euclid/src/web_api.rs` | `#[wasm_bindgen]` functions: `JsValue` in → `WasmTriangularMesh` out; zero-copy typed-array getters |

## Data flow

```
Model (serde-serializable struct)
  │  impl GeometryModel
  ▼
to_polygon_mesh() ──► PolygonMesh (f64, polygons, material_map)
  │  TriangularMesh::from_polygon_mesh()
  ▼
TriangularMesh (f32, per-material triangle soups)
  │
  ├──► C bridge: triangular_mesh_to_c() ──► CTriangularMesh (repr(C), caller frees)
  │
  └──► WASM bridge: triangular_mesh_to_wasm() ──► WasmTriangularMesh (typed-array views)
```

## The mesh pipeline

1. **Build** — each geometry type constructs a `PolygonMesh` in `to_polygon_mesh()`: vertices
   with position/normal/uv (f64 via `glam`), polygons with indices + normal + geometry_id, and a
   material map (geometry_id → material_id string).
2. **Tessellate** — `TriangularMesh::from_polygon_mesh()` fan-triangulates convex polygons,
   converts f64 → f32, and groups triangles by material id into one `TriangularGeometry` per
   material.
3. **Bridge** — the C and WASM bridges convert the `TriangularMesh` into their respective
   representations. Neither bridge exposes geometry internals; both hand out mesh data only.

## Bridge surfaces

### C bridge (`c_api.rs` + `include/solid_euclid.h`)

| Function                     | Input             | Returns               |
| ---------------------------- | ----------------- | --------------------- |
| `create_platonic_solid_c`    | JSON string       | `CTriangularMesh`     |
| `create_sphere_c`            | JSON string       | `CTriangularMesh`     |
| `create_torus_c`             | JSON string       | `CTriangularMesh`     |
| `create_capsule_c`           | JSON string       | `CTriangularMesh`     |
| `create_cylinder_c`          | JSON string       | `CTriangularMesh`     |
| `create_conical_frustum_c`   | JSON string       | `CTriangularMesh`     |
| `create_extrude_c`           | JSON string       | `CTriangularMesh`     |
| `get_platonic_solid_types_c` | —                 | JSON array string     |
| `get_sphere_types_c`         | —                 | JSON array string     |
| `hello_c`                    | —                 | string                |
| `triangular_mesh_free`       | `CTriangularMesh` | frees all allocations |
| `string_free_c`              | `*mut c_char`     | frees string          |
| `hello_c_free`               | `*mut c_char`     | frees string          |

Three `#[repr(C)]` structs mirror the per-material grouping: `CTriangularGeometry` (flat f32
arrays + u32 indices), `CTriangularMeshEntry` (material_id string + geometry), `CTriangularMesh`
(entries array + error string). The header `include/solid_euclid.h` mirrors `c_api.rs` one to one.

### WASM web API (`web_api.rs` → `pkg/`)

| Function                   | Input     | Returns                                   |
| -------------------------- | --------- | ----------------------------------------- |
| `create_platonic_solid`    | `JsValue` | `WasmTriangularMesh`                      |
| `create_sphere`            | `JsValue` | `WasmTriangularMesh`                      |
| `create_torus`             | `JsValue` | `WasmTriangularMesh`                      |
| `create_capsule`           | `JsValue` | `WasmTriangularMesh`                      |
| `create_cylinder`          | `JsValue` | `WasmTriangularMesh`                      |
| `create_conical_frustum`   | `JsValue` | `WasmTriangularMesh`                      |
| `create_extrude`           | `JsValue` | `WasmTriangularMesh`                      |
| `create_planar_profile`    | `JsValue` | `JsValue` (reserialized profile, no mesh) |
| `get_platonic_solid_types` | —         | `js_sys::Array`                           |
| `get_sphere_types`         | —         | `js_sys::Array`                           |
| `hello_wasm`               | —         | `String`                                  |

`WasmTriangularMesh` exposes `geometry_count`, `material_ids`, `get_geometry(index)`,
`get_geometry_by_material(id)`. `WasmTriangularGeometry` exposes zero-copy typed-array views
(`vertices`, `normals`, `uvs`, `indices`) plus `vertex_count` and `triangle_count`.

## Examples

| Location                  | Technology                   | Contents                                                                                                                |
| ------------------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `examples/web/`           | Vite + three.js (TypeScript) | One page per geometry type: `platonic_solid/`, `sphere/`, `sweeps/`, `extrude/`; shared helpers in `util/three-util.ts` |
| `examples/web/__tests__/` | vitest                       | One test file per geometry type, exercising the WASM API                                                                |
| `examples/cpp/`           | CMake + doctest (C++23)      | `main.cpp` example; `tests/` with one file per geometry type                                                            |

## Key invariants

- **f64 inside, f32 at the boundary** — the kernel uses `glam::DVec3`/`DVec2`; only the
  `TriangularMesh` stores f32.
- **Winding** — polygon normals come from the cross product of the first three vertices; front
  faces are counter-clockwise seen from outside.
- **Convex polygons** — `add_convex_polygon_from_vertices` assumes convex; concave outlines with
  holes go through the earcut path in `planar_contour.rs`.
- **Error handling** — fallible conversions return `GeometryError`; the bridges convert it at
  their edge (JsError in WASM, error mesh in C). Never panic on caller-controlled input.
- **No geometry logic in bridges or examples** — all geometry construction lives in the Rust core.
- **Header in sync** — `include/solid_euclid.h` mirrors `c_api.rs` one to one; a bridge function
  without a header declaration (or the reverse) is a bug.
- **`pkg/` is generated** — never edit it; change `web_api.rs` and run `npm run build:wasm`.

## Adding a geometry type

See the step-by-step pattern in [`AGENTS.md`](../AGENTS.md#adding-new-geometry) and the detailed
skill files:

- [solid-euclid-geometry-model.md](skills/solid-euclid-geometry-model.md) — the `GeometryModel`
  trait, mesh pipeline, conventions
- [solid-euclid-web-api.md](skills/solid-euclid-web-api.md) — WASM API, vitest, example pages
- [solid-euclid-c-api.md](skills/solid-euclid-c-api.md) — C bridge, header, CMake, doctest

## Dependencies

Rust crate (`solid_euclid/Cargo.toml`):

| Crate                     | Purpose                                               |
| ------------------------- | ----------------------------------------------------- |
| `glam` (0.30, serde)      | f64/f32 vector and matrix math                        |
| `serde` / `serde_json`    | Serialization for model structs and C bridge JSON     |
| `serde-wasm-bindgen`      | `JsValue` ↔ Rust struct conversion in the WASM bridge |
| `wasm-bindgen` / `js-sys` | WASM bindings and typed-array views                   |
| `earcutr`                 | Earcut triangulation for concave polygons             |
| `sha2` / `hex`            | SHA256 hashing for `GeometryHash`                     |

Web tooling (`package.json`): vite, vitest, typescript, eslint, prettier, three.js, wasm-pack
(via `npm run build:wasm`).

C++ tooling: CMake (≥3.16), C++23, doctest (vendored in `examples/cpp/tests/doctest.h`).
