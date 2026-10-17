#include "doctest.h"
#include "test-utils.h"
#include <cfloat>
#include <cmath>
#include <span>
#include <string>

using test_utils::MeshGuard;
using test_utils::triangle_count;
using test_utils::vertex_count;

namespace {

    // The hexahedron spans ±1/sqrt(3) on every axis
    constexpr auto L = 0.5773502691896258f;  // 1/sqrt(3)

    // Builds a transform JSON wrapping a hexahedron with the given transformation
    auto hexahedron_transform_json(
        std::string_view transformation
    ) -> std::string {
        return std::string{R"({"transformation":)"} +
               std::string{transformation} +
               R"(,"model":{"PlatonicSolid":{"geometry":"Hexahedron","material":{"Uniform":"default"}}}})";
    }

    struct Bounds {
        float min[3];
        float max[3];
    };

    // Computes the per-axis min/max of the vertex positions
    auto compute_bounds(
        const CTriangularGeometry& geom
    ) -> Bounds {
        const auto vertices = std::span{geom.vertices, geom.vertices_len};
        Bounds bounds{{FLT_MAX, FLT_MAX, FLT_MAX}, {FLT_MIN, FLT_MIN, FLT_MIN}};
        for (std::size_t i = 0; i < vertices.size(); i += 3) {
            for (std::size_t axis = 0; axis < 3; ++axis) {
                bounds.min[axis] = std::fmin(bounds.min[axis], vertices[i + axis]);
                bounds.max[axis] = std::fmax(bounds.max[axis], vertices[i + axis]);
            }
        }
        return bounds;
    }

    constexpr auto EPSILON = 0.0001f;

    // Asserts that every triangle's geometric normal points away from the origin,
    // i.e. the winding is outward-facing for a model centered at the origin.
    void check_outward_winding(
        const CTriangularGeometry& geom
    ) {
        const auto vertices = std::span{geom.vertices, geom.vertices_len};
        const auto indices = std::span{geom.indices, geom.indices_len};
        for (std::size_t t = 0; t < indices.size(); t += 3) {
            const auto a = indices[t] * 3;
            const auto b = indices[t + 1] * 3;
            const auto c = indices[t + 2] * 3;
            const auto ux = vertices[b] - vertices[a];
            const auto uy = vertices[b + 1] - vertices[a + 1];
            const auto uz = vertices[b + 2] - vertices[a + 2];
            const auto wx = vertices[c] - vertices[a];
            const auto wy = vertices[c + 1] - vertices[a + 1];
            const auto wz = vertices[c + 2] - vertices[a + 2];
            const auto nx = uy * wz - uz * wy;
            const auto ny = uz * wx - ux * wz;
            const auto nz = ux * wy - uy * wx;
            const auto cx = (vertices[a] + vertices[b] + vertices[c]) / 3.0f;
            const auto cy = (vertices[a + 1] + vertices[b + 1] + vertices[c + 1]) / 3.0f;
            const auto cz = (vertices[a + 2] + vertices[b + 2] + vertices[c + 2]) / 3.0f;
            const auto dot = nx * cx + ny * cy + nz * cz;
            CHECK(dot > 0.0f);
        }
    }

}  // namespace

// =============================================================================
// Transform C API Tests
// =============================================================================

