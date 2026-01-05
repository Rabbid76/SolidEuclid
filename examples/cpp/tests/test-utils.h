#pragma once

#include <solid_euclid.h>

#include <cstring>
#include <span>
#include <string_view>

namespace test_utils {

/// RAII wrapper for CTriangularMesh that ensures proper cleanup.
class MeshGuard {
public:
    explicit MeshGuard(CTriangularMesh mesh) : mesh_{mesh} {}
    ~MeshGuard() { triangular_mesh_free(mesh_); }

    MeshGuard(const MeshGuard&) = delete;
    MeshGuard& operator=(const MeshGuard&) = delete;
    MeshGuard(MeshGuard&&) = delete;
    MeshGuard& operator=(MeshGuard&&) = delete;

    [[nodiscard]] auto get() const noexcept -> const CTriangularMesh& { return mesh_; }
    [[nodiscard]] auto has_error() const noexcept -> bool { return mesh_.error != nullptr; }
    [[nodiscard]] auto error_message() const -> std::string_view {
        return mesh_.error ? mesh_.error : "";
    }
    [[nodiscard]] auto entry_count() const noexcept -> std::size_t { return mesh_.entries_len; }
    [[nodiscard]] auto entry(std::size_t index) const -> const CTriangularMeshEntry& {
        return mesh_.entries[index];
    }

private:
    CTriangularMesh mesh_;
};

/// RAII wrapper for C strings that ensures proper cleanup.
class StringGuard {
public:
    explicit StringGuard(char* str) : str_{str} {}
    ~StringGuard() { string_free_c(str_); }

    StringGuard(const StringGuard&) = delete;
    StringGuard& operator=(const StringGuard&) = delete;
    StringGuard(StringGuard&&) = delete;
    StringGuard& operator=(StringGuard&&) = delete;

    [[nodiscard]] auto get() const noexcept -> const char* { return str_; }
    [[nodiscard]] auto view() const -> std::string_view { return str_ ? str_ : ""; }
    [[nodiscard]] auto is_null() const noexcept -> bool { return str_ == nullptr; }

private:
    char* str_;
};

/// Helper to get vertex count from geometry.
[[nodiscard]] constexpr auto vertex_count(const CTriangularGeometry& geom) noexcept -> std::size_t {
    return geom.vertices_len / 3;
}

/// Helper to get triangle count from geometry.
[[nodiscard]] constexpr auto triangle_count(const CTriangularGeometry& geom) noexcept -> std::size_t {
    return geom.indices_len / 3;
}

} // namespace test_utils
