add_executable(solid_euclid_cpp_example main.cpp)

target_link_libraries(solid_euclid_cpp_example PRIVATE solid_euclid)

# Link system libraries required by Rust
if(APPLE)
    target_link_libraries(solid_euclid_cpp_example PRIVATE
        "-framework Security"
        "-framework CoreFoundation"
    )
elseif(UNIX)
    target_link_libraries(solid_euclid_cpp_example PRIVATE
        pthread
        dl
        m
    )
elseif(WIN32)
    target_link_libraries(solid_euclid_cpp_example PRIVATE
        ws2_32
        userenv
        bcrypt
        ntdll
    )
endif()

# ============================================================================
# Unit Tests
# ============================================================================

# Create test executable
add_executable(solid_euclid_tests
    tests/test-main.cpp
    tests/test-platonic-solid.cpp
    tests/test-sphere.cpp
    tests/test-sweeps.cpp
)

target_link_libraries(solid_euclid_tests PRIVATE solid_euclid)

# Add include path for doctest
target_include_directories(solid_euclid_tests PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/tests)

# Link system libraries required by Rust
if(APPLE)
    target_link_libraries(solid_euclid_tests PRIVATE
        "-framework Security"
        "-framework CoreFoundation"
    )
elseif(UNIX)
    target_link_libraries(solid_euclid_tests PRIVATE
        pthread
        dl
        m
    )
elseif(WIN32)
    target_link_libraries(solid_euclid_tests PRIVATE
        ws2_32
        userenv
        bcrypt
        ntdll
    )
endif()

# Enable testing
enable_testing()
add_test(NAME solid_euclid_tests COMMAND solid_euclid_tests)
