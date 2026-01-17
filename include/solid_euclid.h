#ifndef SOLID_EUCLID_H
#define SOLID_EUCLID_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Returns a C-compatible, heap-allocated string with the hello message.
 *
 * The caller takes ownership of the returned pointer and is responsible for
 * freeing it exactly once by calling hello_c_free(). The pointer must not be
 * freed with free() or any deallocation function other than hello_c_free().
 */
char* hello_c(void);

/**
 * Frees a string allocated by hello_c().
 *
 * The argument must be a pointer previously returned by hello_c() that has
 * not already been freed. Passing any other pointer value, including NULL
 * or a pointer that has already been freed, results in undefined behavior.
 */
void hello_c_free(char* s);

/* ============================================================================
 * Triangular Geometry C API Types
 * ============================================================================ */

/**
 * Represents a triangular geometry.
 * Contains flat arrays of vertices, normals, UVs, and indices.
 */
typedef struct {
    /** Pointer to the vertices array (3 floats per vertex: x, y, z). */
    float* vertices;
    /** Number of floats in the vertices array (vertex_count * 3). */
    size_t vertices_len;
    /** Pointer to the normals array (3 floats per normal: x, y, z). */
    float* normals;
    /** Number of floats in the normals array (vertex_count * 3). */
    size_t normals_len;
    /** Pointer to the UVs array (2 floats per UV: u, v). */
    float* uvs;
    /** Number of floats in the uvs array (vertex_count * 2). */
    size_t uvs_len;
    /** Pointer to the indices array (3 indices per triangle). */
    uint32_t* indices;
    /** Number of indices in the indices array. */
    size_t indices_len;
} CTriangularGeometry;

/**
 * Represents a single geometry entry in a triangular mesh, associated with a material ID.
 */
typedef struct {
    /** The material ID as a null-terminated string. */
    char* material_id;
    /** The triangular geometry data. */
    CTriangularGeometry geometry;
} CTriangularMeshEntry;

/**
 * Represents a triangular mesh.
 * Contains multiple geometry entries, each associated with a material ID.
 */
typedef struct {
    /** Pointer to an array of mesh entries. */
    CTriangularMeshEntry* entries;
    /** Number of entries in the mesh. */
    size_t entries_len;
    /** Error message if creation failed, null if successful. */
    char* error;
} CTriangularMesh;

/* ============================================================================
 * Platonic Solid C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for a Platonic solid from a JSON string.
 *
 * @param model_json A null-terminated JSON string representing the model.
 *
 * Example JSON:
 * @code
 * {
 *   "geometry": "Hexahedron",
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * Or with material array:
 * @code
 * {
 *   "geometry": "Hexahedron",
 *   "material": { "Array": ["mat0", "mat1", "mat2", "mat3", "mat4", "mat5"] }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_platonic_solid_c(const char* model_json);

/**
 * Returns a JSON array of available Platonic solid types as a C string.
 *
 * @return A null-terminated JSON string containing the array of types:
 *         ["Tetrahedron","Hexahedron","Octahedron","Dodecahedron","Icosahedron"]
 *         The caller must free the returned string using string_free_c().
 */
char* get_platonic_solid_types_c(void);

/* ============================================================================
 * Sphere C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for a Sphere from a JSON string.
 *
 * @param model_json A null-terminated JSON string representing the model.
 *
 * Example JSON (UV Sphere with slices):
 * @code
 * {
 *   "geometry": { "Slices": { "slices": 16, "segments": 32 } },
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * Example JSON (Cube-mapped sphere):
 * @code
 * {
 *   "geometry": { "Cube": { "segments": 8 } },
 *   "material": { "Array": ["mat0", "mat1", "mat2", "mat3", "mat4", "mat5"] }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_sphere_c(const char* model_json);

/**
 * Returns a JSON array of available Sphere types as a C string.
 *
 * @return A null-terminated JSON string containing the array of types:
 *         ["Slices","Cube"]
 *         The caller must free the returned string using string_free_c().
 */
char* get_sphere_types_c(void);

