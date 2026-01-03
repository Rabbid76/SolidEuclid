use crate::geometry_error::GeometryError;
use crate::polygon_mesh::PolygonMesh;
use crate::triangular_mesh::TriangularMesh;

/// A trait for geometry models that can be converted to polygon or triangular meshes.
///
/// This trait provides a common interface for generating mesh representations
/// from various geometry models (e.g., platonic solids, parametric surfaces, etc.).
pub trait GeometryModel {
    /// Converts the geometry model to a polygon mesh.
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the model,
    /// or a `GeometryError` if the conversion fails.
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError>;

    /// Converts the geometry model to a triangular mesh.
    ///
    /// # Returns
    /// A `Result` containing the `TriangularMesh` representation of the model,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Default Implementation
    /// The default implementation first converts the model to a `PolygonMesh`
    /// using `to_polygon_mesh()`, then tessellates it into triangles using
    /// `TriangularMesh::from_polygon_mesh()`.
    fn to_triangular_mesh(&self) -> Result<TriangularMesh, GeometryError> {
        let polygon_mesh = self.to_polygon_mesh()?;
        Ok(TriangularMesh::from_polygon_mesh(&polygon_mesh))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_mesh::Polygon;
    use crate::vertex::Vertex;
    use glam::{DVec2, DVec3};

    /// A simple test geometry model representing a single triangle.
    struct TestTriangleModel {
        material_id: String,
    }

    impl TestTriangleModel {
        fn new(material_id: &str) -> Self {
            Self {
                material_id: material_id.to_string(),
            }
        }
    }

    impl GeometryModel for TestTriangleModel {
        fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
            let mut mesh = PolygonMesh::new();

            mesh.add_vertex(Vertex::new(
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(0.0, 0.0),
            ));
            mesh.add_vertex(Vertex::new(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(1.0, 0.0),
            ));
            mesh.add_vertex(Vertex::new(
                DVec3::new(0.5, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(0.5, 1.0),
            ));

            mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
            mesh.set_material(1, self.material_id.clone());

            Ok(mesh)
        }
    }

    /// A simple test geometry model representing a quad.
    struct TestQuadModel {
        material_id: String,
    }

    impl TestQuadModel {
        fn new(material_id: &str) -> Self {
            Self {
                material_id: material_id.to_string(),
            }
        }
    }

    impl GeometryModel for TestQuadModel {
        fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
            let mut mesh = PolygonMesh::new();

            mesh.add_vertex(Vertex::new(
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(0.0, 0.0),
            ));
            mesh.add_vertex(Vertex::new(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(1.0, 0.0),
            ));
            mesh.add_vertex(Vertex::new(
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(1.0, 1.0),
            ));
            mesh.add_vertex(Vertex::new(
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(0.0, 1.0),
            ));

            mesh.add_polygon(Polygon::new(vec![0, 1, 2, 3], DVec3::Z, 1));
            mesh.set_material(1, self.material_id.clone());

            Ok(mesh)
        }
    }

    /// A test geometry model that always fails.
    struct FailingModel;

    impl GeometryModel for FailingModel {
        fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
            Err(GeometryError::InsufficientVertices {
                required: 3,
                provided: 0,
            })
        }
    }

    #[test]
    fn test_to_polygon_mesh_triangle() {
        let model = TestTriangleModel::new("test_material");
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.polygons.len(), 1);
    }

    #[test]
    fn test_to_polygon_mesh_quad() {
        let model = TestQuadModel::new("quad_material");
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.polygons.len(), 1);
    }

    #[test]
    fn test_to_triangular_mesh_default_implementation_triangle() {
        let model = TestTriangleModel::new("tri_material");
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();
        assert_eq!(mesh.len(), 1);

        let geometry = mesh.get("tri_material").unwrap();
        assert_eq!(geometry.vertex_count(), 3);
        assert_eq!(geometry.indices.len(), 3); // 1 triangle = 3 indices
    }

    #[test]
    fn test_to_triangular_mesh_default_implementation_quad() {
        let model = TestQuadModel::new("quad_material");
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();
        assert_eq!(mesh.len(), 1);

        let geometry = mesh.get("quad_material").unwrap();
        assert_eq!(geometry.vertex_count(), 4);
        assert_eq!(geometry.indices.len(), 6); // 2 triangles = 6 indices (fan triangulation)
    }

    #[test]
    fn test_to_polygon_mesh_error_propagation() {
        let model = FailingModel;
        let result = model.to_polygon_mesh();

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            error,
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 0
            }
        );
    }

    #[test]
    fn test_to_triangular_mesh_error_propagation() {
        let model = FailingModel;
        let result = model.to_triangular_mesh();

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            error,
            GeometryError::InsufficientVertices {
                required: 3,
                provided: 0
            }
        );
    }

    #[test]
    fn test_trait_object_polygon_mesh() {
        let model: Box<dyn GeometryModel> = Box::new(TestTriangleModel::new("dynamic"));
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();
        assert_eq!(mesh.vertices.len(), 3);
    }

    #[test]
    fn test_trait_object_triangular_mesh() {
        let model: Box<dyn GeometryModel> = Box::new(TestQuadModel::new("dynamic_quad"));
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();
        let geometry = mesh.get("dynamic_quad").unwrap();
        assert_eq!(geometry.indices.len(), 6);
    }
}
