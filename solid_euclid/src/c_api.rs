use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::geometry_model::GeometryModel;
use crate::platonic_solid::Model as PlatonicSolidModel;
use crate::sphere::Model as SphereModel;
use crate::triangular_mesh::TriangularMesh;

/// Returns a C-compatible string with the hello message.
/// The caller is responsible for freeing the returned string using `hello_c_free`.
#[unsafe(no_mangle)]
pub extern "C" fn hello_c() -> *mut c_char {
    let result = crate::hello();
    match CString::new(result) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a string allocated by `hello_c`.
/// # Safety
/// The pointer must be a valid pointer returned by `hello_c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hello_c_free(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

// =============================================================================
// Triangular Geometry C API Types
// =============================================================================

/// Represents a triangular geometry for C API.
/// Contains flat arrays of vertices, normals, UVs, and indices.
#[repr(C)]
pub struct CTriangularGeometry {
    /// Pointer to the vertices array (3 floats per vertex: x, y, z).
    pub vertices: *mut f32,
    /// Number of floats in the vertices array (vertex_count * 3).
    pub vertices_len: usize,
    /// Pointer to the normals array (3 floats per normal: x, y, z).
    pub normals: *mut f32,
    /// Number of floats in the normals array (vertex_count * 3).
    pub normals_len: usize,
    /// Pointer to the UVs array (2 floats per UV: u, v).
    pub uvs: *mut f32,
    /// Number of floats in the uvs array (vertex_count * 2).
    pub uvs_len: usize,
    /// Pointer to the indices array (3 indices per triangle).
    pub indices: *mut u32,
    /// Number of indices in the indices array.
    pub indices_len: usize,
}

/// Represents a single geometry entry in a triangular mesh, associated with a material ID.
#[repr(C)]
pub struct CTriangularMeshEntry {
    /// The material ID as a null-terminated string.
    pub material_id: *mut c_char,
    /// The triangular geometry data.
    pub geometry: CTriangularGeometry,
}

/// Represents a triangular mesh for C API.
/// Contains multiple geometry entries, each associated with a material ID.
#[repr(C)]
pub struct CTriangularMesh {
    /// Pointer to an array of mesh entries.
    pub entries: *mut CTriangularMeshEntry,
    /// Number of entries in the mesh.
    pub entries_len: usize,
    /// Error message if creation failed, null if successful.
    pub error: *mut c_char,
}

impl Default for CTriangularMesh {
    fn default() -> Self {
        Self {
            entries: std::ptr::null_mut(),
            entries_len: 0,
            error: std::ptr::null_mut(),
        }
    }
}

/// Creates a CTriangularMesh from a TriangularMesh.
fn triangular_mesh_to_c(mesh: TriangularMesh) -> CTriangularMesh {
    let mut sorted_materials: Vec<_> = mesh.geometries.keys().collect();
    sorted_materials.sort();

    let mut entries: Vec<CTriangularMeshEntry> = Vec::with_capacity(sorted_materials.len());

    for material_id in sorted_materials {
        if let Some(geometry) = mesh.geometries.get(material_id) {
            // Clone and convert to owned data
            let mut vertices = geometry.vertices.clone();
            let mut normals = geometry.normals.clone();
            let mut uvs = geometry.uvs.clone();
            let mut indices: Vec<u32> = geometry.indices.iter().map(|&i| i as u32).collect();

            // Ensure capacity equals length before transferring ownership to C
            vertices.shrink_to_fit();
            normals.shrink_to_fit();
            uvs.shrink_to_fit();
            indices.shrink_to_fit();

            let vertices_len = vertices.len();
            let normals_len = normals.len();
            let uvs_len = uvs.len();
            let indices_len = indices.len();

            let vertices_ptr = vertices.as_mut_ptr();
            let normals_ptr = normals.as_mut_ptr();
            let uvs_ptr = uvs.as_mut_ptr();
            let indices_ptr = indices.as_mut_ptr();

            // Prevent deallocation - ownership transferred to C
            std::mem::forget(vertices);
            std::mem::forget(normals);
            std::mem::forget(uvs);
            std::mem::forget(indices);

            let material_id_cstring = CString::new(material_id.clone()).unwrap_or_default();
            let material_id_ptr = material_id_cstring.into_raw();

            entries.push(CTriangularMeshEntry {
                material_id: material_id_ptr,
                geometry: CTriangularGeometry {
                    vertices: vertices_ptr,
                    vertices_len,
                    normals: normals_ptr,
                    normals_len,
                    uvs: uvs_ptr,
                    uvs_len,
                    indices: indices_ptr,
                    indices_len,
                },
            });
        }
    }

    let entries_len = entries.len();
    let entries_ptr = if entries.is_empty() {
        std::ptr::null_mut()
    } else {
        // Ensure capacity equals length before transferring ownership to C
        entries.shrink_to_fit();
        let ptr = entries.as_mut_ptr();
        std::mem::forget(entries);
        ptr
    };

    CTriangularMesh {
        entries: entries_ptr,
        entries_len,
        error: std::ptr::null_mut(),
    }
}

/// Creates an error CTriangularMesh with the given error message.
fn create_error_mesh(error_msg: &str) -> CTriangularMesh {
    let error_cstring = CString::new(error_msg).unwrap_or_default();
    CTriangularMesh {
        entries: std::ptr::null_mut(),
        entries_len: 0,
        error: error_cstring.into_raw(),
    }
}

// =============================================================================
// Platonic Solid C API
// =============================================================================

/// Creates a triangular mesh for a Platonic solid from a JSON string.
///
/// # Arguments
/// * `model_json` - A null-terminated JSON string representing the model.
///
/// # Example JSON
/// ```json
/// {
///   "geometry": "Hexahedron",
///   "material": { "Uniform": "default" }
/// }
/// ```
///
/// Or with material array:
/// ```json
/// {
///   "geometry": "Hexahedron",
///   "material": { "Array": ["mat0", "mat1", "mat2", "mat3", "mat4", "mat5"] }
/// }
/// ```
///
/// # Returns
/// A `CTriangularMesh` containing the mesh data organized by material.
/// Check the `error` field for any errors.
///
/// # Safety
/// The caller must free the returned mesh using `triangular_mesh_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_platonic_solid_c(model_json: *const c_char) -> CTriangularMesh {
    unsafe {
        if model_json.is_null() {
            return create_error_mesh("Null JSON string provided");
        }

        let json_str = match CStr::from_ptr(model_json).to_str() {
            Ok(s) => s,
            Err(e) => return create_error_mesh(&format!("Invalid UTF-8 in JSON string: {}", e)),
        };

        let model: PlatonicSolidModel = match serde_json::from_str(json_str) {
            Ok(m) => m,
            Err(e) => return create_error_mesh(&format!("Failed to parse JSON: {}", e)),
        };

        match model.to_triangular_mesh() {
            Ok(mesh) => triangular_mesh_to_c(mesh),
            Err(e) => create_error_mesh(&format!("Failed to create mesh: {:?}", e)),
        }
    }
}

