# SolidEuclid Development Guide

Build, test, lint, debug and deployment commands. Living reference — describe what is, not what
was. For the workflow rules (commit format, PR policy, change workflow) see
[`AGENTS.md`](../AGENTS.md).

## Prerequisites

| Tool                          | Purpose                                       |
| ----------------------------- | --------------------------------------------- |
| Rust (edition 2024 toolchain) | Build and test the kernel                     |
| Node.js + npm                 | Web tooling, examples, vitest, wasm-pack      |
| wasm-pack                     | Build the WASM package (`npm run build:wasm`) |
| CMake ≥ 3.16                  | Build the C++ examples and doctest suite      |
| C++23 compiler                | Compile the C++ examples                      |
| clang-format / clang-tidy     | C++ formatting and static analysis            |

## Quick reference

```bash
# ── Rust core ──────────────────────────────────────────────
cargo test                  # unit tests beside the code
cargo test <name>           # run tests matching <name>
cargo build --release       # release build (used by CMake)
cargo fmt --check           # check rustfmt
cargo fmt                   # format the crate
cargo fix --allow-dirty     # apply compiler suggestions
cargo doc --open            # build and open rustdoc

# ── WASM build ─────────────────────────────────────────────
npm run build:wasm          # wasm-pack build into pkg/

# ── Web examples + tests ───────────────────────────────────
npm install                 # install dependencies (first time)
npm run dev                 # wasm build + vite dev server (opens browser)
npm run build               # wasm build + tsc + vite build into dist/
npm run preview             # serve dist/
npm test                    # wasm build + vitest run
npm run test:watch          # wasm build + vitest watch mode

# ── C++ examples + tests ───────────────────────────────────
cmake -B build              # configure (builds Rust staticlib via CMake)
npm run test:cpp            # build doctest target + run it

# ── All suites ─────────────────────────────────────────────
npm run test:all            # cargo test + npm test + npm run test:cpp

# ── Lint / format ──────────────────────────────────────────
npm run lint                # ESLint (TypeScript)
npm run lint:fix            # ESLint --fix
npm run format:check        # Prettier check
npm run format              # Prettier write
npm run format:cpp          # clang-format on examples/cpp
npm run tidy-up             # full pass: prettier, eslint --fix, cargo fix, cargo fmt, clang-format

# ── Documentation ──────────────────────────────────────────
node .agents/scripts/check-markdown-links.js <file.md> …   # check relative markdown links
```

## Test suites

Three independent suites, one per layer:

| Suite     | Command            | Framework      | Location                              | What it covers                                                   |
| --------- | ------------------ | -------------- | ------------------------------------- | ---------------------------------------------------------------- |
| Rust core | `cargo test`       | `#[cfg(test)]` | `solid_euclid/src/*.rs` (beside code) | Geometry construction, mesh pipeline, materials, errors, hashing |
| Web API   | `npm test`         | vitest         | `examples/web/__tests__/*.test.ts`    | WASM bridge functions, mesh data integrity, material grouping    |
| C API     | `npm run test:cpp` | doctest        | `examples/cpp/tests/test-*.cpp`       | C bridge functions, JSON parsing, mesh data, memory management   |

`npm run test:all` runs all three in sequence (Rust → web → C++).

### Rust tests

Unit tests live beside the code in `#[cfg(test)] mod tests` blocks. They test geometry
construction, vertex/polygon/triangle counts, normals, winding, material grouping, error cases,
and serialization round-trips. Run a single test with `cargo test <name>`.

### Web API tests (vitest)

Each test file (`examples/web/__tests__/<name>.test.ts`) follows the same pattern:

1. `beforeAll` loads `pkg/solid_euclid_bg.wasm` via `initSync({ module: wasmBuffer })`.
2. Each test calls a `create_*` function and asserts `geometry_count`, `material_ids`,
   `vertex_count` / `triangle_count` of the per-material geometries.
3. `mesh.free()` releases the wasm object after reading.

Run with `npm test` (builds wasm first) or `npm run test:watch`.

### C API tests (doctest)

Each test file (`examples/cpp/tests/test-*.cpp`) exercises the C bridge:

1. Build a JSON string with `CString::new(r#"{...}"#)`.
2. Call the `create_*_c` function.
3. Assert `mesh.error == nullptr`, check `entries_len`, material ids, vertex/triangle counts.
4. Call `triangular_mesh_free(mesh)`.

