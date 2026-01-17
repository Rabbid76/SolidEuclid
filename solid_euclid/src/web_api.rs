use wasm_bindgen::prelude::*;

use crate::geometry_model::GeometryModel;
use crate::planar_profile::PlanarProfile;
use crate::platonic_solid::Model;
use crate::sphere::Model as SphereModel;
use crate::sweeps::{Capsule, ConicalFrustum, Cylinder, Extrude, Torus};
use crate::triangular_mesh::TriangularMesh;

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

/// Converts a `TriangularMesh` into a `WasmTriangularMesh`.
///
/// This helper function handles the common conversion pattern used across
/// all geometry creation functions, reducing code duplication.
fn triangular_mesh_to_wasm(triangular_mesh: TriangularMesh) -> WasmTriangularMesh {
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

    WasmTriangularMesh {
        material_ids,
        geometries,
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

    Ok(triangular_mesh_to_wasm(triangular_mesh))
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

/// Creates a triangular mesh for a Sphere model from a JavaScript object.
///
/// # Arguments
/// * `model` - A JavaScript object representing a `sphere::Model` with `geometry` and `material` fields.
///
/// # Example TypeScript
/// ```typescript
/// // UV Sphere (slices)
/// const mesh = create_sphere({
///   geometry: { Slices: { slices: 16, segments: 32 } },
///   material: { Uniform: "default" }
/// });
///
/// // Cube-mapped sphere
/// const mesh = create_sphere({
///   geometry: { Cube: { segments: 8 } },
///   material: { Array: ["mat0", "mat1", "mat2", "mat3", "mat4", "mat5"] }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_sphere(model: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let model: SphereModel = serde_wasm_bindgen::from_value(model)
        .map_err(|e| JsError::new(&format!("Failed to parse sphere model: {}", e)))?;

    let triangular_mesh = model
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create sphere mesh: {:?}", e)))?;

    Ok(triangular_mesh_to_wasm(triangular_mesh))
}

/// Returns a list of available Sphere types.
#[wasm_bindgen]
pub fn get_sphere_types() -> js_sys::Array {
    let arr = js_sys::Array::new();
    arr.push(&JsValue::from_str("Slices"));
    arr.push(&JsValue::from_str("Cube"));
    arr
}

/// Creates a triangular mesh for a Torus from a JavaScript object.
///
/// # Arguments
/// * `torus` - A JavaScript object representing a `Torus` with the following fields:
///   - `major_radius`: The distance from the center to the tube center (f64)
///   - `minor_radius`: The radius of the tube (f64)
///   - `major_segments`: Number of segments around the major circle (u32)
///   - `minor_segments`: Number of segments around the minor circle (u32)
///   - `material`: The material assignment
///
/// # Example TypeScript
/// ```typescript
/// const mesh = create_torus({
///   major_radius: 2.0,
///   minor_radius: 0.5,
///   major_segments: 36,
///   minor_segments: 18,
///   material: { Uniform: "default" }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_torus(torus: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let torus: Torus = serde_wasm_bindgen::from_value(torus)
        .map_err(|e| JsError::new(&format!("Failed to parse torus: {}", e)))?;

    let triangular_mesh = torus
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create torus mesh: {:?}", e)))?;

    Ok(triangular_mesh_to_wasm(triangular_mesh))
}

/// Creates a triangular mesh for a Capsule from a JavaScript object.
///
/// A capsule is a cylinder with hemispheres at both ends. It supports 3 material regions:
/// - Face 0: Bottom hemisphere
/// - Face 1: Cylindrical body
/// - Face 2: Top hemisphere
///
/// # Arguments
/// * `capsule` - A JavaScript object representing a `Capsule` with the following fields:
///   - `radius`: The radius of the cylinder and hemispheres (f64)
///   - `height`: The length of the cylindrical body (f64)
///   - `segments`: Number of segments for tessellation (u32)
///   - `material`: The material assignment
///
/// # Example TypeScript
/// ```typescript
/// // Capsule with uniform material
/// const mesh = create_capsule({
///   radius: 0.5,
///   height: 2.0,
///   segments: 24,
///   material: { Uniform: "default" }
/// });
///
/// // Capsule with 3 materials (bottom, body, top)
/// const mesh = create_capsule({
///   radius: 0.5,
///   height: 2.0,
///   segments: 24,
///   material: { Array: ["bottom", "body", "top"] }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_capsule(capsule: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let capsule: Capsule = serde_wasm_bindgen::from_value(capsule)
        .map_err(|e| JsError::new(&format!("Failed to parse capsule: {}", e)))?;

    let triangular_mesh = capsule
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create capsule mesh: {:?}", e)))?;

    Ok(triangular_mesh_to_wasm(triangular_mesh))
}

