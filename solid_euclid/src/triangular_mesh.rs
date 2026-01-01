use std::collections::HashMap;

use crate::vertex::{Index, Vertex};

/// Represents a triangular geometry with vertices and indices.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TriangularGeometry {
    /// The vertices of the geometry.
    pub vertices: Vec<Vertex>,
    /// The indices defining the triangles.
    pub indices: Vec<Index>,
}

impl TriangularGeometry {
    /// Creates a new empty `TriangularGeometry`.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Creates a new `TriangularGeometry` with the given vertices and indices.
    pub fn with_data(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self { vertices, indices }
    }
}

/// Represents a triangular mesh composed of multiple geometries identified by material IDs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TriangularMesh {
    /// A map from material ID to triangular geometry.
    pub geometries: HashMap<String, TriangularGeometry>,
}

impl TriangularMesh {
    /// Creates a new empty `TriangularMesh`.
    pub fn new() -> Self {
        Self {
            geometries: HashMap::new(),
        }
    }

    /// Creates a new `TriangularMesh` with the given geometries.
    pub fn with_geometries(geometries: HashMap<String, TriangularGeometry>) -> Self {
        Self { geometries }
    }

    /// Inserts a geometry with the given material ID.
    pub fn insert(&mut self, material_id: String, geometry: TriangularGeometry) {
        self.geometries.insert(material_id, geometry);
    }

    /// Gets a reference to the geometry for the given material ID.
    pub fn get(&self, material_id: &str) -> Option<&TriangularGeometry> {
        self.geometries.get(material_id)
    }

    /// Gets a mutable reference to the geometry for the given material ID.
    pub fn get_mut(&mut self, material_id: &str) -> Option<&mut TriangularGeometry> {
        self.geometries.get_mut(material_id)
    }

    /// Returns the number of geometries in the mesh.
    pub fn len(&self) -> usize {
        self.geometries.len()
    }

    /// Returns true if the mesh contains no geometries.
    pub fn is_empty(&self) -> bool {
        self.geometries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DVec2, DVec3};

    fn create_test_vertex(x: f64, y: f64, z: f64) -> Vertex {
        Vertex::new(
            DVec3::new(x, y, z),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.0, 0.0),
        )
    }

    fn create_test_triangle() -> (Vec<Vertex>, Vec<Index>) {
        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(0.5, 1.0, 0.0),
        ];
        let indices = vec![0, 1, 2];
        (vertices, indices)
    }

    // TriangularGeometry tests

    #[test]
    fn test_triangular_geometry_new() {
        let geometry = TriangularGeometry::new();

        assert!(geometry.vertices.is_empty());
        assert!(geometry.indices.is_empty());
    }

    #[test]
    fn test_triangular_geometry_default() {
        let geometry = TriangularGeometry::default();

        assert!(geometry.vertices.is_empty());
        assert!(geometry.indices.is_empty());
    }

    #[test]
    fn test_triangular_geometry_with_data() {
        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices.clone(), indices.clone());

        assert_eq!(geometry.vertices.len(), 3);
        assert_eq!(geometry.indices.len(), 3);
        assert_eq!(geometry.vertices, vertices);
        assert_eq!(geometry.indices, indices);
    }

    #[test]
    fn test_triangular_geometry_clone() {
        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices, indices);

        let cloned = geometry.clone();

        assert_eq!(geometry, cloned);
    }

    #[test]
    fn test_triangular_geometry_equality() {
        let (vertices1, indices1) = create_test_triangle();
        let (vertices2, indices2) = create_test_triangle();

        let geometry1 = TriangularGeometry::with_data(vertices1, indices1);
        let geometry2 = TriangularGeometry::with_data(vertices2, indices2);
        let geometry3 = TriangularGeometry::new();

        assert_eq!(geometry1, geometry2);
        assert_ne!(geometry1, geometry3);
    }

    // TriangularMesh tests

    #[test]
    fn test_triangular_mesh_new() {
        let mesh = TriangularMesh::new();

        assert!(mesh.geometries.is_empty());
        assert!(mesh.is_empty());
        assert_eq!(mesh.len(), 0);
    }

    #[test]
    fn test_triangular_mesh_default() {
        let mesh = TriangularMesh::default();

        assert!(mesh.geometries.is_empty());
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_triangular_mesh_with_geometries() {
        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices, indices);

        let mut geometries = HashMap::new();
        geometries.insert("material1".to_string(), geometry);

        let mesh = TriangularMesh::with_geometries(geometries);

        assert_eq!(mesh.len(), 1);
        assert!(!mesh.is_empty());
        assert!(mesh.get("material1").is_some());
    }

    #[test]
    fn test_triangular_mesh_insert() {
        let mut mesh = TriangularMesh::new();
        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices, indices);

        mesh.insert("wood".to_string(), geometry.clone());

        assert_eq!(mesh.len(), 1);
        assert!(mesh.get("wood").is_some());
        assert_eq!(mesh.get("wood").unwrap(), &geometry);
    }

    #[test]
    fn test_triangular_mesh_get() {
        let mut mesh = TriangularMesh::new();
        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices, indices);

        mesh.insert("metal".to_string(), geometry.clone());

        assert!(mesh.get("metal").is_some());
        assert!(mesh.get("plastic").is_none());
    }

    #[test]
    fn test_triangular_mesh_get_mut() {
        let mut mesh = TriangularMesh::new();
        let geometry = TriangularGeometry::new();

        mesh.insert("material".to_string(), geometry);

        if let Some(geo) = mesh.get_mut("material") {
            geo.vertices.push(create_test_vertex(1.0, 2.0, 3.0));
            geo.indices.push(0);
        }

        assert_eq!(mesh.get("material").unwrap().vertices.len(), 1);
        assert_eq!(mesh.get("material").unwrap().indices.len(), 1);
    }

    #[test]
    fn test_triangular_mesh_multiple_materials() {
        let mut mesh = TriangularMesh::new();

        let (vertices1, indices1) = create_test_triangle();
        let geometry1 = TriangularGeometry::with_data(vertices1, indices1);

        let (vertices2, indices2) = create_test_triangle();
        let geometry2 = TriangularGeometry::with_data(vertices2, indices2);

        mesh.insert("material_a".to_string(), geometry1);
        mesh.insert("material_b".to_string(), geometry2);

        assert_eq!(mesh.len(), 2);
        assert!(mesh.get("material_a").is_some());
        assert!(mesh.get("material_b").is_some());
    }

    #[test]
    fn test_triangular_mesh_clone() {
        let mut mesh = TriangularMesh::new();
        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices, indices);

        mesh.insert("material".to_string(), geometry);

        let cloned = mesh.clone();

        assert_eq!(mesh, cloned);
    }

    #[test]
    fn test_triangular_mesh_equality() {
        let mut mesh1 = TriangularMesh::new();
        let mut mesh2 = TriangularMesh::new();

        let (vertices, indices) = create_test_triangle();
        let geometry = TriangularGeometry::with_data(vertices, indices);

        mesh1.insert("material".to_string(), geometry.clone());
        mesh2.insert("material".to_string(), geometry);

        let mesh3 = TriangularMesh::new();

        assert_eq!(mesh1, mesh2);
        assert_ne!(mesh1, mesh3);
    }
}