A new test file must be registered in `examples/cpp/CMakeLists.txt` under
`add_executable(solid_euclid_tests …)`.

Run with `npm run test:cpp` (requires `cmake -B build` first).

## Building

### Rust crate

```bash
cargo build                 # debug build
cargo build --release       # release build (staticlib for CMake)
```

The crate produces three library types (`solid_euclid/Cargo.toml`): `cdylib` (WASM),
`staticlib` (C bridge via CMake), `rlib` (Rust consumers).

### WASM package

```bash
npm run build:wasm
```

Runs `wasm-pack build solid_euclid --target web --out-dir ../pkg`. Output (`pkg/`) is git-ignored
and never edited — change `web_api.rs` and rebuild.

### Web examples

```bash
npm run dev                 # dev server with hot reload
npm run build               # production build into dist/
npm run preview             # serve dist/
```

Vite aliases `@solid-euclid` to `pkg/`. Example pages are registered in
`vite.config.ts` under `rollupOptions.input` — a new page is not built until added there.

### C++ examples

```bash
cmake -B build              # configure (triggers cargo build --release)
cmake --build build         # build everything
cmake --build build --target solid_euclid_tests   # build tests only
./build/examples/cpp/solid_euclid_tests           # run tests
./build/examples/cpp/solid_euclid_cpp_example     # run example
```

The root `CMakeLists.txt` builds the Rust staticlib via a custom target and links it into the
C++ executables. Platform-specific system libraries are linked (macOS: Security/CoreFoundation;
Linux: pthread/dl/m; Windows: ws2_32/userenv/bcrypt/ntdll).

## Linting and formatting

| Language              | Tool                     | Commands                                                                                      |
| --------------------- | ------------------------ | --------------------------------------------------------------------------------------------- |
| Rust                  | rustfmt                  | `cargo fmt`, `cargo fmt --check`                                                              |
| Rust                  | clippy (via `cargo fix`) | `cargo fix --allow-dirty` (part of `tidy-up`)                                                 |
| TypeScript            | ESLint                   | `npm run lint`, `npm run lint:fix` (config: `eslint.config.js`)                               |
| TypeScript / Markdown | Prettier                 | `npm run format`, `npm run format:check` (config: `.prettierrc`, excludes: `.prettierignore`) |
| C++                   | clang-format             | `npm run format:cpp` (config: `.clang-format`)                                                |
| C++                   | clang-tidy               | `.clang-tidy` config (run manually or via IDE)                                                |
| Markdown              | link checker             | `node .agents/scripts/check-markdown-links.js <file.md> …`                                    |

`npm run tidy-up` runs the full pass: prettier, eslint --fix, cargo fix, cargo fmt,
clang-format. Run it before committing a larger change.

## Debugging

### Debugging wrong geometry

1. **Reproduce at the Rust layer** — write a `#[cfg(test)]` unit test in the module where the
   wrongness starts. The Rust core is the fastest feedback loop.
2. **Trace the mesh pipeline** — check `PolygonMesh` (winding, normals, materials) →
   `TriangularMesh::from_polygon_mesh()` (tessellation, per-material grouping).
3. **Fix the assumption, not the output** — wrong geometry means a wrong assumption in the
   algorithm (winding order, tessellation density, edge treatment), not a wrong output mesh.

### Debugging the WASM bridge

- The `pkg/` output is git-ignored; rebuild with `npm run build:wasm` after changing
  `web_api.rs`.
- `npm run dev` starts a vite dev server with hot reload — the browser opens automatically.
- vitest tests load the wasm directly; `npm run test:watch` gives fast iteration.
- Console output from Rust (e.g., `console_log!` or `web_sys::console`) appears in the browser
  dev tools.

### Debugging the C bridge

- Build in debug mode: `cmake -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build`.
- The doctest suite prints failures with file:line; run a single test case with
  `./build/examples/cpp/solid_euclid_tests -tc=<name>`.
- Memory errors: the C bridge uses raw pointers across the ABI; a missing `triangular_mesh_free`
  call leaks, a double-free is UB. The doctest tests always free.

### Common pitfalls

