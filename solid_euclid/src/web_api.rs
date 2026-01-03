use wasm_bindgen::prelude::*;

use crate::geometry_model::GeometryModel;
use crate::platonic_solid::{Model, PlatonicSolid};

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

/// Creates a triangular geometry for a Platonic solid.
///
/// # Arguments
/// * `solid_type` - The type of Platonic solid: "tetrahedron", "hexahedron", "octahedron", "dodecahedron", or "icosahedron"
/// * `material_name` - The material name to use for all faces (uniform material)
///
/// # Returns
/// A `WasmTriangularGeometry` containing the mesh data ready for GPU rendering.
#[wasm_bindgen]
pub fn create_platonic_solid(solid_type: &str, material_name: &str) -> Result<WasmTriangularGeometry, JsError> {
    let solid = match solid_type.to_lowercase().as_str() {
        "tetrahedron" => PlatonicSolid::Tetrahedron,
        "hexahedron" | "cube" => PlatonicSolid::Hexahedron,
        "octahedron" => PlatonicSolid::Octahedron,
        "dodecahedron" => PlatonicSolid::Dodecahedron,
        "icosahedron" => PlatonicSolid::Icosahedron,
        _ => return Err(JsError::new(&format!("Unknown solid type: {}", solid_type))),
    };

    let model = Model::with_uniform_material(solid, material_name.to_string());
    let triangular_mesh = model
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create mesh: {:?}", e)))?;

    // Get the geometry for the uniform material
    let geometry = triangular_mesh
        .get(material_name)
        .ok_or_else(|| JsError::new("Geometry not found for material"))?;

    Ok(WasmTriangularGeometry {
        vertices: geometry.vertices.clone(),
        normals: geometry.normals.clone(),
        uvs: geometry.uvs.clone(),
        indices: geometry.indices.iter().map(|&i| i as u32).collect(),
    })
}

/// Returns a list of available Platonic solid types.
#[wasm_bindgen]
pub fn get_platonic_solid_types() -> js_sys::Array {
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_str("tetrahedron"));
    arr.push(&JsValue::from_str("hexahedron"));
    arr.push(&JsValue::from_str("octahedron"));
    arr.push(&JsValue::from_str("dodecahedron"));
    arr.push(&JsValue::from_str("icosahedron"));
    arr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_wasm() {
        let result = hello_wasm();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_create_platonic_solid_tetrahedron() {
        let geometry = create_platonic_solid("tetrahedron", "default").unwrap();
        assert_eq!(geometry.vertex_count(), 12); // 4 faces * 3 vertices
        assert_eq!(geometry.triangle_count(), 4); // 4 triangular faces
    }

    #[test]
    fn test_create_platonic_solid_hexahedron() {
        let geometry = create_platonic_solid("hexahedron", "default").unwrap();
        assert_eq!(geometry.vertex_count(), 24); // 6 faces * 4 vertices
        assert_eq!(geometry.triangle_count(), 12); // 6 quad faces = 12 triangles
    }

    #[test]
    fn test_create_platonic_solid_cube_alias() {
        let geometry = create_platonic_solid("cube", "default").unwrap();
        assert_eq!(geometry.triangle_count(), 12);
    }

    #[test]
    fn test_create_platonic_solid_octahedron() {
        let geometry = create_platonic_solid("octahedron", "default").unwrap();
        assert_eq!(geometry.vertex_count(), 24); // 8 faces * 3 vertices
        assert_eq!(geometry.triangle_count(), 8); // 8 triangular faces
    }

    #[test]
    fn test_create_platonic_solid_dodecahedron() {
        let geometry = create_platonic_solid("dodecahedron", "default").unwrap();
        assert_eq!(geometry.vertex_count(), 60); // 12 faces * 5 vertices
        assert_eq!(geometry.triangle_count(), 36); // 12 pentagonal faces = 36 triangles (3 per face)
    }

    #[test]
    fn test_create_platonic_solid_icosahedron() {
        let geometry = create_platonic_solid("icosahedron", "default").unwrap();
        assert_eq!(geometry.vertex_count(), 60); // 20 faces * 3 vertices
        assert_eq!(geometry.triangle_count(), 20); // 20 triangular faces
    }

    #[test]
    fn test_create_platonic_solid_case_insensitive() {
        let geometry = create_platonic_solid("HEXAHEDRON", "default").unwrap();
        assert_eq!(geometry.triangle_count(), 12);
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_create_platonic_solid_invalid_type() {
        let result = create_platonic_solid("invalid", "default");
        assert!(result.is_err());
    }

    #[test]
    fn test_geometry_data_not_empty() {
        let geometry = create_platonic_solid("tetrahedron", "default").unwrap();
        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.normals.is_empty());
        assert!(!geometry.uvs.is_empty());
        assert!(!geometry.indices.is_empty());
    }

    #[test]
    fn test_geometry_data_sizes_consistent() {
        let geometry = create_platonic_solid("hexahedron", "default").unwrap();
        // vertices and normals should have same length (3 components each)
        assert_eq!(geometry.vertices.len(), geometry.normals.len());
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        assert_eq!(geometry.uvs.len() * 3, geometry.vertices.len() * 2);
    }

    #[test]
    fn test_normals_are_normalized() {
        let geometry = create_platonic_solid("octahedron", "default").unwrap();
        for i in (0..geometry.normals.len()).step_by(3) {
            let nx = geometry.normals[i];
            let ny = geometry.normals[i + 1];
            let nz = geometry.normals[i + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            assert!((length - 1.0).abs() < 0.001, "Normal not normalized: length = {}", length);
        }
    }

    #[test]
    fn test_indices_within_bounds() {
        let geometry = create_platonic_solid("icosahedron", "default").unwrap();
        let vertex_count = geometry.vertex_count() as u32;
        for &index in &geometry.indices {
            assert!(index < vertex_count, "Index {} out of bounds (vertex count: {})", index, vertex_count);
        }
    }
}