/// Returns a JSON array of available Platonic solid types as a C string.
///
/// # Returns
/// A null-terminated JSON string containing the array of types.
/// The caller must free the returned string using `string_free_c`.
#[unsafe(no_mangle)]
pub extern "C" fn get_platonic_solid_types_c() -> *mut c_char {
    let types = r#"["Tetrahedron","Hexahedron","Octahedron","Dodecahedron","Icosahedron"]"#;
    match CString::new(types) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// =============================================================================
// Sphere C API
// =============================================================================

/// Creates a triangular mesh for a Sphere from a JSON string.
///
/// # Arguments
/// * `model_json` - A null-terminated JSON string representing the model.
///
/// # Example JSON (UV Sphere with slices)
/// ```json
/// {
///   "geometry": { "Slices": { "slices": 16, "segments": 32 } },
///   "material": { "Uniform": "default" }
/// }
/// ```
///
/// # Example JSON (Cube-mapped sphere)
/// ```json
/// {
///   "geometry": { "Cube": { "segments": 8 } },
///   "material": { "Array": ["mat0", "mat1", "mat2", "mat3", "mat4", "mat5"] }
/// }
/// ```
///
/// # Returns
/// A `CTriangularMesh` containing the mesh data organized by material.
/// Check the `error` field for any errors.
///
/// # Safety
/// The caller must free the returned mesh using `triangular_mesh_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_sphere_c(model_json: *const c_char) -> CTriangularMesh {
    unsafe {
        if model_json.is_null() {
            return create_error_mesh("Null JSON string provided");
        }

        let json_str = match CStr::from_ptr(model_json).to_str() {
            Ok(s) => s,
            Err(e) => return create_error_mesh(&format!("Invalid UTF-8 in JSON string: {}", e)),
        };

        let model: SphereModel = match serde_json::from_str(json_str) {
            Ok(m) => m,
            Err(e) => return create_error_mesh(&format!("Failed to parse JSON: {}", e)),
        };

        match model.to_triangular_mesh() {
            Ok(mesh) => triangular_mesh_to_c(mesh),
            Err(e) => create_error_mesh(&format!("Failed to create mesh: {:?}", e)),
        }
    }
}

