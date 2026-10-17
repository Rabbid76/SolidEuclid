# SolidEuclid C API Skill (C Bridge)

The C++ face of the kernel: the `extern "C"` functions in
[`c_api.rs`](../../solid_euclid/src/c_api.rs), the header
[`solid_euclid.h`](../../include/solid_euclid.h), the CMake build and the doctest suite. Load
this skill for changes to the C API, `include/solid_euclid.h`, `examples/cpp`, CMake or doctest.

## ABI

- **Functions** — `pub unsafe extern "C" fn create_<name>_c(model_json: *const c_char)
-> CTriangularMesh`: the model arrives as a null-terminated JSON string (parsed with
  `serde_json::from_str` into the same serde model the web API takes), the mesh leaves as a
  `#[repr(C)]` struct returned by value. `get_*_types_c()` return a JSON array as `*mut c_char`.
- **Error handling** — a failed parse or conversion returns an error mesh built by
  `create_error_mesh` (no entries, the message in the `error` field — check it before reading the
  entries); a null input is answered the same way, never a crash.
- **Ownership** — the bridge allocates; the caller frees. Every returned mesh is released with
  `triangular_mesh_free` (it frees the error string, every entry's material id and geometry
  arrays, then the entries array), every returned string with `string_free_c` (or `hello_c_free`).
  Freeing with `free()` or freeing twice is undefined behaviour — the header documents this per
  function.
- **Structures** — three `#[repr(C)]` structs, mirroring the per-material grouping of
  `TriangularMesh`:
  - `CTriangularGeometry` — flat `*mut f32` arrays `vertices`, `normals` and `uvs` (each with a
    `*_len`), plus `indices: *mut u32` with `indices_len` — indices cross the ABI as `u32`.
  - `CTriangularMeshEntry` — one material id (`material_id: *mut c_char`, a null-terminated C
    string) and its `geometry: CTriangularGeometry`.
  - `CTriangularMesh` — `entries: *mut CTriangularMeshEntry` with `entries_len`, plus
    `error: *mut c_char` (null on success) — an array of per-material entries, not flat float
    buffers.

## Header

`include/solid_euclid.h` mirrors `c_api.rs` one to one: same functions, same structs, doc comments
stating the ownership rule of every returned pointer. Keep the two in sync — a bridge function
without a header declaration (or the reverse) is a bug.

## Build and tests

- **Build** — the root `CMakeLists.txt` builds the Rust crate as a staticlib (`cargo build
--release`) and links it into the C++ examples (C++23). `examples/cpp/CMakeLists.txt` adds the
  example executable and the `solid_euclid_tests` doctest executable.
- **Tests** — doctest, one file per geometry type in `examples/cpp/tests/` (`test-platonic-solid.cpp`,
  `test-sphere.cpp`, `test-sweeps.cpp`, …). A new test file must be added to the
  `add_executable(solid_euclid_tests …)` list in `examples/cpp/CMakeLists.txt`.
- **Run** — `npm run test:cpp` (cmake build of the `solid_euclid_tests` target, then runs it).
  `npm run test:all` runs the Rust, web and C++ suites.
- **Formatting** — `.clang-format` / `.clang-tidy` govern the C++ code; `npm run format:cpp`
  formats `examples/cpp` (the vendored `doctest.h` is excluded).

## Adding a C API function

1. The model struct exists in the Rust core, implements `GeometryModel`, and is serde-serializable
   — see [solid-euclid-geometry-model.md](./solid-euclid-geometry-model.md).
2. Add `create_<name>_c(model_json: *const c_char) -> CTriangularMesh` to `c_api.rs` following the
   existing shape (null check → parse → `to_triangular_mesh()` → `triangular_mesh_to_c`, errors via
   `create_error_mesh`).
3. Declare it in `include/solid_euclid.h` with the ownership doc comment.
4. Add doctest cases to `examples/cpp/tests/` (and register a new file in
   `examples/cpp/CMakeLists.txt`).
5. Run `npm run test:cpp`.
