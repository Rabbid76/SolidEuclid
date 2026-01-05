# Copilot Instructions for SolidEuclid

## Project Overview

SolidEuclid is a Rust-based geometry library that exposes its functionality to C++ (via C API) and Web/TypeScript (via WebAssembly).

## Naming Conventions

### Rust (Library)

- **Source Code & Directory Structure**: Follow standard Rust naming conventions.
  - Files and Directories: `snake_case` (e.g., `geometry_model.rs`, `platonic_solid.rs`)
  - Functions and Methods: `snake_case`
  - Types (Structs, Enums, Traits): `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE`
  - Macros: `snake_case!`

### TypeScript / Web (Examples & Tests)

- **Filenames**: `kebab-case` (e.g., `platonic-solid.test.ts`, `main.ts`)
- **Code**:
  - Variables and Functions: `camelCase`
  - Classes and Interfaces: `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE`

### C++ (Examples & Tests)

- **Filenames**: `kebab-case` (e.g., `test-platonic-solid.cpp`, `main.cpp`)
- **Code**:
  - Types (Classes, Structs): `PascalCase` (e.g., `MeshGuard`, `StringGuard`)
  - Variables, Functions, and Methods: `snake_case` (e.g., `create_platonic_solid_c`, `vertex_count`)
  - Constants: `SCREAMING_SNAKE_CASE`

## Testing Strategy

### Feature Tests

- **Primary Location**: Feature logic should be primarily tested in **Rust**.
- Ensure core geometry logic is verified within the Rust crate's test suite.

### TypeScript Tests

- **Framework**: `vitest`
- **Scope**: Mainly for testing the **Web API** (Wasm bindings).
- **Location**: `examples/web/__tests__`

### C++ Tests

- **Framework**: `doctest`
- **Scope**: Mainly for testing the **C API**.
- **Location**: `examples/cpp/tests`

## Directory Structure Highlights

- `solid_euclid/`: Rust crate source code.
- `examples/web/`: TypeScript/Web examples and tests.
- `examples/cpp/`: C++ examples and tests.
- `include/`: C header files.
