# 0001 — GeometryModel is the single mesh-conversion contract

Status: Accepted
Date: 2026-10-17

## Context

Every geometry type in the kernel — platonic solids, the sphere, the sweep bodies (torus, capsule,
cylinder, conical frustum), extrusions from planar profiles and contour sets — has to reach the same
destinations: a renderable triangular mesh for the GPU, grouped by material. The kernel is
consumed through two bridges (C and WebAssembly), and both bridges must offer the same surface
without duplicating geometry logic. Geometry types are added frequently; each new type needs a
predictable path from "model" to "mesh".

## Decision

All geometry types implement the `GeometryModel` trait
([`geometry_model.rs`](../../solid_euclid/src/geometry_model.rs#L15)): `to_polygon_mesh()`
builds a `PolygonMesh` (f64 vertices with position, normal and uv; polygons with a normal and a
geometry id; one material id per geometry id), and `to_triangular_mesh()` has a default
implementation that tessellates the polygon mesh into a `TriangularMesh` — per-material triangle
soups of f32 vertices, normals and uvs with `Index = i32` indices (the bridges convert the indices
to `u32` at their edge). Both bridges convert through
`to_triangular_mesh()` and hand out mesh data only; they never expose geometry internals, and no
geometry logic lives in `c_api.rs`, `web_api.rs` or the examples. New geometry types implement the
trait and appear in the bridges through one `create_*` function per layer.

## Alternatives

- **Per-bridge meshing.** Letting each bridge tessellate on its side would duplicate the
  triangulation and material grouping and could make the C and web output disagree.
- **Returning geometry structs across the bridges.** Exposing the model types through
  wasm-bindgen or `extern "C"` would leak kernel internals into the ABI and force every consumer to
  understand every geometry type.
- **Triangulating eagerly in the model.** Building the triangular mesh inside each geometry type
  would push the material grouping and the f32 conversion into every type instead of one shared
  path.

## Consequences

- A new geometry type is one `impl GeometryModel` plus one `create_*` function per bridge — the
  pattern every primitive follows (platonic solid → sphere → torus → capsule → cylinder → conical
  frustum → extrude).
- The polygon mesh is the single place where winding, normals and material ids are decided; the
  triangular mesh is a pure projection of it.
- `to_triangular_mesh()` stays overridable for a type whose triangulation is cheaper than the
  generic polygon path.
- The bridges inherit the contract's error type (`GeometryError`) and convert it at their edge.

The public contract is maintained in
[`../skills/solid-euclid-geometry-model.md`](../skills/solid-euclid-geometry-model.md).