| Pitfall                           | Fix                                                                                                                                |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Forgotten `lib.rs` registration   | A new module not declared `pub mod` in `lib.rs` does not compile                                                                   |
| Editing `pkg/` directly           | Never edit `pkg/`; change `web_api.rs` and run `npm run build:wasm`                                                                |
| Forgotten vite input              | A new `examples/web/<name>/` page needs an entry in `rollupOptions.input` in `vite.config.ts`                                      |
| Forgotten CMake test registration | A new `examples/cpp/tests/test-*.cpp` needs to be added to `add_executable(solid_euclid_tests …)` in `examples/cpp/CMakeLists.txt` |
| Precision mix-ups                 | f64 in the kernel, f32 at the mesh boundary; widening the mesh or narrowing the model is a bug                                     |
| Winding                           | Polygon normals come from `(p1-p0) × (p2-p0)`; flipped winding flips the facing                                                    |
| Header out of sync                | `include/solid_euclid.h` mirrors `c_api.rs` one to one — keep them in sync                                                         |

## Deployment

There is no CI/CD pipeline or deployment automation in the repository. Distribution is manual:

| Artifact                    | How produced                        | Consumed by                                                            |
| --------------------------- | ----------------------------------- | ---------------------------------------------------------------------- |
| Rust crate (`solid_euclid`) | `cargo build --release`             | Rust consumers via `rlib`; CMake via `staticlib`                       |
| C static library            | `cargo build --release` (via CMake) | C/C++ consumers linking `libsolid_euclid.a` + `include/solid_euclid.h` |
| WASM package (`pkg/`)       | `npm run build:wasm` (wasm-pack)    | TypeScript/JavaScript consumers importing from `@solid-euclid`         |
| Web examples (`dist/`)      | `npm run build` (vite)              | Static hosting (any web server)                                        |
| C++ example binary          | `cmake --build build`               | Local execution                                                        |

The `pkg/` directory and `dist/` are git-ignored build outputs. The C header
(`include/solid_euclid.h`) is the stable ABI surface for C++ consumers.

## Project layout

```
.
├── solid_euclid/                 # The Rust crate (edition 2024)
│   ├── Cargo.toml                # crate-type: cdylib + staticlib + rlib
│   └── src/                      # All geometry logic (see architecture.md)
├── include/
│   └── solid_euclid.h            # C header (mirrors c_api.rs)
├── examples/
│   ├── web/                      # TypeScript/three.js example pages (vite)
│   │   ├── __tests__/            # vitest tests (one file per geometry type)
│   │   ├── platonic_solid/       # example page
│   │   ├── sphere/               # example page
│   │   ├── sweeps/               # example page (torus, capsule, cylinder, frustum)
│   │   ├── extrude/              # example page
│   │   └── util/three-util.ts    # shared three.js helpers
│   └── cpp/                      # C++ example + doctest tests (CMake)
│       ├── main.cpp
│       └── tests/                # test-*.cpp + doctest.h
├── pkg/                          # wasm-pack output (git-ignored)
├── dist/                         # vite build output (git-ignored)
├── build/                        # CMake build directory (git-ignored)
├── CMakeLists.txt                # Root CMake: builds Rust staticlib + C++ examples
├── Cargo.toml                    # Workspace root
├── package.json                  # npm scripts
├── vite.config.ts                # Vite config (root: examples/web, alias @solid-euclid → pkg)
├── vitest.config.ts              # vitest config
├── tsconfig.json                 # TypeScript config (strict, bundler mode)
├── eslint.config.js              # ESLint config
├── .prettierrc / .prettierignore # Prettier config
├── .clang-format / .clang-tidy   # C++ formatting and static analysis
├── AGENTS.md                     # AI assistant instructions (canonical)
├── .github/copilot-instructions.md # GitHub Copilot instructions (mirrors AGENTS.md)
├── .agents/                      # Digital brain
│   ├── README.md                 # Index
│   ├── architecture.md           # This project's codebase map (you are here)
│   ├── development-guide.md      # Build, test, debug, deploy commands (you are here)
│   ├── skills/                   # On-demand domain knowledge
│   ├── decisions/                # ADRs
│   ├── backlog/                  # Open work
│   ├── bug-analysis/             # Bug analyses (historical)
│   ├── feature-analysis/         # Feature analyses (historical)
│   ├── refactoring-analysis/     # Refactoring analyses (historical)
│   └── scripts/                  # Utility scripts (JavaScript)
├── .internal/                    # Private working notes
└── .temp/                        # Scratch space (git-ignored)
```