TEST_SUITE(
    "Transform C API"
) {
    TEST_CASE("create_transform_c returns error for null JSON") {
        MeshGuard mesh{create_transform_c(nullptr)};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Null"));
        CHECK(mesh.entry_count() == 0);
    }

    TEST_CASE("create_transform_c returns error for invalid JSON") {
        MeshGuard mesh{create_transform_c("{invalid json}")};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("create_transform_c returns error for unknown transformation variant") {
        MeshGuard mesh{create_transform_c(hexahedron_transform_json(R"({"Bogus":1.0})").c_str())};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("create_transform_c returns error for unknown model variant") {
        MeshGuard mesh{create_transform_c(
            R"({"transformation":{"Translation":[0.0,0.0,0.0]},"model":{"Bogus":{}}})"
        )};

        CHECK(mesh.has_error());
        CHECK(mesh.error_message().contains("Failed to parse"));
    }

    TEST_CASE("Translated hexahedron with uniform material") {
        MeshGuard mesh{create_transform_c(
            hexahedron_transform_json(R"({"Translation":[1.0,2.0,3.0]})").c_str()
        )};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& entry = mesh.entry(0);
        CHECK(std::string_view{entry.material_id} == "default");
        CHECK(vertex_count(entry.geometry) == 24);    // 6 faces * 4 vertices
        CHECK(triangle_count(entry.geometry) == 12);  // 6 faces * 2 triangles
    }

    TEST_CASE("Translation shifts the vertex bounds") {
        MeshGuard mesh{create_transform_c(
            hexahedron_transform_json(R"({"Translation":[1.0,2.0,3.0]})").c_str()
        )};

        REQUIRE_FALSE(mesh.has_error());
        const auto bounds = compute_bounds(mesh.entry(0).geometry);

        CHECK(std::abs(bounds.min[0] - (1.0f - L)) < EPSILON);
        CHECK(std::abs(bounds.max[0] - (1.0f + L)) < EPSILON);
        CHECK(std::abs(bounds.min[1] - (2.0f - L)) < EPSILON);
        CHECK(std::abs(bounds.max[1] - (2.0f + L)) < EPSILON);
        CHECK(std::abs(bounds.min[2] - (3.0f - L)) < EPSILON);
        CHECK(std::abs(bounds.max[2] - (3.0f + L)) < EPSILON);
    }

    TEST_CASE("Uniform scale scales the vertex bounds") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"UniformScale":2.0})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        const auto bounds = compute_bounds(mesh.entry(0).geometry);

        for (std::size_t axis = 0; axis < 3; ++axis) {
            CHECK(std::abs(bounds.min[axis] - -2.0f * L) < EPSILON);
            CHECK(std::abs(bounds.max[axis] - 2.0f * L) < EPSILON);
        }
    }

    TEST_CASE("Non-uniform scale scales each axis independently") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"ScaleXYZ":[2.0,3.0,4.0]})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        const auto bounds = compute_bounds(mesh.entry(0).geometry);
        const float scale[3] = {2.0f, 3.0f, 4.0f};

        for (std::size_t axis = 0; axis < 3; ++axis) {
            CHECK(std::abs(bounds.min[axis] - -scale[axis] * L) < EPSILON);
            CHECK(std::abs(bounds.max[axis] - scale[axis] * L) < EPSILON);
        }
    }

    TEST_CASE("Rotation around Z rotates the vertices") {
        MeshGuard mesh{create_transform_c(
            hexahedron_transform_json(R"({"RotationZ":1.5707963267948966})").c_str()
        )};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        // First vertex of the hexahedron is (-L, -L, L); rotated 90° around Z
        // it becomes (L, -L, L)
        CHECK(std::abs(geom.vertices[0] - L) < EPSILON);
        CHECK(std::abs(geom.vertices[1] - -L) < EPSILON);
        CHECK(std::abs(geom.vertices[2] - L) < EPSILON);
    }

    TEST_CASE("Quaternion rotation rotates the vertices") {
        // 90° rotation around Z as a quaternion (x, y, z, w)
        MeshGuard mesh{create_transform_c(
            hexahedron_transform_json(
                R"({"Quaternion":[0.0,0.0,0.7071067811865476,0.7071067811865476]})"
            )
                .c_str()
        )};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(std::abs(geom.vertices[0] - L) < EPSILON);
        CHECK(std::abs(geom.vertices[1] - -L) < EPSILON);
        CHECK(std::abs(geom.vertices[2] - L) < EPSILON);
    }

    TEST_CASE("Matrix transformation applies the matrix") {
        // Uniform scale by 2 as a flat column-major 4x4 matrix
        MeshGuard mesh{create_transform_c(
            hexahedron_transform_json(
                R"({"Matrix":[2.0,0.0,0.0,0.0,0.0,2.0,0.0,0.0,0.0,0.0,2.0,0.0,0.0,0.0,0.0,1.0]})"
            )
                .c_str()
        )};

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(std::abs(geom.vertices[0] - -2.0f * L) < EPSILON);
        CHECK(std::abs(geom.vertices[1] - -2.0f * L) < EPSILON);
        CHECK(std::abs(geom.vertices[2] - 2.0f * L) < EPSILON);
    }

    TEST_CASE("Nested transforms apply inside-out") {
        // Scale X by 2 first, then translate Z by 5
        constexpr auto json =
            R"({"transformation":{"Translation":[0.0,0.0,5.0]},"model":{"Transform":{"transformation":{"ScaleXYZ":[2.0,1.0,1.0]},"model":{"PlatonicSolid":{"geometry":"Hexahedron","material":{"Uniform":"default"}}}}}})";
        MeshGuard mesh{create_transform_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) == 24);
        CHECK(triangle_count(geom) == 12);

        const auto bounds = compute_bounds(geom);
        CHECK(std::abs(bounds.min[0] - -2.0f * L) < EPSILON);
        CHECK(std::abs(bounds.max[0] - 2.0f * L) < EPSILON);
        CHECK(std::abs(bounds.min[2] - (5.0f - L)) < EPSILON);
        CHECK(std::abs(bounds.max[2] - (5.0f + L)) < EPSILON);
    }

    TEST_CASE("Positive scale keeps outward winding") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"UniformScale":2.0})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        check_outward_winding(mesh.entry(0).geometry);
    }

    TEST_CASE("Negative scale flips winding so faces stay outward") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"UniformScale":-1.0})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        check_outward_winding(mesh.entry(0).geometry);
    }

    TEST_CASE("Transformed torus base model") {
        constexpr auto json =
            R"({"transformation":{"Translation":[0.0,0.0,3.0]},"model":{"Torus":{"major_radius":2.0,"minor_radius":0.5,"major_segments":24,"minor_segments":12,"material":{"Uniform":"default"}}}})";
        MeshGuard mesh{create_transform_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) > 0);
        CHECK(triangle_count(geom) > 0);

        // The torus lies in the XY-plane; translated by 3 along Z
        const auto bounds = compute_bounds(geom);
        CHECK(std::abs(bounds.min[2] - 2.5f) < EPSILON);
        CHECK(std::abs(bounds.max[2] - 3.5f) < EPSILON);
    }

    TEST_CASE("Transformed tetrahedron base model") {
        constexpr auto json =
            R"({"transformation":{"UniformScale":2.0},"model":{"PlatonicSolid":{"geometry":"Tetrahedron","material":{"Uniform":"default"}}}})";
        MeshGuard mesh{create_transform_c(json)};

        REQUIRE_FALSE(mesh.has_error());
        REQUIRE(mesh.entry_count() == 1);

        const auto& geom = mesh.entry(0).geometry;
        CHECK(vertex_count(geom) == 12);  // 4 faces * 3 vertices
        CHECK(triangle_count(geom) == 4);
    }

    TEST_CASE("Transform geometry data arrays are consistent") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"UniformScale":2.0})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;

        CHECK(geom.vertices_len == geom.normals_len);
        CHECK(geom.uvs_len * 3 == geom.vertices_len * 2);
        CHECK(geom.vertices != nullptr);
        CHECK(geom.normals != nullptr);
        CHECK(geom.uvs != nullptr);
        CHECK(geom.indices != nullptr);
    }

    TEST_CASE("Transform indices are within vertex bounds") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"RotationY":0.7})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto vert_count = static_cast<std::uint32_t>(vertex_count(geom));
        const auto indices = std::span{geom.indices, geom.indices_len};

        for (auto index : indices) {
            CHECK(index < vert_count);
        }
    }

    TEST_CASE("Transform normals are normalized after non-uniform scale") {
        MeshGuard mesh{
            create_transform_c(hexahedron_transform_json(R"({"ScaleXYZ":[2.0,3.0,4.0]})").c_str())
        };

        REQUIRE_FALSE(mesh.has_error());
        const auto& geom = mesh.entry(0).geometry;
        const auto normals = std::span{geom.normals, geom.normals_len};

        for (std::size_t i = 0; i < normals.size(); i += 3) {
            const auto nx = normals[i];
            const auto ny = normals[i + 1];
            const auto nz = normals[i + 2];
            const auto length = std::sqrt(nx * nx + ny * ny + nz * nz);
            CHECK(std::abs(length - 1.0f) < 0.001f);
        }
    }
}
