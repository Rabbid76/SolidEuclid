#include "doctest.h"
#include "test-utils.h"

using test_utils::MeshGuard;
using test_utils::triangle_count;
using test_utils::vertex_count;

// =============================================================================
// Torus C API Tests
// =============================================================================

TEST_SUITE("Torus C API") {

    TEST_CASE("create_torus_c returns error for null JSON") {
        MeshGuard mesh{create_torus_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_torus_c returns error for invalid JSON") {
        MeshGuard mesh{create_torus_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("Torus with uniform material") {
        constexpr auto json = R"({"major_radius":2.0,"minor_radius":0.5,"major_segments":36,"minor_segments":18,"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_torus_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) > 0);
        CHECK(triangle_count(entry.geometry) > 0);
    }

    TEST_CASE("Torus with smaller segments") {
        constexpr auto json = R"({"major_radius":1.0,"minor_radius":0.25,"major_segments":16,"minor_segments":8,"material":{"Uniform":"ring"}})";
        MeshGuard mesh{create_torus_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& geom = mesh.entry(0).geometry;
        CHECK(triangle_count(geom) > 0);
    }

    TEST_CASE("Torus geometry data arrays are consistent") {
        constexpr auto json = R"({"major_radius":2.0,"minor_radius":0.5,"major_segments":24,"minor_segments":12,"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_torus_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(geom.vertices_len == geom.normals_len);
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
        CHECK(geom.vertices != nullptr);
        CHECK(geom.normals != nullptr);
        CHECK(geom.uvs != nullptr);
        CHECK(geom.indices != nullptr);
    }

    TEST_CASE("Torus indices are within vertex bounds") {
        constexpr auto json = R"({"major_radius":2.0,"minor_radius":0.5,"major_segments":16,"minor_segments":8,"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_torus_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }
}

// =============================================================================
// Capsule C API Tests
// =============================================================================

TEST_SUITE("Capsule C API") {

    TEST_CASE("create_capsule_c returns error for null JSON") {
        MeshGuard mesh{create_capsule_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_capsule_c returns error for invalid JSON") {
        MeshGuard mesh{create_capsule_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("Capsule with uniform material") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":24,"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_capsule_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) > 0);
        CHECK(triangle_count(entry.geometry) > 0);
    }

    TEST_CASE("Capsule with array material creates 3 geometries") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":24,"material":{"Array":["bottom","body","top"]}})";
        MeshGuard mesh{create_capsule_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 3);  // 3 different materials for 3 regions
    }

    TEST_CASE("Capsule with default material") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":16,"material":"Default"})";
        MeshGuard mesh{create_capsule_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 3);  // 3 faces with unique default materials
    }

    TEST_CASE("Capsule geometry data arrays are consistent") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":16,"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_capsule_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(geom.vertices_len == geom.normals_len);
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
        CHECK(geom.vertices != nullptr);
        CHECK(geom.normals != nullptr);
        CHECK(geom.uvs != nullptr);
        CHECK(geom.indices != nullptr);
    }

    TEST_CASE("Capsule indices are within vertex bounds") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":16,"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_capsule_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }
}

// =============================================================================
// Cylinder C API Tests
// =============================================================================

TEST_SUITE("Cylinder C API") {

    TEST_CASE("create_cylinder_c returns error for null JSON") {
        MeshGuard mesh{create_cylinder_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_cylinder_c returns error for invalid JSON") {
        MeshGuard mesh{create_cylinder_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("Cylinder with sharp edges and uniform material") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":24,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) > 0);
        CHECK(triangle_count(entry.geometry) > 0);
    }

    TEST_CASE("Cylinder with chamfered edges") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":24,"bottom_edge":{"Chamfer":{"distance":0.1}},"top_edge":{"Chamfer":{"distance":0.1}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);
        CHECK(triangle_count(mesh.entry(0).geometry) > 0);
    }

    TEST_CASE("Cylinder with filleted edges") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":24,"bottom_edge":{"Fillet":{"radius":0.1}},"top_edge":{"Fillet":{"radius":0.1}},"material":{"Uniform":"metal"}})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);
        CHECK(std::string_view{mesh.entry(0).material_id} == "metal");
    }

    TEST_CASE("Cylinder with array material creates 5 geometries") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":24,"bottom_edge":{"Chamfer":{"distance":0.1}},"top_edge":{"Chamfer":{"distance":0.1}},"material":{"Array":["bottom_cap","bottom_edge","body","top_edge","top_cap"]}})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 5);  // 5 different materials for 5 regions
    }

    TEST_CASE("Cylinder with default material") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":"Default"})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        // With sharp edges, edge regions are empty, so 3 non-empty regions
        CHECK(mesh.entry_count() == 3);
    }

    TEST_CASE("Cylinder geometry data arrays are consistent") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(geom.vertices_len == geom.normals_len);
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
        CHECK(geom.vertices != nullptr);
        CHECK(geom.normals != nullptr);
        CHECK(geom.uvs != nullptr);
        CHECK(geom.indices != nullptr);
    }

    TEST_CASE("Cylinder indices are within vertex bounds") {
        constexpr auto json = R"({"radius":0.5,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_cylinder_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }
}

// =============================================================================
// Conical Frustum C API Tests
// =============================================================================

TEST_SUITE("Conical Frustum C API") {

    TEST_CASE("create_conical_frustum_c returns error for null JSON") {
        MeshGuard mesh{create_conical_frustum_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_conical_frustum_c returns error for invalid JSON") {
        MeshGuard mesh{create_conical_frustum_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("Conical frustum with sharp edges and uniform material") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":24,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) > 0);
        CHECK(triangle_count(entry.geometry) > 0);
    }

    TEST_CASE("Conical frustum with chamfered edges") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":24,"bottom_edge":{"Chamfer":{"distance":0.1}},"top_edge":{"Chamfer":{"distance":0.05}},"material":{"Uniform":"default"}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);
        CHECK(triangle_count(mesh.entry(0).geometry) > 0);
    }

    TEST_CASE("Cone (top_radius = 0)") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.0,"height":2.0,"segments":24,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);
        CHECK(triangle_count(mesh.entry(0).geometry) > 0);
    }

    TEST_CASE("Conical frustum with array material creates 5 geometries") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":24,"bottom_edge":{"Chamfer":{"distance":0.1}},"top_edge":{"Chamfer":{"distance":0.05}},"material":{"Array":["bottom_cap","bottom_edge","body","top_edge","top_cap"]}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 5);  // 5 different materials for 5 regions
    }

    TEST_CASE("Conical frustum with default material") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":"Default"})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        // With sharp edges, edge regions are empty, so 3 non-empty regions
        CHECK(mesh.entry_count() == 3);
    }

    TEST_CASE("Conical frustum geometry data arrays are consistent") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(geom.vertices_len == geom.normals_len);
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
        CHECK(geom.vertices != nullptr);
        CHECK(geom.normals != nullptr);
        CHECK(geom.uvs != nullptr);
        CHECK(geom.indices != nullptr);
    }

    TEST_CASE("Conical frustum indices are within vertex bounds") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":{"Uniform":"default"}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }

    TEST_CASE("Map material creates expected geometries") {
        constexpr auto json = R"({"bottom_radius":0.5,"top_radius":0.25,"height":2.0,"segments":16,"bottom_edge":"Edge","top_edge":"Edge","material":{"Map":{"default":"base","face_map":{"0":"special","4":"special"}}}})";
        MeshGuard mesh{create_conical_frustum_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        CHECK(mesh.entry_count() == 2);  // "base" and "special"
    }
}
