# SolidEuclid Web API Skill (WebAssembly)

The TypeScript/JavaScript face of the kernel: the wasm-bindgen bindings in
[`web_api.rs`](../../solid_euclid/src/web_api.rs), the `pkg/` build, the example pages and the
vitest suite. Load this skill for changes to the web API, `examples/web`, wasm-pack or vitest.

## Build and binding

- **Build** — `npm run build:wasm` runs `wasm-pack build solid_euclid --target web --out-dir
../pkg`. The output in `pkg/` (JS glue, `.d.ts`, `.wasm`) is git-ignored; never edit it — change
  `web_api.rs` and rebuild.
- **Binding** — vite and vitest alias `@solid-euclid` to `pkg` (`vite.config.ts`,
  `vitest.config.ts`). Examples and tests import from `@solid-euclid`.
- **ABI** — `#[wasm_bindgen]` functions take the model as `JsValue` and parse it with
  `serde_wasm_bindgen::from_value` (the TypeScript call passes a plain object matching the
  serde shape of the model struct, e.g. `{ geometry: "Hexahedron", material: { Uniform:
"default" } }`). The geometry functions return `WasmTriangularMesh` or fail with `JsError`; the one
  exception is `create_planar_profile`, which returns the reserialized profile as `JsValue` (see the
  function surface below).
- **Zero-copy** — `WasmTriangularGeometry` exposes `vertices`, `normals`, `uvs` and `indices` as
  typed-array views (`js_sys::Float32Array::view` / `Uint32Array::view`) plus `vertex_count` and
  `triangle_count`. The data stays in wasm memory; do not copy it per call.
- **Ownership** — wasm-bindgen generates `mesh.free()`; the tests call it after reading.

## The function surface

One `create_*` function per geometry type. Each converts through `to_triangular_mesh()` and returns
a `WasmTriangularMesh` — with one exception, noted after the table:

| Function                          | Model                           |
| --------------------------------- | ------------------------------- |
| `create_platonic_solid(model)`    | `platonic_solid::Model`         |
| `create_sphere(model)`            | `sphere::Model`                 |
| `create_torus(torus)`             | `sweeps::Torus`                 |
| `create_capsule(capsule)`         | `sweeps::Capsule`               |
| `create_cylinder(cylinder)`       | `sweeps::Cylinder`              |
| `create_conical_frustum(frustum)` | `sweeps::ConicalFrustum`        |
| `create_extrude(extrude)`         | `sweeps::Extrude`               |
| `create_planar_profile(profile)`  | `planar_profile::PlanarProfile` |

Plus `get_platonic_solid_types()` / `get_sphere_types()` returning `js_sys::Array` of the variant
names, and `hello_wasm()`.

`create_planar_profile` is the exception: it parses a `PlanarProfile` and returns it reserialized as
`JsValue` for use with `create_extrude`
([`web_api.rs`](../../solid_euclid/src/web_api.rs#L520)). It builds no mesh — the returned profile
has no geometry properties and no `free()`.

## Tests

- **Framework** — vitest (`vitest.config.ts`, includes `examples/web/__tests__/**/*.test.ts`).
- **Location** — `examples/web/__tests__/<name>.test.ts`, one file per geometry type.
- **Pattern** — `beforeAll` loads `pkg/solid_euclid_bg.wasm` and calls `initSync({ module:
wasmBuffer })`; each test calls the `create_*` function and asserts `geometry_count`,
  `material_ids`, `vertex_count` / `triangle_count` of the per-material geometries, then
  `mesh.free()`.
- **Run** — `npm test` (builds the wasm first) or `npm run test:watch`.

## Example pages

- One folder per geometry type under `examples/web/` (`platonic_solid/`, `sphere/`, `sweeps/`,
  `extrude/`), each with an `index.html` and a `main.ts` rendering the mesh with three.js via
  `examples/web/util/three-util.ts`.
- A new page needs an entry in `rollupOptions.input` of `vite.config.ts` or it is not built.
- **Run** — `npm run dev` (builds the wasm, starts vite, opens the browser); `npm run build`
  produces `dist/`; `npm run preview` serves it.

## Adding a web API function

1. The model struct exists in the Rust core, implements `GeometryModel`, and is serde-serializable
   — see [solid-euclid-geometry-model.md](./solid-euclid-geometry-model.md).
2. Add `create_<name>(model: JsValue) -> Result<WasmTriangularMesh, JsError>` to
   `web_api.rs`: parse with `serde_wasm_bindgen::from_value`, convert with `to_triangular_mesh()`,
   map errors to `JsError`, wrap with the existing `triangular_mesh_to_wasm` helper. Document the
   accepted JSON in the doc comment with a TypeScript example.
3. Add `examples/web/__tests__/<name>.test.ts` following the pattern above.
4. Add the example page `examples/web/<name>/{index.html,main.ts}` and register it in
   `vite.config.ts`.
5. Run `npm test` and `npm run build`.
