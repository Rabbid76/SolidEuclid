use std::collections::HashMap;

use glam::DVec3;

use crate::vertex::{Index, Vertex};

/// Represents a polygon defined by vertex indices.
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    /// The indices of the vertices that define the polygon.
    pub indices: Vec<Index>,
    /// The normal vector of the polygon.
    pub normal: DVec3,
    /// The geometry ID that associates this polygon to a material in the material map.
    pub geometry_id: i32,
}

impl Polygon {
    /// Creates a new `Polygon` with the given indices, normal, and geometry ID.
    pub fn new(indices: Vec<Index>, normal: DVec3, geometry_id: i32) -> Self {
        Self {
            indices,
            normal,
            geometry_id,
        }
    }
}

impl Default for Polygon {
    fn default() -> Self {
        Self {
            indices: Vec::new(),
            normal: DVec3::Z,
            geometry_id: 0,
        }
    }
}

/// Represents a polygon mesh with vertices, polygons, and material mappings.
///
/// **Note:** All polygons in this mesh are assumed to be convex. Non-convex polygons
/// may produce incorrect results during triangulation or other geometric operations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolygonMesh {
    /// The vertices of the mesh.
    pub vertices: Vec<Vertex>,
    /// The polygons that make up the mesh.
    pub polygons: Vec<Polygon>,
    /// A map from geometry ID to material ID.
    pub material_map: HashMap<i32, String>,
}