/// Creates a triangular mesh for a Cylinder from a JavaScript object.
///
/// A cylinder supports 5 material regions:
/// - Face 0: Bottom cap
/// - Face 1: Bottom edge (chamfer or fillet)
/// - Face 2: Cylindrical body
/// - Face 3: Top edge (chamfer or fillet)
/// - Face 4: Top cap
///
/// # Arguments
/// * `cylinder` - A JavaScript object representing a `Cylinder` with the following fields:
///   - `radius`: The radius of the cylinder (f64)
///   - `height`: The total height of the cylinder (f64)
///   - `segments`: Number of segments for tessellation (u32)
///   - `bottom_edge`: Corner treatment for bottom edge (object with type)
///   - `top_edge`: Corner treatment for top edge (object with type)
///   - `material`: The material assignment
///
/// # Example TypeScript
/// ```typescript
/// // Cylinder with sharp edges
/// const mesh = create_cylinder({
///   radius: 0.5,
///   height: 2.0,
///   segments: 24,
///   bottom_edge: "Edge",
///   top_edge: "Edge",
///   material: { Uniform: "default" }
/// });
///
/// // Cylinder with chamfered edges
/// const mesh = create_cylinder({
///   radius: 0.5,
///   height: 2.0,
///   segments: 24,
///   bottom_edge: { Chamfer: { distance: 0.1 } },
///   top_edge: { Chamfer: { distance: 0.1 } },
///   material: { Array: ["bottom_cap", "bottom_edge", "body", "top_edge", "top_cap"] }
/// });
///
/// // Cylinder with filleted edges
/// const mesh = create_cylinder({
///   radius: 0.5,
///   height: 2.0,
///   segments: 24,
///   bottom_edge: { Fillet: { radius: 0.1 } },
///   top_edge: { Fillet: { radius: 0.1 } },
///   material: { Uniform: "metal" }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_cylinder(cylinder: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let cylinder: Cylinder = serde_wasm_bindgen::from_value(cylinder)
        .map_err(|e| JsError::new(&format!("Failed to parse cylinder: {}", e)))?;

    let triangular_mesh = cylinder
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create cylinder mesh: {:?}", e)))?;

    Ok(triangular_mesh_to_wasm(triangular_mesh))
}

/// Creates a triangular mesh for a ConicalFrustum from a JavaScript object.
///
/// A conical frustum (truncated cone) supports 5 material regions:
/// - Face 0: Bottom cap
/// - Face 1: Bottom edge (chamfer or fillet)
/// - Face 2: Conical body
/// - Face 3: Top edge (chamfer or fillet)
/// - Face 4: Top cap
///
/// # Arguments
/// * `frustum` - A JavaScript object representing a `ConicalFrustum` with the following fields:
///   - `bottom_radius`: The radius at the bottom of the frustum (f64)
///   - `top_radius`: The radius at the top of the frustum (f64)
///   - `height`: The total height of the frustum (f64)
///   - `segments`: Number of segments for tessellation (u32)
///   - `bottom_edge`: Corner treatment for bottom edge (object with type)
///   - `top_edge`: Corner treatment for top edge (object with type)
///   - `material`: The material assignment
///
/// # Example TypeScript
/// ```typescript
/// // Conical frustum with sharp edges
/// const mesh = create_conical_frustum({
///   bottom_radius: 0.5,
///   top_radius: 0.25,
///   height: 2.0,
///   segments: 24,
///   bottom_edge: "Edge",
///   top_edge: "Edge",
///   material: { Uniform: "default" }
/// });
///
/// // Conical frustum with chamfered edges
/// const mesh = create_conical_frustum({
///   bottom_radius: 0.5,
///   top_radius: 0.25,
///   height: 2.0,
///   segments: 24,
///   bottom_edge: { Chamfer: { distance: 0.1 } },
///   top_edge: { Chamfer: { distance: 0.1 } },
///   material: { Array: ["bottom_cap", "bottom_edge", "body", "top_edge", "top_cap"] }
/// });
///
/// // Cone (top_radius = 0)
/// const mesh = create_conical_frustum({
///   bottom_radius: 0.5,
///   top_radius: 0.0,
///   height: 2.0,
///   segments: 24,
///   bottom_edge: "Edge",
///   top_edge: "Edge",
///   material: { Uniform: "default" }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_conical_frustum(frustum: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let frustum: ConicalFrustum = serde_wasm_bindgen::from_value(frustum)
        .map_err(|e| JsError::new(&format!("Failed to parse conical frustum: {}", e)))?;

    let triangular_mesh = frustum
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create conical frustum mesh: {:?}", e)))?;

    Ok(triangular_mesh_to_wasm(triangular_mesh))
}

/// Creates a triangular mesh for an Extrude from a JavaScript object.
///
/// An extrude is created by sweeping a 2D profile along the Y-axis.
/// The extrusion goes from -height/2 to +height/2, centered at the origin.
///
/// The extrude supports separate materials for:
/// - Side faces (geometry_id based on profile segment IDs)
/// - Bottom cap (geometry_id = -1)
/// - Top cap (geometry_id = -2)
///
/// # Arguments
/// * `extrude` - A JavaScript object representing an `Extrude` with the following fields:
///   - `profile`: The 2D profile to extrude (PlanarProfile with commands)
///   - `height`: The total height of the extrusion (f64)
///   - `sample_distance`: The sample distance for profile curves (f64)
///   - `material`: The material assignment
///
/// # Example TypeScript
/// ```typescript
/// // Simple square extrusion with uniform material
/// const mesh = create_extrude({
///   profile: {
///     commands: [
///       { path: { Move: { point: [0.0, 0.0] } }, corner: "Edge" },
///       { path: { Line: { point: [1.0, 0.0] } }, corner: "Edge" },
///       { path: { Line: { point: [1.0, 1.0] } }, corner: "Edge" },
///       { path: { Line: { point: [0.0, 1.0] } }, corner: "Edge" },
///       { path: { Line: { point: [0.0, 0.0] } }, corner: "Edge" },
///     ]
///   },
///   height: 2.0,
///   sample_distance: 0.1,
///   material: { Uniform: "default" }
/// });
///
/// // Triangle extrusion with filleted corners
/// const mesh = create_extrude({
///   profile: {
///     commands: [
///       { path: { Move: { point: [0.0, 0.0] } }, corner: "Edge" },
///       { path: { Line: { point: [1.0, 0.0] } }, corner: { Fillet: { radius: 0.1 } } },
///       { path: { Line: { point: [0.5, 0.866] } }, corner: { Fillet: { radius: 0.1 } } },
///       { path: { Line: { point: [0.0, 0.0] } }, corner: { Fillet: { radius: 0.1 } } },
///     ]
///   },
///   height: 1.5,
///   sample_distance: 0.05,
///   material: { Uniform: "metal" }
/// });
/// ```
///
/// # Returns
/// A `WasmTriangularMesh` containing the mesh data organized by material.
#[wasm_bindgen]
pub fn create_extrude(extrude: JsValue) -> Result<WasmTriangularMesh, JsError> {
    let extrude: Extrude = serde_wasm_bindgen::from_value(extrude)
        .map_err(|e| JsError::new(&format!("Failed to parse extrude: {}", e)))?;

    let triangular_mesh = extrude
        .to_triangular_mesh()
        .map_err(|e| JsError::new(&format!("Failed to create extrude mesh: {:?}", e)))?;

    Ok(triangular_mesh_to_wasm(triangular_mesh))
}

