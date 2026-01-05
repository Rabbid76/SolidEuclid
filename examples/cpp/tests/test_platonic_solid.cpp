#include "doctest.h"
#include <solid_euclid.h>

#include <array>
#include <cstring>
#include <memory>
#include <span>
#include <string>
#include <string_view>

namespace {

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
[[nodiscard]] constexpr auto vertex_count(const CTriangularGeometry& geom) noexcept {
    return geom.vertices_len / 3;
}

/// Helper to get triangle count from geometry.
[[nodiscard]] constexpr auto triangle_count(const CTriangularGeometry& geom) noexcept {
    return geom.indices_len / 3;
}

} // anonymous namespace

TEST_SUITE("Platonic Solid C API") {

    TEST_CASE("create_platonic_solid_c returns error for null JSON") {
        MeshGuard mesh{create_platonic_solid_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_platonic_solid_c returns error for invalid JSON") {
        MeshGuard mesh{create_platonic_solid_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("create_platonic_solid_c returns error for unknown geometry") {
        MeshGuard mesh{create_platonic_solid_c(
            R"({"geometry":"InvalidSolid","material":{"Uniform":"default"}})")};

        CHECK(mesh.has_error());
    }

    TEST_CASE("Tetrahedron with uniform material") {
        constexpr auto json = R"({"geometry":"Tetrahedron","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) == 12);    // 4 faces × 3 vertices
        CHECK(triangle_count(entry.geometry) == 4);   // 4 triangular faces
    }

    TEST_CASE("Hexahedron with uniform material") {
        constexpr auto json = R"({"geometry":"Hexahedron","material":{"Uniform":"cube_mat"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) == 24);    // 6 faces × 4 vertices
        CHECK(triangle_count(geom) == 12);  // 6 quad faces = 12 triangles
    }

    TEST_CASE("Hexahedron with array material creates 6 geometries") {
        constexpr auto json = R"({"geometry":"Hexahedron","material":{"Array":["mat0","mat1","mat2","mat3","mat4","mat5"]}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 6);  // 6 different materials for 6 faces
    }

    TEST_CASE("Tetrahedron with default material creates 4 geometries") {
        constexpr auto json = R"({"geometry":"Tetrahedron","material":"Default"})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 4);  // 4 faces with unique default materials
    }

    TEST_CASE("Octahedron geometry") {
        constexpr auto json = R"({"geometry":"Octahedron","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) == 24);   // 8 faces × 3 vertices
        CHECK(triangle_count(geom) == 8);  // 8 triangular faces
    }

    TEST_CASE("Dodecahedron geometry") {
        constexpr auto json = R"({"geometry":"Dodecahedron","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) == 60);    // 12 faces × 5 vertices
        CHECK(triangle_count(geom) == 36);  // 12 pentagonal faces = 36 triangles
    }

    TEST_CASE("Icosahedron geometry") {
        constexpr auto json = R"({"geometry":"Icosahedron","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) == 60);    // 20 faces × 3 vertices
        CHECK(triangle_count(geom) == 20);  // 20 triangular faces
    }

    TEST_CASE("Map material creates expected geometries") {
        constexpr auto json = R"({"geometry":"Tetrahedron","material":{"Map":{"default":"base","face_map":{"0":"special","2":"special"}}}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 2);  // "base" and "special"
    }

    TEST_CASE("Geometry data arrays are consistent") {
        constexpr auto json = R"({"geometry":"Hexahedron","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        // vertices and normals should have same length (3 components each)
        CHECK(geom.vertices_len == geom.normals_len);
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
        // All arrays should be non-empty
        CHECK(geom.vertices != nullptr);
        CHECK(geom.normals != nullptr);
        CHECK(geom.uvs != nullptr);
        CHECK(geom.indices != nullptr);
    }

    TEST_CASE("Indices are within vertex bounds") {
        constexpr auto json = R"({"geometry":"Icosahedron","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }

    TEST_CASE("Cyclic material array groups faces correctly") {
        // Using 3 materials cyclically for 6 faces should result in 3 geometries
        constexpr auto json = R"({"geometry":"Hexahedron","material":{"Array":["mat0","mat1","mat2","mat0","mat1","mat2"]}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 3);  // 3 unique materials
    }

    TEST_CASE("Total triangles across all geometries is correct for hexahedron") {
        constexpr auto json = R"({"geometry":"Hexahedron","material":{"Array":["m0","m1","m2","m3","m4","m5"]}})";
        MeshGuard mesh{create_platonic_solid_c(json)};

        REQUIRE_FALSE(mesh.has_error());

        std::size_t total_triangles = 0;
        for (std::size_t i = 0; i < mesh.entry_count(); ++i) {
            total_triangles += triangle_count(mesh.entry(i).geometry);
        }
        CHECK(total_triangles == 12);  // Hexahedron has 12 triangles total
    }

}

TEST_SUITE("Platonic Solid Types API") {

    TEST_CASE("get_platonic_solid_types_c returns valid JSON array") {
        StringGuard types{get_platonic_solid_types_c()};

        REQUIRE_FALSE(types.is_null());

        const auto json = types.view();
        CHECK(json.contains("Tetrahedron"));
        CHECK(json.contains("Hexahedron"));
        CHECK(json.contains("Octahedron"));
        CHECK(json.contains("Dodecahedron"));
        CHECK(json.contains("Icosahedron"));
    }

    TEST_CASE("get_platonic_solid_types_c returns proper JSON format") {
        StringGuard types{get_platonic_solid_types_c()};

        REQUIRE_FALSE(types.is_null());

        const auto json = types.view();
        CHECK(json.starts_with('['));
        CHECK(json.ends_with(']'));
    }

}