impl PolygonMesh {
    /// Creates a new empty `PolygonMesh`.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            polygons: Vec::new(),
            material_map: HashMap::new(),
        }
    }

    /// Creates a new `PolygonMesh` with the given vertices, polygons, and material map.
    pub fn with_data(
        vertices: Vec<Vertex>,
        polygons: Vec<Polygon>,
        material_map: HashMap<i32, String>,
    ) -> Self {
        Self {
            vertices,
            polygons,
            material_map,
        }
    }

    /// Adds a vertex to the mesh and returns its index.
    pub fn add_vertex(&mut self, vertex: Vertex) -> Index {
        let index = self.vertices.len() as Index;
        self.vertices.push(vertex);
        index
    }

    /// Adds a polygon to the mesh.
    pub fn add_polygon(&mut self, polygon: Polygon) {
        self.polygons.push(polygon);
    }

    /// Sets the material for a geometry ID.
    pub fn set_material(&mut self, geometry_id: i32, material_id: String) {
        self.material_map.insert(geometry_id, material_id);
    }

    /// Gets the material ID for a geometry ID.
    pub fn get_material(&self, geometry_id: i32) -> Option<&String> {
        self.material_map.get(&geometry_id)
    }

    /// Gets the material ID for a polygon.
    pub fn get_polygon_material(&self, polygon: &Polygon) -> Option<&String> {
        self.material_map.get(&polygon.geometry_id)
    }

    /// Returns the number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of polygons in the mesh.
    pub fn polygon_count(&self) -> usize {
        self.polygons.len()
    }

    /// Returns true if the mesh is empty (no vertices and no polygons).
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.polygons.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec2;

    fn create_test_vertex(x: f64, y: f64, z: f64) -> Vertex {
        Vertex::new(
            DVec3::new(x, y, z),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.0, 0.0),
        )
    }

    // Polygon tests

    #[test]
    fn test_polygon_new() {
        let indices = vec![0, 1, 2, 3];
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let geometry_id = 5;

        let polygon = Polygon::new(indices.clone(), normal, geometry_id);

        assert_eq!(polygon.indices, indices);
        assert_eq!(polygon.normal, normal);
        assert_eq!(polygon.geometry_id, geometry_id);
    }

    #[test]
    fn test_polygon_default() {
        let polygon = Polygon::default();

        assert!(polygon.indices.is_empty());
        assert_eq!(polygon.normal, DVec3::Z);
        assert_eq!(polygon.geometry_id, 0);
    }

    #[test]
    fn test_polygon_clone() {
        let polygon = Polygon::new(vec![0, 1, 2], DVec3::new(1.0, 0.0, 0.0), 1);

        let cloned = polygon.clone();

        assert_eq!(polygon, cloned);
    }

    #[test]
    fn test_polygon_equality() {
        let polygon1 = Polygon::new(vec![0, 1, 2], DVec3::Y, 1);
        let polygon2 = Polygon::new(vec![0, 1, 2], DVec3::Y, 1);
        let polygon3 = Polygon::new(vec![0, 1, 2, 3], DVec3::X, 2);

        assert_eq!(polygon1, polygon2);
        assert_ne!(polygon1, polygon3);
    }

    // PolygonMesh tests

    #[test]
    fn test_polygon_mesh_new() {
        let mesh = PolygonMesh::new();

        assert!(mesh.vertices.is_empty());
        assert!(mesh.polygons.is_empty());
        assert!(mesh.material_map.is_empty());
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_polygon_mesh_default() {
        let mesh = PolygonMesh::default();

        assert!(mesh.vertices.is_empty());
        assert!(mesh.polygons.is_empty());
        assert!(mesh.material_map.is_empty());
    }

    #[test]
    fn test_polygon_mesh_with_data() {
        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(1.0, 1.0, 0.0),
            create_test_vertex(0.0, 1.0, 0.0),
        ];
        let polygons = vec![Polygon::new(vec![0, 1, 2, 3], DVec3::Z, 1)];
        let mut material_map = HashMap::new();
        material_map.insert(1, "wood".to_string());

        let mesh = PolygonMesh::with_data(vertices.clone(), polygons.clone(), material_map.clone());

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.polygons.len(), 1);
        assert_eq!(mesh.material_map.len(), 1);
    }

    #[test]
    fn test_polygon_mesh_add_vertex() {
        let mut mesh = PolygonMesh::new();

        let idx0 = mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        let idx1 = mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        let idx2 = mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert_eq!(mesh.vertex_count(), 3);
    }

    #[test]
    fn test_polygon_mesh_add_polygon() {
        let mut mesh = PolygonMesh::new();

        mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
        mesh.add_polygon(Polygon::new(vec![2, 3, 0], DVec3::Z, 1));

        assert_eq!(mesh.polygon_count(), 2);
    }

    #[test]
    fn test_polygon_mesh_set_and_get_material() {
        let mut mesh = PolygonMesh::new();

        mesh.set_material(1, "metal".to_string());
        mesh.set_material(2, "glass".to_string());

        assert_eq!(mesh.get_material(1), Some(&"metal".to_string()));
        assert_eq!(mesh.get_material(2), Some(&"glass".to_string()));
        assert_eq!(mesh.get_material(3), None);
    }

    #[test]
    fn test_polygon_mesh_get_polygon_material() {
        let mut mesh = PolygonMesh::new();
        mesh.set_material(1, "concrete".to_string());
        mesh.set_material(2, "brick".to_string());

        let polygon1 = Polygon::new(vec![0, 1, 2], DVec3::Z, 1);
        let polygon2 = Polygon::new(vec![3, 4, 5], DVec3::Y, 2);
        let polygon3 = Polygon::new(vec![6, 7, 8], DVec3::X, 99);

        assert_eq!(mesh.get_polygon_material(&polygon1), Some(&"concrete".to_string()));
        assert_eq!(mesh.get_polygon_material(&polygon2), Some(&"brick".to_string()));
        assert_eq!(mesh.get_polygon_material(&polygon3), None);
    }

    #[test]
    fn test_polygon_mesh_counts() {
        let mut mesh = PolygonMesh::new();

        mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 0));

        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.polygon_count(), 1);
        assert!(!mesh.is_empty());
    }

    #[test]
    fn test_polygon_mesh_clone() {
        let mut mesh = PolygonMesh::new();
        mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh.add_polygon(Polygon::new(vec![0], DVec3::Z, 1));
        mesh.set_material(1, "stone".to_string());

        let cloned = mesh.clone();

        assert_eq!(mesh, cloned);
    }

    #[test]
    fn test_polygon_mesh_equality() {
        let mut mesh1 = PolygonMesh::new();
        let mut mesh2 = PolygonMesh::new();

        mesh1.add_vertex(create_test_vertex(1.0, 2.0, 3.0));
        mesh1.set_material(1, "mat".to_string());

        mesh2.add_vertex(create_test_vertex(1.0, 2.0, 3.0));
        mesh2.set_material(1, "mat".to_string());

        let mesh3 = PolygonMesh::new();

        assert_eq!(mesh1, mesh2);
        assert_ne!(mesh1, mesh3);
    }

    #[test]
    fn test_polygon_mesh_complex() {
        let mut mesh = PolygonMesh::new();

        // Create a simple quad
        mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(1.0, 1.0, 0.0));
        mesh.add_vertex(create_test_vertex(0.0, 1.0, 0.0));

        mesh.add_polygon(Polygon::new(vec![0, 1, 2, 3], DVec3::Z, 1));

        mesh.set_material(1, "floor".to_string());

        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.polygon_count(), 1);
        assert_eq!(mesh.get_material(1), Some(&"floor".to_string()));
        assert_eq!(
            mesh.get_polygon_material(&mesh.polygons[0]),
            Some(&"floor".to_string())
        );
    }
}