/// Returns a JSON array of available Sphere types as a C string.
///
/// # Returns
/// A null-terminated JSON string containing the array of types.
/// The caller must free the returned string using `string_free_c`.
#[unsafe(no_mangle)]
pub extern "C" fn get_sphere_types_c() -> *mut c_char {
    let types = r#"["Slices","Cube"]"#;
    match CString::new(types) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// =============================================================================
// Memory Management
// =============================================================================

/// Frees a triangular mesh allocated by `create_platonic_solid_c` or `create_sphere_c`.
///
/// # Safety
/// The pointer must be a valid `CTriangularMesh` returned by one of the create functions.
/// After calling this function, the mesh pointer is invalid and must not be used.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn triangular_mesh_free(mesh: CTriangularMesh) {
    unsafe {
        // Free the error string if present
        if !mesh.error.is_null() {
            let _ = CString::from_raw(mesh.error);
        }

        // Free each entry
        if !mesh.entries.is_null() && mesh.entries_len > 0 {
            let entries = Vec::from_raw_parts(mesh.entries, mesh.entries_len, mesh.entries_len);

            for entry in entries {
                // Free material ID string
                if !entry.material_id.is_null() {
                    let _ = CString::from_raw(entry.material_id);
                }

                // Free geometry arrays
                let geom = &entry.geometry;
                if !geom.vertices.is_null() && geom.vertices_len > 0 {
                    let _ =
                        Vec::from_raw_parts(geom.vertices, geom.vertices_len, geom.vertices_len);
                }
                if !geom.normals.is_null() && geom.normals_len > 0 {
                    let _ = Vec::from_raw_parts(geom.normals, geom.normals_len, geom.normals_len);
                }
                if !geom.uvs.is_null() && geom.uvs_len > 0 {
                    let _ = Vec::from_raw_parts(geom.uvs, geom.uvs_len, geom.uvs_len);
                }
                if !geom.indices.is_null() && geom.indices_len > 0 {
                    let _ = Vec::from_raw_parts(geom.indices, geom.indices_len, geom.indices_len);
                }
            }
        }
    }
}