/* ============================================================================
 * Torus C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for a Torus from a JSON string.
 *
 * @param torus_json A null-terminated JSON string representing the torus.
 *
 * Example JSON:
 * @code
 * {
 *   "major_radius": 2.0,
 *   "minor_radius": 0.5,
 *   "major_segments": 36,
 *   "minor_segments": 18,
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_torus_c(const char* torus_json);

/* ============================================================================
 * Capsule C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for a Capsule from a JSON string.
 *
 * A capsule is a cylinder with hemispheres at both ends. It supports 3 material regions:
 * - Face 0: Bottom hemisphere
 * - Face 1: Cylindrical body
 * - Face 2: Top hemisphere
 *
 * @param capsule_json A null-terminated JSON string representing the capsule.
 *
 * Example JSON:
 * @code
 * {
 *   "radius": 0.5,
 *   "height": 2.0,
 *   "segments": 24,
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * Or with 3 materials:
 * @code
 * {
 *   "radius": 0.5,
 *   "height": 2.0,
 *   "segments": 24,
 *   "material": { "Array": ["bottom", "body", "top"] }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_capsule_c(const char* capsule_json);

/* ============================================================================
 * Cylinder C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for a Cylinder from a JSON string.
 *
 * A cylinder supports 5 material regions:
 * - Face 0: Bottom cap
 * - Face 1: Bottom edge (chamfer or fillet)
 * - Face 2: Cylindrical body
 * - Face 3: Top edge (chamfer or fillet)
 * - Face 4: Top cap
 *
 * @param cylinder_json A null-terminated JSON string representing the cylinder.
 *
 * Example JSON (sharp edges):
 * @code
 * {
 *   "radius": 0.5,
 *   "height": 2.0,
 *   "segments": 24,
 *   "bottom_edge": "Edge",
 *   "top_edge": "Edge",
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * Example JSON (chamfered edges):
 * @code
 * {
 *   "radius": 0.5,
 *   "height": 2.0,
 *   "segments": 24,
 *   "bottom_edge": { "Chamfer": { "distance": 0.1 } },
 *   "top_edge": { "Chamfer": { "distance": 0.1 } },
 *   "material": { "Array": ["bottom_cap", "bottom_edge", "body", "top_edge", "top_cap"] }
 * }
 * @endcode
 *
 * Example JSON (filleted edges):
 * @code
 * {
 *   "radius": 0.5,
 *   "height": 2.0,
 *   "segments": 24,
 *   "bottom_edge": { "Fillet": { "radius": 0.1 } },
 *   "top_edge": { "Fillet": { "radius": 0.1 } },
 *   "material": { "Uniform": "metal" }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_cylinder_c(const char* cylinder_json);

/* ============================================================================
 * Conical Frustum C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for a Conical Frustum (truncated cone) from a JSON string.
 *
 * A conical frustum supports 5 material regions:
 * - Face 0: Bottom cap
 * - Face 1: Bottom edge (chamfer or fillet)
 * - Face 2: Conical body
 * - Face 3: Top edge (chamfer or fillet)
 * - Face 4: Top cap
 *
 * @param frustum_json A null-terminated JSON string representing the frustum.
 *
 * Example JSON (sharp edges):
 * @code
 * {
 *   "bottom_radius": 0.5,
 *   "top_radius": 0.25,
 *   "height": 2.0,
 *   "segments": 24,
 *   "bottom_edge": "Edge",
 *   "top_edge": "Edge",
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * Example JSON (cone with top_radius = 0):
 * @code
 * {
 *   "bottom_radius": 0.5,
 *   "top_radius": 0.0,
 *   "height": 2.0,
 *   "segments": 24,
 *   "bottom_edge": "Edge",
 *   "top_edge": "Edge",
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_conical_frustum_c(const char* frustum_json);

/* ============================================================================
 * Extrude C API
 * ============================================================================ */

/**
 * Creates a triangular mesh for an Extrude from a JSON string.
 *
 * An extrude is created by sweeping a 2D profile along the Y-axis.
 * The extrusion goes from -height/2 to +height/2, centered at the origin.
 *
 * The extrude supports separate materials for:
 * - Side faces (geometry_id based on profile segment IDs)
 * - Bottom cap (geometry_id = -1)
 * - Top cap (geometry_id = -2)
 *
 * @param extrude_json A null-terminated JSON string representing the extrude.
 *
 * Example JSON (simple square extrusion):
 * @code
 * {
 *   "profile": {
 *     "commands": [
 *       { "path": { "Move": { "point": [0.0, 0.0] } }, "corner": "Edge" },
 *       { "path": { "Line": { "point": [1.0, 0.0] } }, "corner": "Edge" },
 *       { "path": { "Line": { "point": [1.0, 1.0] } }, "corner": "Edge" },
 *       { "path": { "Line": { "point": [0.0, 1.0] } }, "corner": "Edge" },
 *       { "path": { "Line": { "point": [0.0, 0.0] } }, "corner": "Edge" }
 *     ]
 *   },
 *   "height": 2.0,
 *   "sample_distance": 0.1,
 *   "material": { "Uniform": "default" }
 * }
 * @endcode
 *
 * Example JSON (triangle with filleted corners):
 * @code
 * {
 *   "profile": {
 *     "commands": [
 *       { "path": { "Move": { "point": [0.0, 0.0] } }, "corner": "Edge" },
 *       { "path": { "Line": { "point": [1.0, 0.0] } }, "corner": { "Fillet": { "radius": 0.1 } } },
 *       { "path": { "Line": { "point": [0.5, 0.866] } }, "corner": { "Fillet": { "radius": 0.1 } } },
 *       { "path": { "Line": { "point": [0.0, 0.0] } }, "corner": { "Fillet": { "radius": 0.1 } } }
 *     ]
 *   },
 *   "height": 1.5,
 *   "sample_distance": 0.05,
 *   "material": { "Uniform": "metal" }
 * }
 * @endcode
 *
 * @return A CTriangularMesh containing the mesh data organized by material.
 *         Check the `error` field for any errors.
 *         The caller must free the returned mesh using triangular_mesh_free().
 */
CTriangularMesh create_extrude_c(const char* extrude_json);

/* ============================================================================
 * Memory Management
 * ============================================================================ */

/**
 * Frees a triangular mesh allocated by create_platonic_solid_c() or create_sphere_c().
 *
 * @param mesh The CTriangularMesh to free.
 *             After calling this function, the mesh is invalid and must not be used.
 */
void triangular_mesh_free(CTriangularMesh mesh);

/**
 * Frees a string allocated by get_platonic_solid_types_c() or get_sphere_types_c().
 *
 * @param s The string to free. May be NULL (no-op in that case).
 */
void string_free_c(char* s);

#ifdef __cplusplus
}
#endif

#endif /* SOLID_EUCLID_H */
