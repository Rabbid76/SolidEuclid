use std::collections::HashMap;

use crate::polygon_mesh::PolygonMesh;
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

    /// Creates a new `TriangularMesh` from a `PolygonMesh`.
    ///
    /// This function tessellates all convex polygons into triangles using fan triangulation.
    /// Each polygon with n vertices is converted into (n-2) triangles.
    ///
    /// The material mapping from the `PolygonMesh` is preserved, with each polygon's
    /// `geometry_id` used to look up its material in the material map.
    pub fn from_polygon_mesh(polygon_mesh: &PolygonMesh) -> Self {
        let mut mesh = TriangularMesh::new();

        // Group polygons by their material
        let mut material_polygons: HashMap<String, Vec<usize>> = HashMap::new();

        for (polygon_idx, polygon) in polygon_mesh.polygons.iter().enumerate() {
            let material_id = polygon_mesh
                .material_map
                .get(&polygon.geometry_id)
                .cloned()
                .unwrap_or_else(|| format!("material_{}", polygon.geometry_id));

            material_polygons
                .entry(material_id)
                .or_default()
                .push(polygon_idx);
        }

        // Process each material group
        for (material_id, polygon_indices) in material_polygons {
            let mut geometry = TriangularGeometry::new();
            let mut vertex_map: HashMap<Index, Index> = HashMap::new();

            for polygon_idx in polygon_indices {
                let polygon = &polygon_mesh.polygons[polygon_idx];

                // Map original vertex indices to new indices in this geometry
                let mut local_indices: Vec<Index> = Vec::new();
                for &orig_idx in &polygon.indices {
                    let new_idx = *vertex_map.entry(orig_idx).or_insert_with(|| {
                        let idx = geometry.vertices.len() as Index;
                        if let Some(vertex) = polygon_mesh.vertices.get(orig_idx as usize) {
                            geometry.vertices.push(*vertex);
                        }
                        idx
                    });
                    local_indices.push(new_idx);
                }

                // Fan triangulation: for a convex polygon with vertices [0, 1, 2, ..., n-1],
                // create triangles (0, 1, 2), (0, 2, 3), ..., (0, n-2, n-1)
                if local_indices.len() >= 3 {
                    let first = local_indices[0];
                    for i in 1..local_indices.len() - 1 {
                        geometry.indices.push(first);
                        geometry.indices.push(local_indices[i]);
                        geometry.indices.push(local_indices[i + 1]);
                    }
                }
            }

            if !geometry.vertices.is_empty() {
                mesh.insert(material_id, geometry);
            }
        }

        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_mesh::Polygon;
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

    // from_polygon_mesh tests

    #[test]
    fn test_from_polygon_mesh_empty() {
        let polygon_mesh = PolygonMesh::new();
        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        assert!(triangular_mesh.is_empty());
    }

    #[test]
    fn test_from_polygon_mesh_single_triangle() {
        let mut polygon_mesh = PolygonMesh::new();

        polygon_mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));

        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
        polygon_mesh.set_material(1, "wood".to_string());

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        assert_eq!(triangular_mesh.len(), 1);
        let geometry = triangular_mesh.get("wood").unwrap();
        assert_eq!(geometry.vertices.len(), 3);
        assert_eq!(geometry.indices.len(), 3); // 1 triangle = 3 indices
    }

    #[test]
    fn test_from_polygon_mesh_quad() {
        let mut polygon_mesh = PolygonMesh::new();

        // Create a quad
        polygon_mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 1.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(0.0, 1.0, 0.0));

        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2, 3], DVec3::Z, 1));
        polygon_mesh.set_material(1, "floor".to_string());

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        assert_eq!(triangular_mesh.len(), 1);
        let geometry = triangular_mesh.get("floor").unwrap();
        assert_eq!(geometry.vertices.len(), 4);
        assert_eq!(geometry.indices.len(), 6); // 2 triangles = 6 indices

        // Verify fan triangulation: (0,1,2) and (0,2,3)
        assert_eq!(geometry.indices[0], 0);
        assert_eq!(geometry.indices[1], 1);
        assert_eq!(geometry.indices[2], 2);
        assert_eq!(geometry.indices[3], 0);
        assert_eq!(geometry.indices[4], 2);
        assert_eq!(geometry.indices[5], 3);
    }

    #[test]
    fn test_from_polygon_mesh_pentagon() {
        let mut polygon_mesh = PolygonMesh::new();

        // Create a pentagon (5 vertices)
        polygon_mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.5, 0.5, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(-0.5, 0.5, 0.0));

        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2, 3, 4], DVec3::Z, 1));
        polygon_mesh.set_material(1, "pentagon".to_string());

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        let geometry = triangular_mesh.get("pentagon").unwrap();
        assert_eq!(geometry.vertices.len(), 5);
        assert_eq!(geometry.indices.len(), 9); // 3 triangles = 9 indices (n-2 triangles)
    }

    #[test]
    fn test_from_polygon_mesh_multiple_materials() {
        let mut polygon_mesh = PolygonMesh::new();

        // Vertices for two separate quads
        polygon_mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 1.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(0.0, 1.0, 0.0));

        polygon_mesh.add_vertex(create_test_vertex(2.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(3.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(3.0, 1.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(2.0, 1.0, 0.0));

        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2, 3], DVec3::Z, 1));
        polygon_mesh.add_polygon(Polygon::new(vec![4, 5, 6, 7], DVec3::Z, 2));

        polygon_mesh.set_material(1, "wood".to_string());
        polygon_mesh.set_material(2, "metal".to_string());

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        assert_eq!(triangular_mesh.len(), 2);

        let wood_geometry = triangular_mesh.get("wood").unwrap();
        assert_eq!(wood_geometry.vertices.len(), 4);
        assert_eq!(wood_geometry.indices.len(), 6);

        let metal_geometry = triangular_mesh.get("metal").unwrap();
        assert_eq!(metal_geometry.vertices.len(), 4);
        assert_eq!(metal_geometry.indices.len(), 6);
    }

    #[test]
    fn test_from_polygon_mesh_multiple_polygons_same_material() {
        let mut polygon_mesh = PolygonMesh::new();

        // Two triangles with the same material
        polygon_mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(2.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(3.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(2.5, 1.0, 0.0));

        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
        polygon_mesh.add_polygon(Polygon::new(vec![3, 4, 5], DVec3::Z, 1));

        polygon_mesh.set_material(1, "brick".to_string());

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        assert_eq!(triangular_mesh.len(), 1);

        let geometry = triangular_mesh.get("brick").unwrap();
        assert_eq!(geometry.vertices.len(), 6);
        assert_eq!(geometry.indices.len(), 6); // 2 triangles = 6 indices
    }

    #[test]
    fn test_from_polygon_mesh_default_material() {
        let mut polygon_mesh = PolygonMesh::new();

        polygon_mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        polygon_mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));

        // Polygon without a material mapping
        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 99));

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);

        assert_eq!(triangular_mesh.len(), 1);
        // Should create a default material name
        assert!(triangular_mesh.get("material_99").is_some());
    }

    #[test]
    fn test_from_polygon_mesh_preserves_vertex_data() {
        let mut polygon_mesh = PolygonMesh::new();

        let v0 = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 0.0, 1.0),
            DVec2::new(0.25, 0.75),
        );
        let v1 = Vertex::new(
            DVec3::new(4.0, 5.0, 6.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.5, 0.5),
        );
        let v2 = Vertex::new(
            DVec3::new(7.0, 8.0, 9.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec2::new(0.0, 1.0),
        );

        polygon_mesh.vertices.push(v0);
        polygon_mesh.vertices.push(v1);
        polygon_mesh.vertices.push(v2);

        polygon_mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
        polygon_mesh.set_material(1, "test".to_string());

        let triangular_mesh = TriangularMesh::from_polygon_mesh(&polygon_mesh);
        let geometry = triangular_mesh.get("test").unwrap();

        // Verify vertex data is preserved
        assert_eq!(geometry.vertices[0].position, v0.position);
        assert_eq!(geometry.vertices[0].normal, v0.normal);
        assert_eq!(geometry.vertices[0].uv, v0.uv);

        assert_eq!(geometry.vertices[1].position, v1.position);
        assert_eq!(geometry.vertices[2].position, v2.position);
    }
}
