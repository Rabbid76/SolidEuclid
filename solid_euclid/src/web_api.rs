use wasm_bindgen::prelude::*;

use crate::geometry_model::GeometryModel;
use crate::platonic_solid::Model;

#[wasm_bindgen]
pub fn hello_wasm() -> String {
    crate::hello()
}

/// Represents a triangular geometry for WebAssembly, providing efficient
/// access to GPU-ready mesh data via typed arrays.
#[wasm_bindgen]
pub struct WasmTriangularGeometry {
    vertices: Vec<f32>,
    normals: Vec<f32>,
    uvs: Vec<f32>,
    indices: Vec<u32>,
}

#[wasm_bindgen]
impl WasmTriangularGeometry {
    /// Returns a pointer to the vertices array for zero-copy access from JavaScript.
    #[wasm_bindgen(getter)]
    pub fn vertices(&self) -> js_sys::Float32Array {
        unsafe { js_sys::Float32Array::view(&self.vertices) }
    }

    /// Returns a pointer to the normals array for zero-copy access from JavaScript.
    #[wasm_bindgen(getter)]
    pub fn normals(&self) -> js_sys::Float32Array {
        unsafe { js_sys::Float32Array::view(&self.normals) }
    }

    /// Returns a pointer to the UVs array for zero-copy access from JavaScript.
    #[wasm_bindgen(getter)]
    pub fn uvs(&self) -> js_sys::Float32Array {
        unsafe { js_sys::Float32Array::view(&self.uvs) }
    }

    /// Returns a pointer to the indices array for zero-copy access from JavaScript.
    #[wasm_bindgen(getter)]
    pub fn indices(&self) -> js_sys::Uint32Array {
        unsafe { js_sys::Uint32Array::view(&self.indices) }
    }

    /// Returns the number of vertices in the geometry.
    #[wasm_bindgen(getter)]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 3
    }

    /// Returns the number of triangles in the geometry.
    #[wasm_bindgen(getter)]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Represents a triangular mesh for WebAssembly, containing multiple geometries
/// organized by material ID.
#[wasm_bindgen]
pub struct WasmTriangularMesh {
    material_ids: Vec<String>,
    geometries: Vec<WasmTriangularGeometry>,
}

#[wasm_bindgen]
impl WasmTriangularMesh {
    /// Returns the number of geometries (materials) in the mesh.
    #[wasm_bindgen(getter)]
    pub fn geometry_count(&self) -> usize {
        self.geometries.len()
    }

    /// Returns the material IDs as a JavaScript array.
    #[wasm_bindgen(getter)]
    pub fn material_ids(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for id in &self.material_ids {
            arr.push(&JsValue::from_str(id));
        }
        arr
    }

    /// Gets a geometry by index.
    pub fn get_geometry(&self, index: usize) -> Option<WasmTriangularGeometry> {
        self.geometries.get(index).map(|g| WasmTriangularGeometry {
            vertices: g.vertices.clone(),
            normals: g.normals.clone(),
            uvs: g.uvs.clone(),
            indices: g.indices.clone(),
        })
    }

    /// Gets a geometry by material ID.
    pub fn get_geometry_by_material(&self, material_id: &str) -> Option<WasmTriangularGeometry> {
        self.material_ids
            .iter()
            .position(|id| id == material_id)
            .and_then(|idx| self.get_geometry(idx))
    }
}

/// Creates a triangular mesh for a Platonic solid model from a JavaScript object.
///
/// # Arguments
/// * `model` - A JavaScript object representing a `Model` with `geometry` and `material` fields.
///
/// # Example TypeScript
/// ```typescript
/// const mesh = create_platonic_solid({
///   geometry: "Hexahedron",
///   material: { Uniform: "default" }
/// });
/// ```
///
/// Or with material array:
/// ```typescript
/// const mesh = create_platonic_solid({
///   geometry: "Hexahedron",
///   material: { Array: ["mat0", "mat1", "mat2", "mat3", "mat4", "mat5"] }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_platonic_solid(model: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let model: Model = serde_wasm_bindgen::from_value(model)
        .map_err(|e| JsError::new(&format!("Failed to parse model: {}", e)))?;

    let triangular_mesh = model
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create mesh: {:?}", e)))?;

    let mut material_ids = Vec::new();
    let mut geometries = Vec::new();

    // Sort material IDs to ensure consistent ordering
    let mut sorted_materials: Vec<_> = triangular_mesh.geometries.keys().collect();
    sorted_materials.sort();

    for material_id in sorted_materials {
        if let Some(geometry) = triangular_mesh.geometries.get(material_id) {
            material_ids.push(material_id.clone());
            geometries.push(WasmTriangularGeometry {
                vertices: geometry.vertices.clone(),
                normals: geometry.normals.clone(),
                uvs: geometry.uvs.clone(),
                indices: geometry.indices.iter().map(|&i| i as u32).collect(),
            });
        }
    }

    Ok(WasmTriangularMesh {
        material_ids,
        geometries,
    })
}

