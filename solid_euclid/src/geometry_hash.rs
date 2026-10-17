use sha2::{Digest, Sha256};

use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::model::AnyModel;
use crate::polygon_mesh::PolygonMesh;
use crate::transform::Transform;
use crate::triangular_mesh::TriangularMesh;
use crate::vertex::Vertex;

/// A trait for computing a SHA256 hash of geometry objects.
///
/// The hash is computed from all relevant data in the object and encoded
/// as a hexadecimal string. For vertex data (position, normal, uv), values
/// are rounded to 3 decimal places to ensure consistent hashing across
/// slight floating-point variations.
pub trait GeometryHash {
    /// Computes a SHA256 hash of the geometry object.
    ///
    /// # Returns
    /// A `Result` containing the hexadecimal-encoded SHA256 hash string,
    /// or a `GeometryError` if the hash computation fails.
    fn geometry_hash(&self) -> Result<String, GeometryError>;
}

/// Rounds a floating-point value to 3 decimal places.
fn round_to_3_decimals(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Rounds a single-precision floating-point value to 3 decimal places.
fn round_f32_to_3_decimals(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

/// Hashes vertex data with 3 decimal place precision.
fn hash_vertex(hasher: &mut Sha256, vertex: &Vertex) {
    // Hash position
    hasher.update(round_to_3_decimals(vertex.position.x).to_le_bytes());
    hasher.update(round_to_3_decimals(vertex.position.y).to_le_bytes());
    hasher.update(round_to_3_decimals(vertex.position.z).to_le_bytes());

    // Hash normal
    hasher.update(round_to_3_decimals(vertex.normal.x).to_le_bytes());
    hasher.update(round_to_3_decimals(vertex.normal.y).to_le_bytes());
    hasher.update(round_to_3_decimals(vertex.normal.z).to_le_bytes());

    // Hash UV
    hasher.update(round_to_3_decimals(vertex.uv.x).to_le_bytes());
    hasher.update(round_to_3_decimals(vertex.uv.y).to_le_bytes());
}

impl GeometryHash for PolygonMesh {
    fn geometry_hash(&self) -> Result<String, GeometryError> {
        let mut hasher = Sha256::new();

        // Hash type identifier
        hasher.update(b"PolygonMesh");

        // Hash vertex count
        hasher.update((self.vertices.len() as u64).to_le_bytes());

        // Hash all vertices
        for vertex in &self.vertices {
            hash_vertex(&mut hasher, vertex);
        }

        // Hash polygon count
        hasher.update((self.polygons.len() as u64).to_le_bytes());

        // Hash all polygons
        for polygon in &self.polygons {
            // Hash index count
            hasher.update((polygon.indices.len() as u64).to_le_bytes());

            // Hash indices
            for &index in &polygon.indices {
                hasher.update(index.to_le_bytes());
            }

            // Hash polygon normal (with 3 decimal precision)
            hasher.update(round_to_3_decimals(polygon.normal.x).to_le_bytes());
            hasher.update(round_to_3_decimals(polygon.normal.y).to_le_bytes());
            hasher.update(round_to_3_decimals(polygon.normal.z).to_le_bytes());

            // Hash geometry ID
            hasher.update(polygon.geometry_id.to_le_bytes());
        }

        // Hash material map (sorted for deterministic ordering)
        let mut materials: Vec<_> = self.material_map.iter().collect();
        materials.sort_by_key(|(k, _)| *k);

        hasher.update((materials.len() as u64).to_le_bytes());
        for (geometry_id, material_id) in materials {
            hasher.update(geometry_id.to_le_bytes());
            hasher.update(material_id.as_bytes());
            hasher.update([0u8]); // Null terminator for string
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
}

impl GeometryHash for TriangularMesh {
    fn geometry_hash(&self) -> Result<String, GeometryError> {
        let mut hasher = Sha256::new();

        // Hash type identifier
        hasher.update(b"TriangularMesh");

        // Get sorted geometry keys for deterministic ordering
        let mut geometry_keys: Vec<_> = self.geometries.keys().collect();
        geometry_keys.sort();

        // Hash geometry count
        hasher.update((geometry_keys.len() as u64).to_le_bytes());

        // Hash all geometries in sorted order
        for material_id in geometry_keys {
            let geometry = &self.geometries[material_id];

            // Hash material ID
            hasher.update(material_id.as_bytes());
            hasher.update([0u8]); // Null terminator for string

            // Hash vertex count (number of vertices = vertices.len() / 3)
            let vertex_count = geometry.vertices.len() / 3;
            hasher.update((vertex_count as u64).to_le_bytes());

            // Hash all vertex positions (with 3 decimal precision)
            for &value in &geometry.vertices {
                hasher.update(round_f32_to_3_decimals(value).to_le_bytes());
            }

            // Hash all normals (with 3 decimal precision)
            for &value in &geometry.normals {
                hasher.update(round_f32_to_3_decimals(value).to_le_bytes());
            }

            // Hash all UVs (with 3 decimal precision)
            for &value in &geometry.uvs {
                hasher.update(round_f32_to_3_decimals(value).to_le_bytes());
            }

            // Hash index count
            hasher.update((geometry.indices.len() as u64).to_le_bytes());

            // Hash all indices
            for &index in &geometry.indices {
                hasher.update(index.to_le_bytes());
            }
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
}

impl<T> GeometryHash for Transform<T>
where
    T: GeometryModel,
{
    fn geometry_hash(&self) -> Result<String, GeometryError> {
        self.to_polygon_mesh()?.geometry_hash()
    }
}

impl GeometryHash for AnyModel {
    fn geometry_hash(&self) -> Result<String, GeometryError> {
        self.to_polygon_mesh()?.geometry_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_mesh::Polygon;
    use crate::triangular_mesh::TriangularGeometry;
    use glam::{DVec2, DVec3};

    fn create_test_vertex(x: f64, y: f64, z: f64) -> Vertex {
        Vertex::new(
            DVec3::new(x, y, z),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.0, 0.0),
        )
    }

    // round_to_3_decimals tests

    #[test]
    fn test_round_to_3_decimals() {
        assert_eq!(round_to_3_decimals(1.2345), 1.235);
        assert_eq!(round_to_3_decimals(1.2344), 1.234);
        assert_eq!(round_to_3_decimals(0.0001), 0.0);
        assert_eq!(round_to_3_decimals(-1.5555), -1.556);
        assert_eq!(round_to_3_decimals(123.456789), 123.457);
    }

    // PolygonMesh hash tests

    #[test]
    fn test_polygon_mesh_hash_empty() {
        let mesh = PolygonMesh::new();
        let hash = mesh.geometry_hash();

        assert!(hash.is_ok());
        let hash_str = hash.unwrap();
        assert_eq!(hash_str.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_polygon_mesh_hash_deterministic() {
        let mut mesh = PolygonMesh::new();
        mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));

        let hash1 = mesh.geometry_hash().unwrap();
        let hash2 = mesh.geometry_hash().unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_polygon_mesh_hash_different_for_different_data() {
        let mut mesh1 = PolygonMesh::new();
        mesh1.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh1.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        mesh1.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        mesh1.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));

        let mut mesh2 = PolygonMesh::new();
        mesh2.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh2.add_vertex(create_test_vertex(2.0, 0.0, 0.0)); // Different position
        mesh2.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        mesh2.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));

        let hash1 = mesh1.geometry_hash().unwrap();
        let hash2 = mesh2.geometry_hash().unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_polygon_mesh_hash_with_materials() {
        let mut mesh = PolygonMesh::new();
        mesh.add_vertex(create_test_vertex(0.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(1.0, 0.0, 0.0));
        mesh.add_vertex(create_test_vertex(0.5, 1.0, 0.0));
        mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
        mesh.set_material(1, "wood".to_string());

        let hash = mesh.geometry_hash();
        assert!(hash.is_ok());
    }

    #[test]
    fn test_polygon_mesh_hash_precision_rounding() {
        // Two meshes with slightly different values that round to the same
        let mut mesh1 = PolygonMesh::new();
        mesh1.add_vertex(Vertex::new(
            DVec3::new(1.0001, 0.0, 0.0),
            DVec3::Y,
            DVec2::ZERO,
        ));

        let mut mesh2 = PolygonMesh::new();
        mesh2.add_vertex(Vertex::new(
            DVec3::new(1.0004, 0.0, 0.0), // Rounds to same value (1.0)
            DVec3::Y,
            DVec2::ZERO,
        ));

        let hash1 = mesh1.geometry_hash().unwrap();
        let hash2 = mesh2.geometry_hash().unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_polygon_mesh_hash_precision_different() {
        // Two meshes with values that round to different values
        let mut mesh1 = PolygonMesh::new();
        mesh1.add_vertex(Vertex::new(
            DVec3::new(1.0001, 0.0, 0.0),
            DVec3::Y,
            DVec2::ZERO,
        ));

        let mut mesh2 = PolygonMesh::new();
        mesh2.add_vertex(Vertex::new(
            DVec3::new(1.0009, 0.0, 0.0), // Rounds to different value (1.001)
            DVec3::Y,
            DVec2::ZERO,
        ));

        let hash1 = mesh1.geometry_hash().unwrap();
        let hash2 = mesh2.geometry_hash().unwrap();

        assert_ne!(hash1, hash2);
    }

    // TriangularMesh hash tests

    #[test]
    fn test_triangular_mesh_hash_empty() {
        let mesh = TriangularMesh::new();
        let hash = mesh.geometry_hash();

        assert!(hash.is_ok());
        let hash_str = hash.unwrap();
        assert_eq!(hash_str.len(), 64);
    }

    #[test]
    fn test_triangular_mesh_hash_deterministic() {
        let mut mesh = TriangularMesh::new();
        let geometry = TriangularGeometry::with_data(
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0], // 3 vertices
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0], // 3 normals
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],                // 3 uvs
            vec![0, 1, 2],
        );
        mesh.insert("material".to_string(), geometry);

        let hash1 = mesh.geometry_hash().unwrap();
        let hash2 = mesh.geometry_hash().unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_triangular_mesh_hash_different_for_different_data() {
        let mut mesh1 = TriangularMesh::new();
        let geometry1 = TriangularGeometry::with_data(
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0, 1, 2],
        );
        mesh1.insert("material".to_string(), geometry1);

        let mut mesh2 = TriangularMesh::new();
        let geometry2 = TriangularGeometry::with_data(
            vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.5, 1.0, 0.0], // Different position (1.0 -> 2.0)
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0, 1, 2],
        );
        mesh2.insert("material".to_string(), geometry2);

        let hash1 = mesh1.geometry_hash().unwrap();
        let hash2 = mesh2.geometry_hash().unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_triangular_mesh_hash_multiple_geometries() {
        let mut mesh = TriangularMesh::new();

        let geometry1 = TriangularGeometry::with_data(
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0, 1, 2],
        );
        mesh.insert("material_a".to_string(), geometry1);

        let geometry2 = TriangularGeometry::with_data(
            vec![2.0, 0.0, 0.0, 3.0, 0.0, 0.0, 2.5, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0, 1, 2],
        );
        mesh.insert("material_b".to_string(), geometry2);

        let hash = mesh.geometry_hash();
        assert!(hash.is_ok());
    }

    #[test]
    fn test_triangular_mesh_hash_ordering_independent() {
        // Create two meshes with same geometries added in different order
        let geometry_a = TriangularGeometry::with_data(
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0, 1, 2],
        );

        let geometry_b = TriangularGeometry::with_data(
            vec![2.0, 0.0, 0.0, 3.0, 0.0, 0.0, 2.5, 1.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0, 1, 2],
        );

        let mut mesh1 = TriangularMesh::new();
        mesh1.insert("a".to_string(), geometry_a.clone());
        mesh1.insert("b".to_string(), geometry_b.clone());

        let mut mesh2 = TriangularMesh::new();
        mesh2.insert("b".to_string(), geometry_b);
        mesh2.insert("a".to_string(), geometry_a);

        let hash1 = mesh1.geometry_hash().unwrap();
        let hash2 = mesh2.geometry_hash().unwrap();

        // Should be equal because we sort keys before hashing
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_triangular_mesh_hash_precision_rounding() {
        let mut mesh1 = TriangularMesh::new();
        let geometry1 = TriangularGeometry::with_data(
            vec![1.0001, 0.0, 0.0], // position
            vec![0.0, 1.0, 0.0],    // normal
            vec![0.0, 0.0],         // uv
            vec![0],
        );
        mesh1.insert("mat".to_string(), geometry1);

        let mut mesh2 = TriangularMesh::new();
        let geometry2 = TriangularGeometry::with_data(
            vec![1.0004, 0.0, 0.0], // Rounds to same value
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0],
            vec![0],
        );
        mesh2.insert("mat".to_string(), geometry2);

        let hash1 = mesh1.geometry_hash().unwrap();
        let hash2 = mesh2.geometry_hash().unwrap();

        assert_eq!(hash1, hash2);
    }

    // Cross-type hash tests

    #[test]
    fn test_polygon_mesh_and_triangular_mesh_different_hashes() {
        // Even with equivalent data, the type identifier should make hashes different
        let polygon_mesh = PolygonMesh::new();
        let triangular_mesh = TriangularMesh::new();

        let hash1 = polygon_mesh.geometry_hash().unwrap();
        let hash2 = triangular_mesh.geometry_hash().unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_format_is_lowercase_hex() {
        let mesh = PolygonMesh::new();
        let hash = mesh.geometry_hash().unwrap();

        // Should be all lowercase hex characters
        assert!(
            hash.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
    }

    // Transform and AnyModel hash tests

    use crate::material::Material;
    use crate::model::AnyModel;
    use crate::platonic_solid::{Model as PlatonicSolidModel, PlatonicSolid};
    use crate::sphere::{Model as SphereModel, Sphere};
    use crate::transform::{Transform, Transformation};

    fn hexahedron() -> PlatonicSolidModel {
        PlatonicSolidModel::new(
            PlatonicSolid::Hexahedron,
            Material::Uniform("default".to_string()),
        )
    }

    #[test]
    fn test_transform_hash_matches_polygon_mesh_hash() {
        let transform = Transform::with_translation(DVec3::new(1.0, 2.0, 3.0), hexahedron());

        let hash = transform.geometry_hash().unwrap();
        let expected = transform
            .to_polygon_mesh()
            .unwrap()
            .geometry_hash()
            .unwrap();

        assert_eq!(hash, expected);
    }

    #[test]
    fn test_transform_hash_deterministic() {
        let transform = Transform::with_uniform_scale(2.0, hexahedron());

        let hash1 = transform.geometry_hash().unwrap();
        let hash2 = transform.geometry_hash().unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_transform_hash_different_for_different_transformations() {
        let identity = Transform::new(Transformation::default(), hexahedron());
        let translated = Transform::with_translation(DVec3::new(1.0, 0.0, 0.0), hexahedron());

        let hash1 = identity.geometry_hash().unwrap();
        let hash2 = translated.geometry_hash().unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_transform_hash_propagates_validation_error() {
        let transform = Transform::with_uniform_scale(f64::NAN, hexahedron());

        let result = transform.geometry_hash();
        assert!(result.is_err());
    }

    #[test]
    fn test_any_model_hash_matches_polygon_mesh_hash() {
        let model = AnyModel::PlatonicSolid(hexahedron());

        let hash = model.geometry_hash().unwrap();
        let expected = model.to_polygon_mesh().unwrap().geometry_hash().unwrap();

        assert_eq!(hash, expected);
    }

    #[test]
    fn test_any_model_hash_deterministic() {
        let model = AnyModel::Sphere(SphereModel::new(
            Sphere::Slices {
                slices: 8,
                segments: 16,
            },
            Material::Uniform("default".to_string()),
        ));

        let hash1 = model.geometry_hash().unwrap();
        let hash2 = model.geometry_hash().unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_any_model_hash_different_variants() {
        let platonic = AnyModel::PlatonicSolid(hexahedron());
        let sphere = AnyModel::Sphere(SphereModel::new(
            Sphere::Slices {
                slices: 8,
                segments: 16,
            },
            Material::Uniform("default".to_string()),
        ));

        let hash1 = platonic.geometry_hash().unwrap();
        let hash2 = sphere.geometry_hash().unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_any_model_hash_nested_transform() {
        let model = AnyModel::Transform(Box::new(Transform::with_translation(
            DVec3::new(5.0, 0.0, 0.0),
            AnyModel::PlatonicSolid(hexahedron()),
        )));

        let hash = model.geometry_hash();
        assert!(hash.is_ok());
        assert_eq!(hash.unwrap().len(), 64);
    }
}
