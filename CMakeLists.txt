cmake_minimum_required(VERSION 3.16)
project(SolidEuclid VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Path to the Rust library output
set(RUST_TARGET_DIR ${CMAKE_SOURCE_DIR}/target/release)
set(SOLID_EUCLID_INCLUDE_DIR ${CMAKE_SOURCE_DIR}/include)

# Platform-specific library name
if(WIN32)
    set(SOLID_EUCLID_LIB_NAME "solid_euclid.lib")
else()
    set(SOLID_EUCLID_LIB_NAME "libsolid_euclid.a")
endif()

# Build the Rust library
add_custom_target(solid_euclid_rust
    COMMAND cargo build --release --manifest-path ${CMAKE_SOURCE_DIR}/solid_euclid/Cargo.toml
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Building Rust library..."
)

# Create an imported library target for the Rust library
add_library(solid_euclid STATIC IMPORTED)
set_target_properties(solid_euclid PROPERTIES
    IMPORTED_LOCATION ${RUST_TARGET_DIR}/${SOLID_EUCLID_LIB_NAME}
    INTERFACE_INCLUDE_DIRECTORIES ${SOLID_EUCLID_INCLUDE_DIR}
)
add_dependencies(solid_euclid solid_euclid_rust)

# Add examples
add_subdirectory(examples/cpp)