/// Creates a PlanarProfile from a JavaScript object.
///
/// This is a utility function for creating profiles programmatically from JavaScript.
///
/// # Arguments
/// * `profile` - A JavaScript object representing a `PlanarProfile` with commands
///
/// # Returns
/// A serialized PlanarProfile object that can be used with create_extrude
#[wasm_bindgen]
pub fn create_planar_profile(profile: JsValue) -> Result<JsValue, JsError> {
    let profile: PlanarProfile = serde_wasm_bindgen::from_value(profile)
        .map_err(|e| JsError::new(&format!("Failed to parse profile: {}", e)))?;

    serde_wasm_bindgen::to_value(&profile)
        .map_err(|e| JsError::new(&format!("Failed to serialize profile: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planar_profile::{Corner, PathCommand, ProfileCommand};
    use crate::platonic_solid::{Material, PlatonicSolid};
    use crate::sphere::Sphere;
    use glam::DVec2;

    #[test]
    fn test_hello_wasm() {
        let result = hello_wasm();
        assert_eq!(result, "Hello World!");
    }

    // Helper to create a Model directly for testing (since we can't create JsValue in tests)
    fn create_mesh_from_model(model: Model) -> WasmTriangularMesh {
        let triangular_mesh = model.to_triangular_mesh().unwrap();
        triangular_mesh_to_wasm(triangular_mesh)
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

    // ==================== Sphere Tests ====================

    // Helper to create a WasmTriangularMesh from a SphereModel directly for testing
    fn create_mesh_from_sphere_model(model: SphereModel) -> WasmTriangularMesh {
        let triangular_mesh = model.to_triangular_mesh().unwrap();
        triangular_mesh_to_wasm(triangular_mesh)
    }

    #[test]
    fn test_create_sphere_slices_uniform() {
        let model = SphereModel::new(
            Sphere::Slices {
                slices: 8,
                segments: 16,
            },
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids.len(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geometry = mesh.get_geometry(0).unwrap();
        // Slices sphere: (slices * segments * 2) triangles (quads = 2 triangles each)
        // Plus 2 * segments triangles for caps
        // 8 slices * 16 segments * 2 = 256 triangles (for middle sections)
        // Plus caps: 16 + 16 = 32 triangles
        // But actually: (slices-1) * segments * 2 + 2 * segments for polar caps
        // = 7 * 16 * 2 + 2 * 16 = 224 + 32 = 256
        assert!(geometry.triangle_count() > 0);
        assert!(geometry.vertex_count() > 0);
    }

    #[test]
    fn test_create_sphere_slices_default_material() {
        // Slices sphere has only 1 face, so default material results in 1 geometry
        let model = SphereModel::new(
            Sphere::Slices {
                slices: 8,
                segments: 16,
            },
            Material::Default,
        );
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 1); // Slices sphere has 1 face
    }

    #[test]
    fn test_create_sphere_cube_uniform() {
        let model = SphereModel::new(
            Sphere::Cube { segments: 4 },
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 1);

        let geometry = mesh.get_geometry(0).unwrap();
        // Cube sphere: 6 faces * segments^2 * 2 triangles per quad
        // = 6 * 16 * 2 = 192 triangles
        assert_eq!(geometry.triangle_count(), 6 * 4 * 4 * 2);
    }

    #[test]
    fn test_create_sphere_cube_array_6_materials() {
        let materials = vec![
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
            "mat3".to_string(),
            "mat4".to_string(),
            "mat5".to_string(),
        ];
        let model = SphereModel::new(Sphere::Cube { segments: 4 }, Material::Array(materials));
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 6); // 6 faces, 6 different materials

        // Each face should have segments^2 * 2 triangles
        for i in 0..6 {
            let geometry = mesh.get_geometry(i).unwrap();
            assert_eq!(geometry.triangle_count(), 4 * 4 * 2); // 32 triangles per face
        }
    }

    #[test]
    fn test_create_sphere_cube_default_material() {
        // Cube sphere has 6 faces, so default material results in 6 geometries
        let model = SphereModel::new(Sphere::Cube { segments: 4 }, Material::Default);
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 6); // 6 faces, each with unique default material
    }

    #[test]
    fn test_create_sphere_cube_cyclic_materials() {
        // Using 3 materials cyclically for 6 faces should result in 3 geometries
        let materials = vec![
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
        ];
        let model = SphereModel::new(Sphere::Cube { segments: 4 }, Material::Array(materials));
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 3); // 3 unique materials
    }

    #[test]
    fn test_sphere_geometry_data_not_empty() {
        let model = SphereModel::new(
            Sphere::Slices {
                slices: 8,
                segments: 16,
            },
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_sphere_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.normals.is_empty());
        assert!(!geometry.uvs.is_empty());
        assert!(!geometry.indices.is_empty());
    }

    #[test]
    fn test_sphere_geometry_data_sizes_consistent() {
        let model = SphereModel::new(
            Sphere::Cube { segments: 4 },
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_sphere_model(model);
        let geometry = mesh.get_geometry(0).unwrap();
        // vertices and normals should have same length (3 components each)
        assert_eq!(geometry.vertices.len(), geometry.normals.len());
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        assert_eq!(geometry.uvs.len() * 3, geometry.vertices.len() * 2);
    }

    #[test]
    fn test_sphere_normals_are_normalized() {
        let model = SphereModel::new(
            Sphere::Slices {
                slices: 8,
                segments: 16,
            },
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_sphere_model(model);
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
    fn test_sphere_indices_within_bounds() {
        let model = SphereModel::new(
            Sphere::Cube { segments: 4 },
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_sphere_model(model);
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
    fn test_sphere_cube_total_triangles() {
        // Ensure total triangles across all geometries is correct for cube sphere
        let materials = vec![
            "m0".to_string(),
            "m1".to_string(),
            "m2".to_string(),
            "m3".to_string(),
            "m4".to_string(),
            "m5".to_string(),
        ];
        let model = SphereModel::new(Sphere::Cube { segments: 4 }, Material::Array(materials));
        let mesh = create_mesh_from_sphere_model(model);

        let total_triangles: usize = (0..mesh.geometry_count())
            .filter_map(|i| mesh.get_geometry(i))
            .map(|g| g.triangle_count())
            .sum();

        // 6 faces * 4*4 quads * 2 triangles = 192 triangles
        assert_eq!(total_triangles, 6 * 4 * 4 * 2);
    }

    #[test]
    fn test_sphere_get_geometry_by_material() {
        let materials = vec![
            "mat_a".to_string(),
            "mat_b".to_string(),
            "mat_a".to_string(),
            "mat_b".to_string(),
            "mat_a".to_string(),
            "mat_b".to_string(),
        ];
        let model = SphereModel::new(Sphere::Cube { segments: 4 }, Material::Array(materials));
        let mesh = create_mesh_from_sphere_model(model);

        let geometry_a = mesh.get_geometry_by_material("mat_a");
        let geometry_b = mesh.get_geometry_by_material("mat_b");
        let geometry_c = mesh.get_geometry_by_material("mat_c");

        assert!(geometry_a.is_some());
        assert!(geometry_b.is_some());
        assert!(geometry_c.is_none());
    }

    #[test]
    fn test_sphere_map_material() {
        use std::collections::HashMap;
        let mut face_map = HashMap::new();
        face_map.insert(0, "special".to_string());
        face_map.insert(2, "special".to_string());
        let model = SphereModel::new(
            Sphere::Cube { segments: 4 },
            Material::Map {
                default: "base".to_string(),
                face_map,
            },
        );
        let mesh = create_mesh_from_sphere_model(model);
        assert_eq!(mesh.geometry_count(), 2); // "base" and "special"
    }

    // ==================== Torus Tests ====================

    // Helper to create a WasmTriangularMesh from a Torus directly for testing
    fn create_mesh_from_torus(torus: Torus) -> WasmTriangularMesh {
        let triangular_mesh = torus.to_triangular_mesh().unwrap();

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
    fn test_create_torus_uniform_material() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);

        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids.len(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_torus_default_material() {
        let torus = Torus::new(2.0, 0.5, 12, 8, Material::Default);
        let mesh = create_mesh_from_torus(torus);

        // Torus with default material creates unique material per face
        assert!(mesh.geometry_count() > 0);
    }

    #[test]
    fn test_create_torus_geometry_data_not_empty() {
        let torus = Torus::new(2.0, 0.5, 24, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);
        let geometry = mesh.get_geometry(0).unwrap();

        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.normals.is_empty());
        assert!(!geometry.uvs.is_empty());
        assert!(!geometry.indices.is_empty());
    }

    #[test]
    fn test_create_torus_geometry_data_sizes_consistent() {
        let torus = Torus::new(2.0, 0.5, 24, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);
        let geometry = mesh.get_geometry(0).unwrap();

        // vertices and normals should have same length (3 components each)
        assert_eq!(geometry.vertices.len(), geometry.normals.len());
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        assert_eq!(geometry.uvs.len() * 3, geometry.vertices.len() * 2);
    }

    #[test]
    fn test_create_torus_normals_are_normalized() {
        let torus = Torus::new(2.0, 0.5, 24, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);
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
    fn test_create_torus_indices_within_bounds() {
        let torus = Torus::new(2.0, 0.5, 24, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);
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
    fn test_create_torus_different_segment_counts() {
        let low_res = Torus::new(2.0, 0.5, 8, 6, Material::Uniform("default".to_string()));
        let high_res = Torus::new(2.0, 0.5, 48, 24, Material::Uniform("default".to_string()));

        let low_mesh = create_mesh_from_torus(low_res);
        let high_mesh = create_mesh_from_torus(high_res);

        let low_geo = low_mesh.get_geometry(0).unwrap();
        let high_geo = high_mesh.get_geometry(0).unwrap();

        // Higher resolution should have more triangles
        assert!(high_geo.triangle_count() > low_geo.triangle_count());
    }

    #[test]
    fn test_create_torus_thin_ring() {
        // Test a torus where minor radius is much smaller than major radius
        let torus = Torus::new(10.0, 0.1, 48, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_torus_fat_ring() {
        // Test a torus where minor radius approaches major radius
        let torus = Torus::new(1.0, 0.9, 24, 24, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_torus(torus);

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_torus_get_geometry_by_material() {
        let torus = Torus::new(
            2.0,
            0.5,
            12,
            8,
            Material::Uniform("my_material".to_string()),
        );
        let mesh = create_mesh_from_torus(torus);

        let geometry = mesh.get_geometry_by_material("my_material");
        let missing = mesh.get_geometry_by_material("nonexistent");

        assert!(geometry.is_some());
        assert!(missing.is_none());
    }

    #[test]
    fn test_create_torus_triangle_count_relationship() {
        // For a torus with m major segments and n minor segments,
        // we should have approximately m * n * 2 triangles (2 triangles per quad)
        let major_segs = 12u32;
        let minor_segs = 8u32;
        let torus = Torus::new(
            2.0,
            0.5,
            major_segs,
            minor_segs,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_torus(torus);
        let geometry = mesh.get_geometry(0).unwrap();

        // The exact number depends on the sampling, but should be proportional
        // to major_segments * minor_segments * 2
        let expected_min = (major_segs * minor_segs * 2) as usize;
        assert!(
            geometry.triangle_count() >= expected_min / 2,
            "Triangle count {} is too low (expected at least {})",
            geometry.triangle_count(),
            expected_min / 2
        );
    }

    // ==================== Capsule Tests ====================

    // Helper to create a WasmTriangularMesh from a Capsule directly for testing
    fn create_mesh_from_capsule(capsule: Capsule) -> WasmTriangularMesh {
        let triangular_mesh = capsule.to_triangular_mesh().unwrap();

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
    fn test_create_capsule_uniform_material() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);

        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids.len(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_capsule_default_material() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Default);
        let mesh = create_mesh_from_capsule(capsule);

        // Default material creates 3 regions for capsule
        assert_eq!(mesh.geometry_count(), 3);
    }

    #[test]
    fn test_create_capsule_array_3_materials() {
        let materials = vec!["bottom".to_string(), "body".to_string(), "top".to_string()];
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Array(materials));
        let mesh = create_mesh_from_capsule(capsule);

        assert_eq!(mesh.geometry_count(), 3);
        assert!(mesh.material_ids.contains(&"bottom".to_string()));
        assert!(mesh.material_ids.contains(&"body".to_string()));
        assert!(mesh.material_ids.contains(&"top".to_string()));
    }

    #[test]
    fn test_create_capsule_geometry_data_not_empty() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);
        let geometry = mesh.get_geometry(0).unwrap();

        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.normals.is_empty());
        assert!(!geometry.uvs.is_empty());
        assert!(!geometry.indices.is_empty());
    }

    #[test]
    fn test_create_capsule_geometry_data_sizes_consistent() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);
        let geometry = mesh.get_geometry(0).unwrap();

        // vertices and normals should have same length (3 components each)
        assert_eq!(geometry.vertices.len(), geometry.normals.len());
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        assert_eq!(geometry.uvs.len() * 3, geometry.vertices.len() * 2);
    }

    #[test]
    fn test_create_capsule_normals_are_normalized() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);
        let geometry = mesh.get_geometry(0).unwrap();

        for i in (0..geometry.normals.len()).step_by(3) {
            let nx = geometry.normals[i];
            let ny = geometry.normals[i + 1];
            let nz = geometry.normals[i + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();

            assert!(
                (length - 1.0).abs() < 0.01,
                "Normal at index {} should be normalized, but length is {}",
                i / 3,
                length
            );
        }
    }

    #[test]
    fn test_create_capsule_indices_within_bounds() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);
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
    fn test_create_capsule_zero_height() {
        // A capsule with zero height is essentially a sphere
        let capsule = Capsule::new(0.5, 0.0, 12, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_capsule_tall() {
        // Test a tall capsule
        let capsule = Capsule::new(0.5, 10.0, 24, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_capsule(capsule);

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_capsule_get_geometry_by_material() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Uniform("my_material".to_string()));
        let mesh = create_mesh_from_capsule(capsule);

        let geometry = mesh.get_geometry_by_material("my_material");
        let missing = mesh.get_geometry_by_material("nonexistent");

        assert!(geometry.is_some());
        assert!(missing.is_none());
    }

    #[test]
    fn test_create_capsule_different_segment_counts() {
        let low_res = Capsule::new(0.5, 2.0, 6, Material::Uniform("default".to_string()));
        let high_res = Capsule::new(0.5, 2.0, 24, Material::Uniform("default".to_string()));

        let low_mesh = create_mesh_from_capsule(low_res);
        let high_mesh = create_mesh_from_capsule(high_res);

        let low_geo = low_mesh.get_geometry(0).unwrap();
        let high_geo = high_mesh.get_geometry(0).unwrap();

        // Higher resolution should have more triangles
        assert!(high_geo.triangle_count() > low_geo.triangle_count());
    }

    // =====================================================
    // Cylinder Tests
    // =====================================================

    fn create_mesh_from_cylinder(cylinder: Cylinder) -> WasmTriangularMesh {
        let triangular_mesh = cylinder
            .to_triangular_mesh()
            .expect("Failed to create cylinder mesh");

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
    fn test_create_cylinder_uniform_material() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids.len(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_cylinder_default_material_sharp_edges() {
        let cylinder = Cylinder::new(0.5, 2.0, 12, Corner::Edge, Corner::Edge, Material::Default);
        let mesh = create_mesh_from_cylinder(cylinder);

        // With sharp edges, cylinder has 3 material regions (bottom_cap, body, top_cap)
        assert_eq!(mesh.geometry_count(), 3);
    }

    #[test]
    fn test_create_cylinder_default_material_with_chamfer() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        // With chamfered edges, cylinder has 5 material regions
        assert_eq!(mesh.geometry_count(), 5);
    }

    #[test]
    fn test_create_cylinder_default_material_with_fillet() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        // With filleted edges, cylinder has 5 material regions
        assert_eq!(mesh.geometry_count(), 5);
    }

    #[test]
    fn test_create_cylinder_array_5_materials() {
        let materials = vec![
            "bottom_cap".to_string(),
            "bottom_edge".to_string(),
            "body".to_string(),
            "top_edge".to_string(),
            "top_cap".to_string(),
        ];
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Array(materials),
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        assert_eq!(mesh.geometry_count(), 5);
        assert!(mesh.material_ids.contains(&"bottom_cap".to_string()));
        assert!(mesh.material_ids.contains(&"bottom_edge".to_string()));
        assert!(mesh.material_ids.contains(&"body".to_string()));
        assert!(mesh.material_ids.contains(&"top_edge".to_string()));
        assert!(mesh.material_ids.contains(&"top_cap".to_string()));
    }

    #[test]
    fn test_create_cylinder_mixed_edges() {
        // Chamfer on bottom, Fillet on top
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        // With both edge types, should have 5 material regions
        assert_eq!(mesh.geometry_count(), 5);
    }

    #[test]
    fn test_create_cylinder_one_edge_sharp() {
        // Sharp bottom, chamfered top
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        // With one sharp edge, should have 4 material regions
        assert_eq!(mesh.geometry_count(), 4);
    }

    #[test]
    fn test_create_cylinder_geometry_data_not_empty() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_cylinder(cylinder);
        let geometry = mesh.get_geometry(0).unwrap();

        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.normals.is_empty());
        assert!(!geometry.uvs.is_empty());
        assert!(!geometry.indices.is_empty());
    }

    #[test]
    fn test_create_cylinder_geometry_data_sizes_consistent() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_cylinder(cylinder);
        let geometry = mesh.get_geometry(0).unwrap();

        // vertices and normals should have same length (3 components each)
        assert_eq!(geometry.vertices.len(), geometry.normals.len());
        // uvs should have 2/3 the length of vertices (2 components vs 3)
        assert_eq!(geometry.uvs.len() * 3, geometry.vertices.len() * 2);
    }

    #[test]
    fn test_create_cylinder_normals_are_normalized() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_cylinder(cylinder);
        let geometry = mesh.get_geometry(0).unwrap();

        for i in (0..geometry.normals.len()).step_by(3) {
            let nx = geometry.normals[i];
            let ny = geometry.normals[i + 1];
            let nz = geometry.normals[i + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();

            assert!(
                (length - 1.0).abs() < 0.01,
                "Normal at index {} should be normalized, but length is {}",
                i / 3,
                length
            );
        }
    }

    #[test]
    fn test_create_cylinder_indices_within_bounds() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_cylinder(cylinder);
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
    fn test_create_cylinder_get_geometry_by_material() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("my_material".to_string()),
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        let geometry = mesh.get_geometry_by_material("my_material");
        let missing = mesh.get_geometry_by_material("nonexistent");

        assert!(geometry.is_some());
        assert!(missing.is_none());
    }

    #[test]
    fn test_create_cylinder_different_segment_counts() {
        let low_res = Cylinder::new(
            0.5,
            2.0,
            6,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let high_res = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );

        let low_mesh = create_mesh_from_cylinder(low_res);
        let high_mesh = create_mesh_from_cylinder(high_res);

        let low_geo = low_mesh.get_geometry(0).unwrap();
        let high_geo = high_mesh.get_geometry(0).unwrap();

        // Higher resolution should have more triangles
        assert!(high_geo.triangle_count() > low_geo.triangle_count());
    }

    #[test]
    fn test_create_cylinder_smooth_edges() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::SmoothEdge,
            Corner::SmoothEdge,
            Material::Default,
        );
        let mesh = create_mesh_from_cylinder(cylinder);

        // Smooth edges behave like sharp edges (no extra segments)
        assert_eq!(mesh.geometry_count(), 3);
    }

    // =====================================================
    // ConicalFrustum Tests
    // =====================================================

    /// Helper to create a WasmTriangularMesh from a ConicalFrustum directly
    fn create_mesh_from_conical_frustum(frustum: ConicalFrustum) -> WasmTriangularMesh {
        let triangular_mesh = frustum.to_triangular_mesh().unwrap();
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
    fn test_create_conical_frustum_uniform_material() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geo = mesh.geometries.get(0).unwrap();
        assert!(geo.vertex_count() > 0);
        assert!(geo.triangle_count() > 0);
    }

    #[test]
    fn test_create_conical_frustum_default_material_sharp_edges() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // Sharp edges: only 3 regions (bottom_cap, body, top_cap)
        assert_eq!(mesh.geometry_count(), 3);
    }

    #[test]
    fn test_create_conical_frustum_default_material_with_chamfer() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // Chamfered edges: 5 regions
        assert_eq!(mesh.geometry_count(), 5);
    }

    #[test]
    fn test_create_conical_frustum_default_material_with_fillet() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // Filleted edges: 5 regions
        assert_eq!(mesh.geometry_count(), 5);
    }

    #[test]
    fn test_create_conical_frustum_array_5_materials() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Array(vec![
                "bottom_cap".to_string(),
                "bottom_edge".to_string(),
                "body".to_string(),
                "top_edge".to_string(),
                "top_cap".to_string(),
            ]),
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        assert_eq!(mesh.geometry_count(), 5);
        assert!(mesh.material_ids.contains(&"bottom_cap".to_string()));
        assert!(mesh.material_ids.contains(&"bottom_edge".to_string()));
        assert!(mesh.material_ids.contains(&"body".to_string()));
        assert!(mesh.material_ids.contains(&"top_edge".to_string()));
        assert!(mesh.material_ids.contains(&"top_cap".to_string()));
    }

    #[test]
    fn test_create_conical_frustum_mixed_edges() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // Both edges have corners: 5 regions
        assert_eq!(mesh.geometry_count(), 5);
    }

    #[test]
    fn test_create_conical_frustum_one_edge_sharp() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Edge,
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // One sharp edge, one chamfered: 4 regions
        assert_eq!(mesh.geometry_count(), 4);
    }

    #[test]
    fn test_create_conical_frustum_geometry_data_not_empty() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        let geo = mesh.geometries.get(0).unwrap();
        assert!(!geo.vertices.is_empty());
        assert!(!geo.normals.is_empty());
        assert!(!geo.uvs.is_empty());
        assert!(!geo.indices.is_empty());
    }

    #[test]
    fn test_create_conical_frustum_geometry_data_sizes_consistent() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        let geo = mesh.geometries.get(0).unwrap();

        // vertices and normals should have same length (3 components per vertex)
        assert_eq!(geo.vertices.len(), geo.normals.len());
        // uvs have 2 components per vertex, vertices have 3
        assert_eq!(geo.uvs.len() * 3, geo.vertices.len() * 2);
    }

    #[test]
    fn test_create_conical_frustum_cone_shape() {
        // Test a cone (top_radius = 0)
        let frustum = ConicalFrustum::new(
            0.5,
            0.0,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        assert_eq!(mesh.geometry_count(), 1);
        let geo = mesh.geometries.get(0).unwrap();
        assert!(geo.vertex_count() > 0);
    }

    #[test]
    fn test_create_conical_frustum_inverted_cone_shape() {
        // Test an inverted cone (bottom_radius = 0)
        let frustum = ConicalFrustum::new(
            0.0,
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("default".to_string()),
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        assert_eq!(mesh.geometry_count(), 1);
        let geo = mesh.geometries.get(0).unwrap();
        assert!(geo.vertex_count() > 0);
    }

    #[test]
    fn test_create_conical_frustum_equal_radii_is_cylinder_like() {
        // When both radii are equal, it behaves like a cylinder
        let frustum = ConicalFrustum::new(
            0.5,
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // Sharp edges: 3 regions
        assert_eq!(mesh.geometry_count(), 3);
    }

    #[test]
    fn test_create_conical_frustum_smooth_edges() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::SmoothEdge,
            Corner::SmoothEdge,
            Material::Default,
        );
        let mesh = create_mesh_from_conical_frustum(frustum);

        // Smooth edges behave like sharp edges (no extra segments)
        assert_eq!(mesh.geometry_count(), 3);
    }

    // ==================== Extrude Tests ====================

    // Helper function to create a square profile for testing
    fn create_square_profile() -> PlanarProfile {
        PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ])
    }

    // Helper function to create a triangle profile for testing
    fn create_triangle_profile() -> PlanarProfile {
        PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 0.866),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ])
    }

    // Helper to create a WasmTriangularMesh from an Extrude directly for testing
    fn create_mesh_from_extrude(extrude: Extrude) -> WasmTriangularMesh {
        let triangular_mesh = extrude.to_triangular_mesh().unwrap();

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
    fn test_create_extrude_square_uniform_material() {
        let profile = create_square_profile();
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        assert_eq!(mesh.geometry_count(), 1);
        assert_eq!(mesh.material_ids.len(), 1);
        assert_eq!(mesh.material_ids[0], "default");

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_extrude_triangle_uniform_material() {
        let profile = create_triangle_profile();
        let extrude = Extrude::new(profile, 1.5, 0.1, Material::Uniform("metal".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        assert_eq!(mesh.geometry_count(), 1);

        let geometry = mesh.get_geometry(0).unwrap();
        assert!(geometry.vertex_count() > 0);
        assert!(geometry.triangle_count() > 0);
    }

    #[test]
    fn test_create_extrude_default_material() {
        let profile = create_square_profile();
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = create_mesh_from_extrude(extrude);

        // Default material creates separate materials for each profile segment and caps
        assert!(mesh.geometry_count() >= 3); // At least sides + bottom cap + top cap
    }

    #[test]
    fn test_create_extrude_array_materials() {
        let profile = create_square_profile();
        // Array materials: [body, bottom_cap, top_cap]
        let materials = vec![
            "body".to_string(),
            "bottom_cap".to_string(),
            "top_cap".to_string(),
        ];
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Array(materials));
        let mesh = create_mesh_from_extrude(extrude);

        assert!(mesh.geometry_count() >= 1);
    }

    #[test]
    fn test_create_extrude_geometry_data_not_empty() {
        let profile = create_square_profile();
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        let geo = mesh.geometries.get(0).unwrap();
        assert!(!geo.vertices.is_empty());
        assert!(!geo.normals.is_empty());
        assert!(!geo.uvs.is_empty());
        assert!(!geo.indices.is_empty());
    }

    #[test]
    fn test_create_extrude_geometry_data_sizes_consistent() {
        let profile = create_square_profile();
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        let geo = mesh.geometries.get(0).unwrap();

        // vertices and normals should have same length (3 components per vertex)
        assert_eq!(geo.vertices.len(), geo.normals.len());
        // uvs have 2 components per vertex, vertices have 3
        assert_eq!(geo.uvs.len() * 3, geo.vertices.len() * 2);
    }

    #[test]
    fn test_create_extrude_normals_normalized() {
        let profile = create_square_profile();
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

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
    fn test_create_extrude_indices_within_bounds() {
        let profile = create_square_profile();
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

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
    fn test_create_extrude_different_heights() {
        // Test with different heights
        for height in [0.5, 1.0, 2.0, 5.0] {
            let profile = create_square_profile();
            let extrude = Extrude::new(
                profile,
                height,
                0.1,
                Material::Uniform("default".to_string()),
            );
            let mesh = create_mesh_from_extrude(extrude);

            assert_eq!(mesh.geometry_count(), 1);
            let geo = mesh.geometries.get(0).unwrap();
            assert!(geo.vertex_count() > 0);
        }
    }

    #[test]
    fn test_create_extrude_different_sample_distances() {
        // Test with different sample distances
        for sample_distance in [0.05, 0.1, 0.2, 0.5] {
            let profile = create_square_profile();
            let extrude = Extrude::new(
                profile,
                2.0,
                sample_distance,
                Material::Uniform("default".to_string()),
            );
            let mesh = create_mesh_from_extrude(extrude);

            assert_eq!(mesh.geometry_count(), 1);
            let geo = mesh.geometries.get(0).unwrap();
            assert!(geo.vertex_count() > 0);
        }
    }

    #[test]
    fn test_create_extrude_with_filleted_corners() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
        ]);
        let extrude = Extrude::new(profile, 2.0, 0.05, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        assert_eq!(mesh.geometry_count(), 1);
        let geo = mesh.geometries.get(0).unwrap();
        assert!(geo.vertex_count() > 0);
        // Filleted corners should produce more vertices than sharp corners
    }

    #[test]
    fn test_create_extrude_with_chamfered_corners() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
        ]);
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        assert_eq!(mesh.geometry_count(), 1);
        let geo = mesh.geometries.get(0).unwrap();
        assert!(geo.vertex_count() > 0);
    }

    #[test]
    fn test_create_extrude_get_geometry_by_material() {
        let profile = create_square_profile();
        let extrude = Extrude::new(
            profile,
            2.0,
            0.1,
            Material::Uniform("my_material".to_string()),
        );
        let mesh = create_mesh_from_extrude(extrude);

        let geometry = mesh.get_geometry_by_material("my_material");
        let missing = mesh.get_geometry_by_material("nonexistent");

        assert!(geometry.is_some());
        assert!(missing.is_none());
    }

    #[test]
    fn test_create_extrude_triangle_with_chamfer_and_triangle_hole_with_chamfer() {
        // This test verifies the exact user scenario that was failing:
        // 3-point outline with chamfer corners + 3-point hole with chamfer corners
        let chamfer_distance = 0.1;
        let hole_chamfer_distance = 0.03;

        // Outer triangle (CCW winding) - radius 0.8
        let outer_radius = 0.8;
        let outer_points: Vec<DVec2> = (0..3)
            .map(|i| {
                let angle =
                    (i as f64 / 3.0) * std::f64::consts::PI * 2.0 - std::f64::consts::PI / 2.0;
                DVec2::new(angle.cos() * outer_radius, angle.sin() * outer_radius)
            })
            .collect();

        // Inner triangle (CW winding for hole) - radius 0.3
        let inner_radius = 0.3;
        let inner_points: Vec<DVec2> = (0..3)
            .map(|i| {
                let angle =
                    (i as f64 / 3.0) * std::f64::consts::PI * 2.0 - std::f64::consts::PI / 2.0;
                DVec2::new(angle.cos() * inner_radius, angle.sin() * inner_radius)
            })
            .collect();

        // Build profile commands
        let profile = PlanarProfile::with_commands(vec![
            // Outer triangle
            ProfileCommand::new(
                PathCommand::Move {
                    point: outer_points[0],
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: outer_points[1],
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: outer_points[2],
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: outer_points[0],
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            // Inner triangle (reversed for hole - CW winding)
            ProfileCommand::new(
                PathCommand::Move {
                    point: inner_points[0],
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: inner_points[2],
                }, // Reversed
                Corner::Chamfer {
                    distance: hole_chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: inner_points[1],
                }, // Reversed
                Corner::Chamfer {
                    distance: hole_chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: inner_points[0],
                },
                Corner::Chamfer {
                    distance: hole_chamfer_distance,
                },
            ),
        ]);

        // Verify profile converts to curve set correctly
        let curve_set = profile.to_curve_set();
        assert!(curve_set.is_ok(), "Profile should convert to curve set");
        let curve_set = curve_set.unwrap();

        // Should have 2 curves (outer + hole)
        assert_eq!(
            curve_set.curves.len(),
            2,
            "Should have 2 curves (outline + hole)"
        );

        // Both curves should have 6 segments (3 edges + 3 chamfers)
        assert_eq!(
            curve_set.curves[0].segments.len(),
            6,
            "Outer triangle with 3 chamfers should have 6 segments"
        );
        assert_eq!(
            curve_set.curves[1].segments.len(),
            6,
            "Inner triangle with 3 chamfers should have 6 segments"
        );

        // Create extrude and mesh
        let extrude = Extrude::new(profile, 1.0, 0.05, Material::Uniform("default".to_string()));
        let mesh = create_mesh_from_extrude(extrude);

        assert_eq!(mesh.geometry_count(), 1);
        let geo = mesh.geometries.get(0).unwrap();

        // Should have substantial geometry for outline + hole with chamfers
        assert!(
            geo.vertex_count() > 20,
            "Should have many vertices for chamfered triangle with hole"
        );
        assert!(
            geo.triangle_count() > 10,
            "Should have many triangles for chamfered triangle with hole"
        );
    }
}