/// Frees a string allocated by `get_platonic_solid_types_c` or `get_sphere_types_c`.
///
/// # Safety
/// The pointer must be a valid pointer returned by one of the string-returning functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_free_c(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_hello_c() {
        let result_ptr = hello_c();
        assert!(!result_ptr.is_null());

        let result = unsafe { CStr::from_ptr(result_ptr) };
        assert_eq!(result.to_str().unwrap(), "Hello World!");

        unsafe { hello_c_free(result_ptr) };
    }

    // =============================================================================
    // Platonic Solid C API Tests
    // =============================================================================

    #[test]
    fn test_create_platonic_solid_c_tetrahedron_uniform() {
        let json =
            CString::new(r#"{"geometry":"Tetrahedron","material":{"Uniform":"default"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null(), "Expected no error");
        assert_eq!(mesh.entries_len, 1);
        assert!(!mesh.entries.is_null());

        unsafe {
            let entry = &*mesh.entries;
            let material_id = CStr::from_ptr(entry.material_id).to_str().unwrap();
            assert_eq!(material_id, "default");

            let geom = &entry.geometry;
            assert_eq!(geom.vertices_len / 3, 12); // 4 faces * 3 vertices
            assert_eq!(geom.indices_len / 3, 4); // 4 triangular faces

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_hexahedron_uniform() {
        let json =
            CString::new(r#"{"geometry":"Hexahedron","material":{"Uniform":"cube_mat"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null(), "Expected no error");
        assert_eq!(mesh.entries_len, 1);

        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            assert_eq!(geom.vertices_len / 3, 24); // 6 faces * 4 vertices
            assert_eq!(geom.indices_len / 3, 12); // 6 quad faces = 12 triangles

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_hexahedron_array() {
        let json = CString::new(
            r#"{"geometry":"Hexahedron","material":{"Array":["mat0","mat1","mat2","mat3","mat4","mat5"]}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null(), "Expected no error");
        assert_eq!(mesh.entries_len, 6); // 6 different materials for 6 faces

        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_create_platonic_solid_c_default_material() {
        let json = CString::new(r#"{"geometry":"Tetrahedron","material":"Default"}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null(), "Expected no error");
        assert_eq!(mesh.entries_len, 4); // 4 faces, each with unique default material

        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_create_platonic_solid_c_octahedron() {
        let json =
            CString::new(r#"{"geometry":"Octahedron","material":{"Uniform":"default"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            assert_eq!(geom.vertices_len / 3, 24); // 8 faces * 3 vertices
            assert_eq!(geom.indices_len / 3, 8); // 8 triangular faces

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_dodecahedron() {
        let json = CString::new(r#"{"geometry":"Dodecahedron","material":{"Uniform":"default"}}"#)
            .unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            assert_eq!(geom.vertices_len / 3, 60); // 12 faces * 5 vertices
            assert_eq!(geom.indices_len / 3, 36); // 12 pentagonal faces = 36 triangles

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_icosahedron() {
        let json =
            CString::new(r#"{"geometry":"Icosahedron","material":{"Uniform":"default"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            assert_eq!(geom.vertices_len / 3, 60); // 20 faces * 3 vertices
            assert_eq!(geom.indices_len / 3, 20); // 20 triangular faces

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_map_material() {
        let json = CString::new(
            r#"{"geometry":"Tetrahedron","material":{"Map":{"default":"base","face_map":{"0":"special","2":"special"}}}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 2); // "base" and "special"

        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_create_platonic_solid_c_null_json() {
        let mesh = unsafe { create_platonic_solid_c(std::ptr::null()) };

        assert!(!mesh.error.is_null());
        unsafe {
            let error = CStr::from_ptr(mesh.error).to_str().unwrap();
            assert!(error.contains("Null"));
            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_invalid_json() {
        let json = CString::new(r#"{"invalid": json}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(!mesh.error.is_null());
        unsafe {
            let error = CStr::from_ptr(mesh.error).to_str().unwrap();
            assert!(error.contains("Failed to parse"));
            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_platonic_solid_c_invalid_geometry() {
        let json = CString::new(r#"{"geometry":"InvalidSolid","material":{"Uniform":"default"}}"#)
            .unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(!mesh.error.is_null());
        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_get_platonic_solid_types_c() {
        let types_ptr = get_platonic_solid_types_c();
        assert!(!types_ptr.is_null());

        let types_str = unsafe { CStr::from_ptr(types_ptr).to_str().unwrap() };
        assert!(types_str.contains("Tetrahedron"));
        assert!(types_str.contains("Hexahedron"));
        assert!(types_str.contains("Octahedron"));
        assert!(types_str.contains("Dodecahedron"));
        assert!(types_str.contains("Icosahedron"));

        unsafe { string_free_c(types_ptr) };
    }

    #[test]
    fn test_platonic_solid_c_geometry_data_consistent() {
        let json =
            CString::new(r#"{"geometry":"Hexahedron","material":{"Uniform":"default"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;

            // vertices and normals should have same length (3 components each)
            assert_eq!(geom.vertices_len, geom.normals_len);
            // uvs should have 2/3 the length of vertices (2 components vs 3)
            assert_eq!(geom.uvs_len * 3, geom.vertices_len * 2);

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_platonic_solid_c_normals_normalized() {
        let json =
            CString::new(r#"{"geometry":"Octahedron","material":{"Uniform":"default"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            let normals = std::slice::from_raw_parts(geom.normals, geom.normals_len);

            for i in (0..normals.len()).step_by(3) {
                let nx = normals[i];
                let ny = normals[i + 1];
                let nz = normals[i + 2];
                let length = (nx * nx + ny * ny + nz * nz).sqrt();
                assert!(
                    (length - 1.0).abs() < 0.001,
                    "Normal not normalized: length = {}",
                    length
                );
            }

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_platonic_solid_c_indices_within_bounds() {
        let json =
            CString::new(r#"{"geometry":"Icosahedron","material":{"Uniform":"default"}}"#).unwrap();
        let mesh = unsafe { create_platonic_solid_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            let indices = std::slice::from_raw_parts(geom.indices, geom.indices_len);
            let vertex_count = (geom.vertices_len / 3) as u32;

            for &index in indices {
                assert!(
                    index < vertex_count,
                    "Index {} out of bounds (vertex count: {})",
                    index,
                    vertex_count
                );
            }

            triangular_mesh_free(mesh);
        }
    }

    // =============================================================================
    // Sphere C API Tests
    // =============================================================================

    #[test]
    fn test_create_sphere_c_slices_uniform() {
        let json = CString::new(
            r#"{"geometry":{"Slices":{"slices":8,"segments":16}},"material":{"Uniform":"default"}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null(), "Expected no error");
        assert_eq!(mesh.entries_len, 1);
        assert!(!mesh.entries.is_null());

        unsafe {
            let entry = &*mesh.entries;
            let material_id = CStr::from_ptr(entry.material_id).to_str().unwrap();
            assert_eq!(material_id, "default");

            let geom = &entry.geometry;
            assert!(geom.vertices_len > 0);
            assert!(geom.indices_len > 0);

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_sphere_c_slices_default_material() {
        let json = CString::new(
            r#"{"geometry":{"Slices":{"slices":8,"segments":16}},"material":"Default"}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 1); // Slices sphere has 1 face

        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_create_sphere_c_cube_uniform() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 1);

        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            // 6 faces * 4*4 quads * 2 triangles = 192 triangles
            assert_eq!(geom.indices_len / 3, 6 * 4 * 4 * 2);

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_sphere_c_cube_array_6_materials() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Array":["mat0","mat1","mat2","mat3","mat4","mat5"]}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 6); // 6 faces, 6 different materials

        unsafe {
            // Each face should have segments^2 * 2 triangles
            for i in 0..6 {
                let entry = &*mesh.entries.add(i);
                let geom = &entry.geometry;
                assert_eq!(geom.indices_len / 3, 4 * 4 * 2); // 32 triangles per face
            }

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_sphere_c_cube_default_material() {
        let json =
            CString::new(r#"{"geometry":{"Cube":{"segments":4}},"material":"Default"}"#).unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 6); // 6 faces, each with unique default material

        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_create_sphere_c_cube_cyclic_materials() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Array":["mat0","mat1","mat2","mat0","mat1","mat2"]}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 3); // 3 unique materials

        unsafe { triangular_mesh_free(mesh) };
    }

    #[test]
    fn test_create_sphere_c_null_json() {
        let mesh = unsafe { create_sphere_c(std::ptr::null()) };

        assert!(!mesh.error.is_null());
        unsafe {
            let error = CStr::from_ptr(mesh.error).to_str().unwrap();
            assert!(error.contains("Null"));
            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_create_sphere_c_invalid_json() {
        let json = CString::new(r#"{"invalid": json}"#).unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(!mesh.error.is_null());
        unsafe {
            let error = CStr::from_ptr(mesh.error).to_str().unwrap();
            assert!(error.contains("Failed to parse"));
            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_get_sphere_types_c() {
        let types_ptr = get_sphere_types_c();
        assert!(!types_ptr.is_null());

        let types_str = unsafe { CStr::from_ptr(types_ptr).to_str().unwrap() };
        assert!(types_str.contains("Slices"));
        assert!(types_str.contains("Cube"));

        unsafe { string_free_c(types_ptr) };
    }

    #[test]
    fn test_sphere_c_geometry_data_consistent() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;

            // vertices and normals should have same length (3 components each)
            assert_eq!(geom.vertices_len, geom.normals_len);
            // uvs should have 2/3 the length of vertices (2 components vs 3)
            assert_eq!(geom.uvs_len * 3, geom.vertices_len * 2);

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_sphere_c_normals_normalized() {
        let json = CString::new(
            r#"{"geometry":{"Slices":{"slices":8,"segments":16}},"material":{"Uniform":"default"}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            let normals = std::slice::from_raw_parts(geom.normals, geom.normals_len);

            for i in (0..normals.len()).step_by(3) {
                let nx = normals[i];
                let ny = normals[i + 1];
                let nz = normals[i + 2];
                let length = (nx * nx + ny * ny + nz * nz).sqrt();
                assert!(
                    (length - 1.0).abs() < 0.001,
                    "Normal not normalized: length = {}",
                    length
                );
            }

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_sphere_c_indices_within_bounds() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Uniform":"default"}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let entry = &*mesh.entries;
            let geom = &entry.geometry;
            let indices = std::slice::from_raw_parts(geom.indices, geom.indices_len);
            let vertex_count = (geom.vertices_len / 3) as u32;

            for &index in indices {
                assert!(
                    index < vertex_count,
                    "Index {} out of bounds (vertex count: {})",
                    index,
                    vertex_count
                );
            }

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_sphere_c_cube_total_triangles() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Array":["m0","m1","m2","m3","m4","m5"]}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        unsafe {
            let mut total_triangles = 0;
            for i in 0..mesh.entries_len {
                let entry = &*mesh.entries.add(i);
                total_triangles += entry.geometry.indices_len / 3;
            }

            // 6 faces * 4*4 quads * 2 triangles = 192 triangles
            assert_eq!(total_triangles, 6 * 4 * 4 * 2);

            triangular_mesh_free(mesh);
        }
    }

    #[test]
    fn test_sphere_c_map_material() {
        let json = CString::new(
            r#"{"geometry":{"Cube":{"segments":4}},"material":{"Map":{"default":"base","face_map":{"0":"special","2":"special"}}}}"#,
        )
        .unwrap();
        let mesh = unsafe { create_sphere_c(json.as_ptr()) };

        assert!(mesh.error.is_null());
        assert_eq!(mesh.entries_len, 2); // "base" and "special"

        unsafe { triangular_mesh_free(mesh) };
    }
}
