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
