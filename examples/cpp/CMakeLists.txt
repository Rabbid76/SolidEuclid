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