/// Returns a list of available Platonic solid types.
#[wasm_bindgen]
pub fn get_platonic_solid_types() -> js_sys::Array {
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_str("Tetrahedron"));
    arr.push(&JsValue::from_str("Hexahedron"));
    arr.push(&JsValue::from_str("Octahedron"));
    arr.push(&JsValue::from_str("Dodecahedron"));
    arr.push(&JsValue::from_str("Icosahedron"));
    arr
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platonic_solid::{Material, PlatonicSolid};

    #[test]
    fn test_hello_wasm() {
        let result = hello_wasm();
        assert_eq!(result, "Hello World!");
    }

    // Helper to create a Model directly for testing (since we can't create JsValue in tests)
    fn create_mesh_from_model(model: Model) -> WasmTriangularMesh {
        let triangular_mesh = model.to_triangular_mesh().unwrap();

        let mut material_ids = Vec::new();
        let mut geometries = Vec::new();

        let mut sorted_materials: Vec<_> = triangular_mesh.geometries.keys().collect();
        sorted_materials.sort();

        for material_id in sorted_materials {
            if let Some(geometry) = triangular_mesh.geometries.get(material_id) {
                material_ids.push(material_id.clone());
                geometries.push(WasmTriangularGeometry {
                    vertices: geometry.vertices.clone(),
                    normals: geometry.normals.clone(),
                    uvs: geometry.uvs.clone(),
                    indices: geometry.indices.iter().map(|&i| i as u32).collect(),
                });
            }
        }

        WasmTriangularMesh {
            material_ids,
            geometries,
        }
    }

    #[test]
    fn test_create_platonic_solid_uniform_tetrahedron() {
        let model = Model::new(
            PlatonicSolid::Tetrahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids.len(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geometry = mesh.get_geometry(0).unwrap();
        assert_eq!(geometry.vertex_count(), 12); // 4 faces * 3 vertices
        assert_eq!(geometry.triangle_count(), 4); // 4 triangular faces
    }

    #[test]
    fn test_create_platonic_solid_uniform_hexahedron() {
        let model = Model::new(
            PlatonicSolid::Hexahedron,
            Material::Uniform("cube_mat".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        assert_eq!(mesh.geometry_count(), 1);

        let geometry = mesh.get_geometry(0).unwrap();
        assert_eq!(geometry.vertex_count(), 24); // 6 faces * 4 vertices
        assert_eq!(geometry.triangle_count(), 12); // 6 quad faces = 12 triangles
    }

    #[test]
    fn test_create_platonic_solid_array_hexahedron() {
        let materials = vec![
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
            "mat3".to_string(),
            "mat4".to_string(),
            "mat5".to_string(),
        ];
        let model = Model::new(PlatonicSolid::Hexahedron, Material::Array(materials));
        let mesh = create_mesh_from_model(model);
        assert_eq!(mesh.geometry_count(), 6); // 6 different materials for 6 faces
    }

    #[test]
    fn test_create_platonic_solid_array_cyclic() {
        // Using 3 materials cyclically for 6 faces should result in 3 geometries
        let materials = vec![
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
        ];
        let model = Model::new(PlatonicSolid::Hexahedron, Material::Array(materials));
        let mesh = create_mesh_from_model(model);
        assert_eq!(mesh.geometry_count(), 3); // 3 unique materials
    }

    #[test]
    fn test_create_platonic_solid_default_material() {
        let model = Model::new(PlatonicSolid::Tetrahedron, Material::Default);
        let mesh = create_mesh_from_model(model);
        assert_eq!(mesh.geometry_count(), 4); // 4 faces, each with unique default material
    }

    #[test]
    fn test_create_platonic_solid_octahedron() {
        let model = Model::new(
            PlatonicSolid::Octahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        assert_eq!(geometry.vertex_count(), 24); // 8 faces * 3 vertices
        assert_eq!(geometry.triangle_count(), 8); // 8 triangular faces
    }

    #[test]
    fn test_create_platonic_solid_dodecahedron() {
        let model = Model::new(
            PlatonicSolid::Dodecahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        assert_eq!(geometry.vertex_count(), 60); // 12 faces * 5 vertices
        assert_eq!(geometry.triangle_count(), 36); // 12 pentagonal faces = 36 triangles
    }

    #[test]
    fn test_create_platonic_solid_icosahedron() {
        let model = Model::new(
            PlatonicSolid::Icosahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        assert_eq!(geometry.vertex_count(), 60); // 20 faces * 3 vertices
        assert_eq!(geometry.triangle_count(), 20); // 20 triangular faces
    }

    #[test]
    fn test_create_platonic_solid_map_material() {
        use std::collections::HashMap;
        let mut face_map = HashMap::new();
        face_map.insert(0, "special".to_string());
        face_map.insert(2, "special".to_string());
        let model = Model::new(
            PlatonicSolid::Tetrahedron,
            Material::Map {
                default: "base".to_string(),
                face_map,
            },
        );
        let mesh = create_mesh_from_model(model);
        assert_eq!(mesh.geometry_count(), 2); // "base" and "special"
    }

    #[test]
    fn test_get_geometry_by_material() {
        let materials = vec![
            "mat_a".to_string(),
            "mat_b".to_string(),
            "mat_a".to_string(),
            "mat_b".to_string(),
            "mat_a".to_string(),
            "mat_b".to_string(),
        ];
        let model = Model::new(PlatonicSolid::Hexahedron, Material::Array(materials));
        let mesh = create_mesh_from_model(model);

        let geometry_a = mesh.get_geometry_by_material("mat_a");
        let geometry_b = mesh.get_geometry_by_material("mat_b");
        let geometry_c = mesh.get_geometry_by_material("mat_c");

        assert!(geometry_a.is_some());
        assert!(geometry_b.is_some());
        assert!(geometry_c.is_none());
    }

    #[test]
    fn test_geometry_data_not_empty() {
        let model = Model::new(
            PlatonicSolid::Tetrahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.normals.is_empty());
        assert!(!geometry.uvs.is_empty());
        assert!(!geometry.indices.is_empty());
    }

    #[test]
    fn test_geometry_data_sizes_consistent() {
        let model = Model::new(
            PlatonicSolid::Hexahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        // vertices and normals should have same length (3 components each)
        assert_eq!(geometry.vertices.len(), geometry.normals.len());
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        assert_eq!(geometry.uvs.len() * 3, geometry.vertices.len() * 2);
    }

    #[test]
    fn test_normals_are_normalized() {
        let model = Model::new(
            PlatonicSolid::Octahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        for i in (0..geometry.normals.len()).step_by(3) {
            let nx = geometry.normals[i];
            let ny = geometry.normals[i + 1];
            let nz = geometry.normals[i + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            assert!(
                (length - 1.0).abs() < 0.001,
                "Normal not normalized: length = {}",
                length
            );
        }
    }

    #[test]
    fn test_indices_within_bounds() {
        let model = Model::new(
            PlatonicSolid::Icosahedron,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        let vertex_count = geometry.vertex_count() as u32;
        for &index in &geometry.indices {
            assert!(
                index < vertex_count,
                "Index {} out of bounds (vertex count: {})",
                index,
                vertex_count
            );
        }
    }

    #[test]
    fn test_array_material_total_triangles() {
        // Ensure total triangles across all geometries is correct for hexahedron
        let materials = vec![
            "m0".to_string(),
            "m1".to_string(),
            "m2".to_string(),
            "m3".to_string(),
            "m4".to_string(),
            "m5".to_string(),
        ];
        let model = Model::new(PlatonicSolid::Hexahedron, Material::Array(materials));
        let mesh = create_mesh_from_model(model);

        let total_triangles: usize = (0..mesh.geometry_count())
            .filter_map(|i| mesh.get_geometry(i))
            .map(|g| g.triangle_count())
            .sum();

        assert_eq!(total_triangles, 12); // Hexahedron has 12 triangles total
    }
}
