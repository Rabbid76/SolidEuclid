use std::collections::HashMap;

use glam::{DVec2, DVec3, DVec4};

use crate::geometry_error::GeometryError;
use crate::plane::Plane;
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
/// This struct uses double-precision floats (`f64`) via [`Vertex`] for vertex data,
/// providing high precision for geometric calculations, CSG (Constructive Solid Geometry)
/// operations, and mesh construction. It is designed for accurate geometric manipulation
/// before conversion to GPU-ready formats.
///
/// # GPU Rendering
///
/// To prepare geometry for GPU-based renderers such as three.js, WebGL, or WebGPU,
/// convert this mesh to a [`TriangularMesh`](crate::triangular_mesh::TriangularMesh)
/// using [`TriangularMesh::from_polygon_mesh`](crate::triangular_mesh::TriangularMesh::from_polygon_mesh).
/// This conversion tessellates polygons into triangles and converts vertex data from
/// `f64` to `f32`, which is the standard precision for GPU rendering pipelines.
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

    /// Adds a new polygon to the mesh from a vector of vertices.
    ///
    /// This function adds all provided vertices to the mesh and creates a polygon
    /// referencing them. The polygon's normal is calculated using the cross product
    /// of vectors formed by the first three vertices.
    ///
    /// # Arguments
    ///
    /// * `geometry_id` - The geometry ID to associate with the polygon (for material mapping)
    /// * `vertices` - A vector of vertices that define the polygon
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the polygon was successfully added.
    ///
    /// # Errors
    ///
    /// Returns `Err(GeometryError::InsufficientVertices)` if fewer than 3 vertices
    /// were provided (a polygon requires at least 3 vertices).
    ///
    /// # Note
    ///
    /// The polygon is assumed to be convex. Non-convex polygons may produce incorrect
    /// results during triangulation or other geometric operations.
    pub fn add_convex_polygon_from_vertices(
        &mut self,
        geometry_id: i32,
        vertices: Vec<Vertex>,
    ) -> Result<(), GeometryError> {
        if vertices.len() < 3 {
            return Err(GeometryError::InsufficientVertices {
                required: 3,
                provided: vertices.len(),
            });
        }

        // Calculate normal using cross product of first 3 vertices
        let p0 = vertices[0].position;
        let p1 = vertices[1].position;
        let p2 = vertices[2].position;

        let v1 = p1 - p0;
        let v2 = p2 - p0;
        let normal = v1.cross(v2).normalize();

        // Add vertices and collect their indices
        let start_index = self.vertices.len() as Index;
        let indices: Vec<Index> = (0..vertices.len() as Index)
            .map(|i| start_index + i)
            .collect();

        // Add all vertices to the mesh
        for vertex in vertices {
            self.vertices.push(vertex);
        }

        // Create and add the polygon
        let polygon = Polygon::new(indices, normal, geometry_id);
        self.polygons.push(polygon);

        Ok(())
    }

    /// Adds a new polygon to the mesh from 2D points on a plane.
    ///
    /// This function creates a polygon from 2D points defined on a plane in 3D space.
    /// The plane is specified by a `Plane` struct containing origin, normal, and x-axis.
    ///
    /// # Arguments
    ///
    /// * `geometry_id` - The geometry ID to associate with the polygon (for material mapping)
    /// * `plane` - The plane on which the polygon lies
    /// * `points` - A vector of 2D points defining the polygon on the plane
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the polygon was successfully added.
    ///
    /// # Errors
    ///
    /// Returns `Err(GeometryError::InsufficientVertices)` if fewer than 3 points
    /// were provided.
    ///
    /// # Coordinate System Construction
    ///
    /// The coordinate system is constructed as follows:
    /// - Z-axis: normalized `normal`
    /// - Y-axis: normalized cross product of Z-axis and `x_axis`
    /// - X-axis: normalized cross product of Y-axis and Z-axis (recalculated for orthogonality)
    ///
    /// The UV coordinates of each vertex are set equal to the corresponding 2D point.
    ///
    /// # Note
    ///
    /// The polygon is assumed to be convex. Non-convex polygons may produce incorrect
    /// results during triangulation or other geometric operations.
    pub fn add_convex_polygon_from_points_on_plane(
        &mut self,
        geometry_id: i32,
        plane: &Plane,
        points: Vec<DVec2>,
    ) -> Result<(), GeometryError> {
        if points.len() < 3 {
            return Err(GeometryError::InsufficientVertices {
                required: 3,
                provided: points.len(),
            });
        }

        // Get the transformation matrix from the plane
        let transform = plane.to_matrix();
        let z_axis = plane.normalized_normal();

        // Transform 2D points to 3D vertices
        let start_index = self.vertices.len() as Index;
        let mut indices: Vec<Index> = Vec::with_capacity(points.len());

        for (i, point) in points.iter().enumerate() {
            // Transform the 2D point (x, y, 0) to 3D
            let local_pos = DVec4::new(point.x, point.y, 0.0, 1.0);
            let world_pos = transform * local_pos;
            let position = DVec3::new(world_pos.x, world_pos.y, world_pos.z);

            // UV is equal to the 2D point
            let uv = *point;

            // The vertex normal is the plane's normalized normal
            let vertex = Vertex::new(position, z_axis, uv);
            self.vertices.push(vertex);
            indices.push(start_index + i as Index);
        }

        // Create and add the polygon
        let polygon = Polygon::new(indices, z_axis, geometry_id);
        self.polygons.push(polygon);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_error::GeometryError;
    use crate::plane::Plane;

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

        assert_eq!(
            mesh.get_polygon_material(&polygon1),
            Some(&"concrete".to_string())
        );
        assert_eq!(
            mesh.get_polygon_material(&polygon2),
            Some(&"brick".to_string())
        );
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

    // add_convex_polygon_from_vertices tests

    #[test]
    fn test_add_convex_polygon_from_vertices_triangle() {
        let mut mesh = PolygonMesh::new();

        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(0.5, 1.0, 0.0),
        ];

        let result = mesh.add_convex_polygon_from_vertices(1, vertices);

        assert!(result.is_ok());
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.polygon_count(), 1);
        assert_eq!(mesh.polygons[0].indices, vec![0, 1, 2]);
        assert_eq!(mesh.polygons[0].geometry_id, 1);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_quad() {
        let mut mesh = PolygonMesh::new();

        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(1.0, 1.0, 0.0),
            create_test_vertex(0.0, 1.0, 0.0),
        ];

        let result = mesh.add_convex_polygon_from_vertices(2, vertices);

        assert!(result.is_ok());
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.polygon_count(), 1);
        assert_eq!(mesh.polygons[0].indices, vec![0, 1, 2, 3]);
        assert_eq!(mesh.polygons[0].geometry_id, 2);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_insufficient_vertices() {
        let mut mesh = PolygonMesh::new();

        // Test with 0 vertices
        let result = mesh.add_convex_polygon_from_vertices(1, vec![]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 0
            }
        );
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.polygon_count(), 0);

        // Test with 1 vertex
        let result =
            mesh.add_convex_polygon_from_vertices(1, vec![create_test_vertex(0.0, 0.0, 0.0)]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 1
            }
        );
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.polygon_count(), 0);

        // Test with 2 vertices
        let result = mesh.add_convex_polygon_from_vertices(
            1,
            vec![
                create_test_vertex(0.0, 0.0, 0.0),
                create_test_vertex(1.0, 0.0, 0.0),
            ],
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 2
            }
        );
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.polygon_count(), 0);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_normal_calculation_xy_plane() {
        let mut mesh = PolygonMesh::new();

        // Triangle in XY plane (Z=0), counter-clockwise when viewed from +Z
        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(0.0, 1.0, 0.0),
        ];

        mesh.add_convex_polygon_from_vertices(1, vertices).unwrap();

        // Normal should point in +Z direction
        let normal = mesh.polygons[0].normal;
        assert!((normal.x).abs() < 1e-10);
        assert!((normal.y).abs() < 1e-10);
        assert!((normal.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_normal_calculation_xz_plane() {
        let mut mesh = PolygonMesh::new();

        // Triangle in XZ plane (Y=0)
        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(0.0, 0.0, 1.0),
            create_test_vertex(1.0, 0.0, 0.0),
        ];

        mesh.add_convex_polygon_from_vertices(1, vertices).unwrap();

        // Normal should point in +Y direction
        let normal = mesh.polygons[0].normal;
        assert!((normal.x).abs() < 1e-10);
        assert!((normal.y - 1.0).abs() < 1e-10);
        assert!((normal.z).abs() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_normal_calculation_yz_plane() {
        let mut mesh = PolygonMesh::new();

        // Triangle in YZ plane (X=0)
        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(0.0, 1.0, 0.0),
            create_test_vertex(0.0, 0.0, 1.0),
        ];

        mesh.add_convex_polygon_from_vertices(1, vertices).unwrap();

        // Normal should point in +X direction
        let normal = mesh.polygons[0].normal;
        assert!((normal.x - 1.0).abs() < 1e-10);
        assert!((normal.y).abs() < 1e-10);
        assert!((normal.z).abs() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_multiple_polygons() {
        let mut mesh = PolygonMesh::new();

        // Add first triangle
        let vertices1 = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(0.5, 1.0, 0.0),
        ];
        mesh.add_convex_polygon_from_vertices(1, vertices1).unwrap();

        // Add second triangle
        let vertices2 = vec![
            create_test_vertex(2.0, 0.0, 0.0),
            create_test_vertex(3.0, 0.0, 0.0),
            create_test_vertex(2.5, 1.0, 0.0),
        ];
        mesh.add_convex_polygon_from_vertices(2, vertices2).unwrap();

        assert_eq!(mesh.vertex_count(), 6);
        assert_eq!(mesh.polygon_count(), 2);

        // First polygon should have indices 0, 1, 2
        assert_eq!(mesh.polygons[0].indices, vec![0, 1, 2]);
        assert_eq!(mesh.polygons[0].geometry_id, 1);

        // Second polygon should have indices 3, 4, 5
        assert_eq!(mesh.polygons[1].indices, vec![3, 4, 5]);
        assert_eq!(mesh.polygons[1].geometry_id, 2);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_preserves_vertex_data() {
        let mut mesh = PolygonMesh::new();

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

        let vertices = vec![v0, v1, v2];
        mesh.add_convex_polygon_from_vertices(1, vertices).unwrap();

        // Verify vertex data is preserved
        assert_eq!(mesh.vertices[0].position, v0.position);
        assert_eq!(mesh.vertices[0].normal, v0.normal);
        assert_eq!(mesh.vertices[0].uv, v0.uv);

        assert_eq!(mesh.vertices[1].position, v1.position);
        assert_eq!(mesh.vertices[1].normal, v1.normal);
        assert_eq!(mesh.vertices[1].uv, v1.uv);

        assert_eq!(mesh.vertices[2].position, v2.position);
        assert_eq!(mesh.vertices[2].normal, v2.normal);
        assert_eq!(mesh.vertices[2].uv, v2.uv);
    }

    #[test]
    fn test_add_convex_polygon_from_vertices_pentagon() {
        let mut mesh = PolygonMesh::new();

        let vertices = vec![
            create_test_vertex(0.0, 0.0, 0.0),
            create_test_vertex(1.0, 0.0, 0.0),
            create_test_vertex(1.5, 0.5, 0.0),
            create_test_vertex(0.5, 1.0, 0.0),
            create_test_vertex(-0.5, 0.5, 0.0),
        ];

        let result = mesh.add_convex_polygon_from_vertices(1, vertices);

        assert!(result.is_ok());
        assert_eq!(mesh.vertex_count(), 5);
        assert_eq!(mesh.polygon_count(), 1);
        assert_eq!(mesh.polygons[0].indices, vec![0, 1, 2, 3, 4]);
    }

    // add_convex_polygon_from_points_on_plane tests

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_xy_plane_at_origin() {
        let mut mesh = PolygonMesh::new();

        let plane = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::X);
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 1.0),
        ];

        let result = mesh.add_convex_polygon_from_points_on_plane(1, &plane, points.clone());

        assert!(result.is_ok());
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.polygon_count(), 1);

        // Check positions - should be in XY plane at Z=0
        assert!((mesh.vertices[0].position - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        assert!((mesh.vertices[1].position - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-10);
        assert!((mesh.vertices[2].position - DVec3::new(0.5, 1.0, 0.0)).length() < 1e-10);

        // Check UVs - should equal the 2D points
        assert_eq!(mesh.vertices[0].uv, points[0]);
        assert_eq!(mesh.vertices[1].uv, points[1]);
        assert_eq!(mesh.vertices[2].uv, points[2]);

        // Check normals - should be Z
        assert!((mesh.vertices[0].normal - DVec3::Z).length() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_translated() {
        let mut mesh = PolygonMesh::new();

        let plane = Plane::new(DVec3::new(10.0, 20.0, 30.0), DVec3::Z, DVec3::X);
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];

        mesh.add_convex_polygon_from_points_on_plane(1, &plane, points)
            .unwrap();

        // Positions should be translated by origin
        assert!((mesh.vertices[0].position - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-10);
        assert!((mesh.vertices[1].position - DVec3::new(11.0, 20.0, 30.0)).length() < 1e-10);
        assert!((mesh.vertices[2].position - DVec3::new(10.0, 21.0, 30.0)).length() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_xz_plane() {
        let mut mesh = PolygonMesh::new();

        // Plane in XZ (normal pointing +Y)
        let plane = Plane::new(DVec3::ZERO, DVec3::Y, DVec3::X);
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];

        mesh.add_convex_polygon_from_points_on_plane(1, &plane, points)
            .unwrap();

        // Y-axis of plane = cross(Z, X) = cross(Y, X) = -Z, so local Y maps to -Z
        // X-axis of plane = cross(Y_plane, Z_plane) = cross(-Z, Y) = X
        // So (x, y, 0) -> (x, 0, -y) in world
        assert!((mesh.vertices[0].position - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        assert!((mesh.vertices[1].position - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-10);
        assert!((mesh.vertices[2].position - DVec3::new(0.0, 0.0, -1.0)).length() < 1e-10);

        // Normal should be +Y
        assert!((mesh.polygons[0].normal - DVec3::Y).length() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_yz_plane() {
        let mut mesh = PolygonMesh::new();

        // Plane in YZ (normal pointing +X)
        let plane = Plane::new(DVec3::ZERO, DVec3::X, DVec3::Y);
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];

        mesh.add_convex_polygon_from_points_on_plane(1, &plane, points)
            .unwrap();

        // Z-axis = X (normal)
        // Y-axis = cross(X, Y) = Z
        // X-axis = cross(Z, X) = Y
        // So local (x, y, 0) maps to (0, x, y) in world
        assert!((mesh.vertices[0].position - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);
        assert!((mesh.vertices[1].position - DVec3::new(0.0, 1.0, 0.0)).length() < 1e-10);
        assert!((mesh.vertices[2].position - DVec3::new(0.0, 0.0, 1.0)).length() < 1e-10);

        // Normal should be +X
        assert!((mesh.polygons[0].normal - DVec3::X).length() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_insufficient_points() {
        let mut mesh = PolygonMesh::new();

        let plane = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::X);

        // Test with 0 points
        let result = mesh.add_convex_polygon_from_points_on_plane(1, &plane, vec![]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 0
            }
        );
        assert_eq!(mesh.vertex_count(), 0);

        // Test with 1 point
        let result = mesh.add_convex_polygon_from_points_on_plane(1, &plane, vec![DVec2::ZERO]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 1
            }
        );

        // Test with 2 points
        let result =
            mesh.add_convex_polygon_from_points_on_plane(1, &plane, vec![DVec2::ZERO, DVec2::X]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 2
            }
        );
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_quad() {
        let mut mesh = PolygonMesh::new();

        let plane = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::X);
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];

        let result = mesh.add_convex_polygon_from_points_on_plane(1, &plane, points);

        assert!(result.is_ok());
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.polygon_count(), 1);
        assert_eq!(mesh.polygons[0].indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_uv_equals_points() {
        let mut mesh = PolygonMesh::new();

        let plane = Plane::new(
            DVec3::new(5.0, 5.0, 5.0),
            DVec3::new(1.0, 1.0, 1.0), // Arbitrary normal
            DVec3::new(1.0, -1.0, 0.0),
        );
        let points = vec![
            DVec2::new(0.25, 0.75),
            DVec2::new(0.5, 0.1),
            DVec2::new(0.9, 0.8),
        ];

        mesh.add_convex_polygon_from_points_on_plane(1, &plane, points.clone())
            .unwrap();

        // UVs should exactly match the input points
        assert_eq!(mesh.vertices[0].uv, points[0]);
        assert_eq!(mesh.vertices[1].uv, points[1]);
        assert_eq!(mesh.vertices[2].uv, points[2]);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_vertex_normals() {
        let mut mesh = PolygonMesh::new();

        let normal = DVec3::new(1.0, 2.0, 3.0); // Non-unit normal
        let plane = Plane::new(DVec3::ZERO, normal, DVec3::X);
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 1.0),
        ];

        mesh.add_convex_polygon_from_points_on_plane(1, &plane, points)
            .unwrap();

        let expected_normal = normal.normalize();

        // All vertex normals should be the normalized plane normal
        for vertex in &mesh.vertices {
            assert!((vertex.normal - expected_normal).length() < 1e-10);
        }

        // Polygon normal should also be normalized
        assert!((mesh.polygons[0].normal - expected_normal).length() < 1e-10);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_multiple_polygons() {
        let mut mesh = PolygonMesh::new();

        // First polygon on XY plane
        let plane1 = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::X);
        let points1 = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 1.0),
        ];
        mesh.add_convex_polygon_from_points_on_plane(1, &plane1, points1)
            .unwrap();

        // Second polygon on a different plane
        let plane2 = Plane::new(DVec3::new(5.0, 0.0, 0.0), DVec3::Z, DVec3::X);
        let points2 = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(2.0, 0.0),
            DVec2::new(1.0, 2.0),
        ];
        mesh.add_convex_polygon_from_points_on_plane(2, &plane2, points2)
            .unwrap();

        assert_eq!(mesh.vertex_count(), 6);
        assert_eq!(mesh.polygon_count(), 2);
        assert_eq!(mesh.polygons[0].indices, vec![0, 1, 2]);
        assert_eq!(mesh.polygons[1].indices, vec![3, 4, 5]);
        assert_eq!(mesh.polygons[0].geometry_id, 1);
        assert_eq!(mesh.polygons[1].geometry_id, 2);
    }

    #[test]
    fn test_add_convex_polygon_from_points_on_plane_orthogonalization() {
        let mut mesh = PolygonMesh::new();

        // Use a non-orthogonal x_axis (45 degrees from ideal)
        let plane = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::new(1.0, 1.0, 0.0));
        let points = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];

        mesh.add_convex_polygon_from_points_on_plane(1, &plane, points)
            .unwrap();

        // The recalculated x-axis should be normalized and orthogonal to z
        // z = (0, 0, 1)
        // y = cross(z, x_axis) = cross((0,0,1), (1,1,0)) = (-1, 1, 0) normalized = (-0.707, 0.707, 0)
        // x = cross(y, z) = cross((-0.707, 0.707, 0), (0, 0, 1)) = (0.707, 0.707, 0)

        let sqrt2_inv = 1.0 / 2.0_f64.sqrt();

        // Point (1, 0) should map to (sqrt2_inv, sqrt2_inv, 0)
        assert!((mesh.vertices[1].position.x - sqrt2_inv).abs() < 1e-10);
        assert!((mesh.vertices[1].position.y - sqrt2_inv).abs() < 1e-10);
        assert!(mesh.vertices[1].position.z.abs() < 1e-10);

        // Point (0, 1) should map to (-sqrt2_inv, sqrt2_inv, 0)
        assert!((mesh.vertices[2].position.x - (-sqrt2_inv)).abs() < 1e-10);
        assert!((mesh.vertices[2].position.y - sqrt2_inv).abs() < 1e-10);
        assert!(mesh.vertices[2].position.z.abs() < 1e-10);
    }
}
