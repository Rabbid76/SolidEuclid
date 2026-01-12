#include "doctest.h"
#include "test-utils.h"

using test_utils::MeshGuard;
using test_utils::StringGuard;
using test_utils::triangle_count;
using test_utils::vertex_count;

TEST_SUITE(
    "Sphere C API"
) {
    TEST_CASE("create_sphere_c returns error for null JSON") {
        MeshGuard mesh{create_sphere_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_sphere_c returns error for invalid JSON") {
        MeshGuard mesh{create_sphere_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("Slices sphere with uniform material") {
        constexpr auto json =
            R"({"geometry":{"Slices":{"slices":8,"segments":16}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) > 0);
        CHECK(triangle_count(entry.geometry) > 0);
    }

    TEST_CASE("Slices sphere with default material creates 1 geometry") {
        constexpr auto json =
            R"({"geometry":{"Slices":{"slices":8,"segments":16}},"material":"Default"})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 1);  // Slices sphere has 1 face
    }

    TEST_CASE("Cube sphere with uniform material") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& geom = mesh.entry(0).geometry;
        // 6 faces × 4×4 quads × 2 triangles = 192 triangles
        CHECK(triangle_count(geom) == 6 * 4 * 4 * 2);
    }

    TEST_CASE("Cube sphere with array material creates 6 geometries") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Array":["mat0","mat1","mat2","mat3","mat4","mat5"]}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 6);

        // Each face should have segments² × 2 triangles
        for (std::size_t i = 0; i < mesh.entry_count(); ++i) {
            const auto& geom = mesh.entry(i).geometry;
            CHECK(triangle_count(geom) == 4 * 4 * 2);  // 32 triangles per face
        }
    }

    TEST_CASE("Cube sphere with default material creates 6 geometries") {
        constexpr auto json = R"({"geometry":{"Cube":{"segments":4}},"material":"Default"})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 6);  // 6 faces with unique default materials
    }

    TEST_CASE("Cyclic material array groups faces correctly") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Array":["mat0","mat1","mat2","mat0","mat1","mat2"]}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 3);  // 3 unique materials
    }

    TEST_CASE("Map material creates expected geometries") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Map":{"default":"base","face_map":{"0":"special","2":"special"}}}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 2);  // "base" and "special"
    }

    TEST_CASE("Geometry data arrays are consistent for slices sphere") {
        constexpr auto json =
            R"({"geometry":{"Slices":{"slices":8,"segments":16}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_sphere_c(json)};

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

    TEST_CASE("Geometry data arrays are consistent for cube sphere") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(geom.vertices_len == geom.normals_len);
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
    }

    TEST_CASE("Indices are within vertex bounds for slices sphere") {
        constexpr auto json =
            R"({"geometry":{"Slices":{"slices":8,"segments":16}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }

    TEST_CASE("Indices are within vertex bounds for cube sphere") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }

    TEST_CASE("Total triangles across all geometries is correct for cube sphere") {
        constexpr auto json =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Array":["m0","m1","m2","m3","m4","m5"]}})";
        MeshGuard mesh{create_sphere_c(json)};

        REQUIRE_FALSE(mesh.has_error());

        std::size_t total_triangles = 0;
        for (std::size_t i = 0; i < mesh.entry_count(); ++i) {
            total_triangles += triangle_count(mesh.entry(i).geometry);
        }
        // 6 faces × 4×4 quads × 2 triangles = 192 triangles
        CHECK(total_triangles == 6 * 4 * 4 * 2);
    }

    TEST_CASE("Different segment counts produce different triangle counts") {
        constexpr auto json_4 =
            R"({"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}})";
        constexpr auto json_8 =
            R"({"geometry":{"Cube":{"segments":8}},"material":{"Uniform":"default"}})";

        MeshGuard mesh_4{create_sphere_c(json_4)};
        MeshGuard mesh_8{create_sphere_c(json_8)};

        REQUIRE_FALSE(mesh_4.has_error());
        REQUIRE_FALSE(mesh_8.has_error());

        const auto triangles_4 = triangle_count(mesh_4.entry(0).geometry);
        const auto triangles_8 = triangle_count(mesh_8.entry(0).geometry);

        // segments=8 should have 4× as many triangles as segments=4
        CHECK(triangles_8 == triangles_4 * 4);
    }
}

TEST_SUITE(
    "Sphere Types API"
) {
    TEST_CASE("get_sphere_types_c returns valid JSON array") {
        StringGuard types{get_sphere_types_c()};

        REQUIRE_FALSE(types.is_null());

        const auto json = types.view();
        CHECK(json.contains("Slices"));
        CHECK(json.contains("Cube"));
    }

    TEST_CASE("get_sphere_types_c returns proper JSON format") {
        StringGuard types{get_sphere_types_c()};

        REQUIRE_FALSE(types.is_null());

        const auto json = types.view();
        CHECK(json.starts_with('['));
        CHECK(json.ends_with(']'));
    }
}
