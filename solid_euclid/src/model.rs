//! The bridge-facing model enum: any geometry model, deserializable from JSON.
//!
//! [`AnyModel`] wraps every concrete geometry type in the kernel plus a `Transform` variant for
//! nested transformations. It exists because the bridges deserialize JSON into concrete types,
//! and generic wrappers like [`Transform<T>`](crate::transform::Transform) need a concrete `T` at
//! the deserialization site. `AnyModel` is that concrete type.
//!
//! The enum is designed for reuse: CSG operators (union, difference, intersection) will use
//! `AnyModel` as their operand type without changes to the enum.

use serde::{Deserialize, Serialize};

use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::platonic_solid::Model as PlatonicSolidModel;
use crate::polygon_mesh::PolygonMesh;
use crate::sphere::Model as SphereModel;
use crate::sweeps::{Capsule, ConicalFrustum, Cylinder, Extrude, Torus};
use crate::transform::Transform;

/// A serde-deserializable enum that can represent any geometry model in the kernel.
///
/// This enum is the bridge-facing deserialization target for generic wrappers such as
/// [`Transform<AnyModel>`](crate::transform::Transform). The bridges parse JSON into
/// `Transform<AnyModel>` and convert it through the standard `GeometryModel` pipeline.
///
/// The `Transform` variant enables nested transformations: a user can wrap a transformed model
/// in another transformation, and the JSON representation nests naturally.
///
/// # Serde representation
///
/// Uses serde's default externally-tagged representation, consistent with
/// [`Material::Uniform`](crate::material::Material):
///
/// ```json
/// { "PlatonicSolid": { "geometry": "Hexahedron", "material": { "Uniform": "default" } } }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnyModel {
    /// A Platonic solid (tetrahedron, hexahedron, octahedron, dodecahedron, icosahedron).
    PlatonicSolid(PlatonicSolidModel),
    /// A sphere (UV slices or cube-mapped).
    Sphere(SphereModel),
    /// A torus (donut shape).
    Torus(Torus),
    /// A capsule (cylinder with hemispherical caps).
    Capsule(Capsule),
    /// A cylinder with optional edge treatments.
    Cylinder(Cylinder),
    /// A conical frustum (truncated cone) with optional edge treatments.
    ConicalFrustum(ConicalFrustum),
    /// An extrusion of a 2D planar profile along the Y-axis.
    Extrude(Extrude),
    /// A transformed model — enables nested transformations.
    Transform(Box<Transform<AnyModel>>),
}

impl GeometryModel for AnyModel {
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        match self {
            AnyModel::PlatonicSolid(m) => m.to_polygon_mesh(),
            AnyModel::Sphere(m) => m.to_polygon_mesh(),
            AnyModel::Torus(m) => m.to_polygon_mesh(),
            AnyModel::Capsule(m) => m.to_polygon_mesh(),
            AnyModel::Cylinder(m) => m.to_polygon_mesh(),
            AnyModel::ConicalFrustum(m) => m.to_polygon_mesh(),
            AnyModel::Extrude(m) => m.to_polygon_mesh(),
            AnyModel::Transform(m) => m.to_polygon_mesh(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Material;
    use crate::platonic_solid::PlatonicSolid;
    use glam::DVec3;

    fn hexahedron_model() -> AnyModel {
        AnyModel::PlatonicSolid(PlatonicSolidModel::new(
            PlatonicSolid::Hexahedron,
            Material::Uniform("default".to_string()),
        ))
    }

    fn translated_hexahedron() -> Transform<AnyModel> {
        Transform::with_translation(DVec3::new(1.0, 2.0, 3.0), hexahedron_model())
    }

    // ==================== Delegation tests ====================

    #[test]
    fn test_platonic_solid_delegation() {
        let model = hexahedron_model();
        let mesh = model.to_polygon_mesh().unwrap();
        assert_eq!(mesh.vertex_count(), 24); // 6 faces * 4 vertices
        assert_eq!(mesh.polygon_count(), 6);
    }

    #[test]
    fn test_sphere_delegation() {
        let model = AnyModel::Sphere(SphereModel::new(
            crate::sphere::Sphere::Cube { segments: 2 },
            Material::Uniform("default".to_string()),
        ));
        let mesh = model.to_polygon_mesh().unwrap();
        assert!(mesh.vertex_count() > 0);
        assert!(mesh.polygon_count() > 0);
    }

    #[test]
    fn test_torus_delegation() {
        let model = AnyModel::Torus(Torus::new(
            2.0,
            0.5,
            12,
            8,
            Material::Uniform("default".to_string()),
        ));
        let mesh = model.to_polygon_mesh().unwrap();
        assert!(mesh.vertex_count() > 0);
    }

    #[test]
    fn test_transform_variant_delegation() {
        let inner = translated_hexahedron();
        let model = AnyModel::Transform(Box::new(inner));
        let mesh = model.to_polygon_mesh().unwrap();
        assert_eq!(mesh.vertex_count(), 24);
        assert_eq!(mesh.polygon_count(), 6);
    }

    #[test]
    fn test_to_triangular_mesh_through_enum() {
        let model = hexahedron_model();
        let tri_mesh = model.to_triangular_mesh().unwrap();
        assert_eq!(tri_mesh.len(), 1);
        let geometry = tri_mesh.get("default").unwrap();
        assert_eq!(geometry.indices.len(), 36); // 6 faces * 2 triangles * 3 indices
    }

    // ==================== Serialization tests ====================

    #[test]
    fn test_serialize_deserialize_platonic_solid() {
        let model = hexahedron_model();
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: AnyModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, deserialized);
    }

    #[test]
    fn test_serialize_deserialize_sphere() {
        let model = AnyModel::Sphere(SphereModel::new(
            crate::sphere::Sphere::Slices {
                slices: 4,
                segments: 8,
            },
            Material::Uniform("sphere_mat".to_string()),
        ));
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: AnyModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, deserialized);
    }

    #[test]
    fn test_serialize_deserialize_torus() {
        let model = AnyModel::Torus(Torus::new(
            2.0,
            0.5,
            12,
            8,
            Material::Uniform("torus_mat".to_string()),
        ));
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: AnyModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, deserialized);
    }

    #[test]
    fn test_serialize_deserialize_transform_variant() {
        let model = AnyModel::Transform(Box::new(translated_hexahedron()));
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: AnyModel = serde_json::from_str(&json).unwrap();
        assert_eq!(model, deserialized);
    }

    #[test]
    fn test_serialize_deserialize_nested_transform() {
        // Outer: scale by 2, Inner: translate by (1, 2, 3)
        let inner = Transform::with_translation(DVec3::new(1.0, 2.0, 3.0), hexahedron_model());
        let outer = Transform::with_uniform_scale(2.0, AnyModel::Transform(Box::new(inner)));
        let json = serde_json::to_string(&outer).unwrap();
        let deserialized: Transform<AnyModel> = serde_json::from_str(&json).unwrap();
        assert_eq!(outer, deserialized);
    }

    #[test]
    fn test_deserialize_unknown_variant_fails() {
        let json = r#"{"UnknownType": {}}"#;
        let result: Result<AnyModel, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_missing_fields_fails() {
        let json = r#"{"PlatonicSolid": {"geometry": "Hexahedron"}}"#;
        let result: Result<AnyModel, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ==================== Transform<AnyModel> integration tests ====================

    #[test]
    fn test_transform_any_model_translation() {
        let transform = translated_hexahedron();
        let mesh = transform.to_polygon_mesh().unwrap();

        // Compute bounding box and verify it is offset by the translation (1, 2, 3)
        let mut min = DVec3::splat(f64::MAX);
        let mut max = DVec3::splat(f64::MIN);
        for vertex in &mesh.vertices {
            min = min.min(vertex.position);
            max = max.max(vertex.position);
        }

        // Original hexahedron is normalized to unit sphere: vertices at ±1/√3 ≈ ±0.577
        let l = 1.0 / 3.0_f64.sqrt();
        let translation = DVec3::new(1.0, 2.0, 3.0);
        let eps = 1e-10;

        assert!((min.x - (-l + translation.x)).abs() < eps);
        assert!((max.x - (l + translation.x)).abs() < eps);
        assert!((min.y - (-l + translation.y)).abs() < eps);
        assert!((max.y - (l + translation.y)).abs() < eps);
        assert!((min.z - (-l + translation.z)).abs() < eps);
        assert!((max.z - (l + translation.z)).abs() < eps);
    }

    #[test]
    fn test_transform_any_model_serialize_deserialize() {
        let transform = translated_hexahedron();
        let json = serde_json::to_string(&transform).unwrap();
        let deserialized: Transform<AnyModel> = serde_json::from_str(&json).unwrap();
        assert_eq!(transform, deserialized);
    }

    #[test]
    fn test_transform_any_model_to_triangular_mesh() {
        let transform = translated_hexahedron();
        let tri_mesh = transform.to_triangular_mesh().unwrap();
        assert_eq!(tri_mesh.len(), 1);
        let geometry = tri_mesh.get("default").unwrap();
        assert_eq!(geometry.indices.len(), 36);
    }

    #[test]
    fn test_nested_transform_any_model() {
        // Translate then scale
        let inner = Transform::with_translation(DVec3::new(1.0, 0.0, 0.0), hexahedron_model());
        let outer = Transform::with_uniform_scale(2.0, AnyModel::Transform(Box::new(inner)));
        let mesh = outer.to_polygon_mesh().unwrap();

        // Original vertex at (0.5, 0.5, 0.5) → translated to (1.5, 0.5, 0.5) → scaled to (3.0, 1.0, 1.0)
        assert_eq!(mesh.vertex_count(), 24);
    }

    #[test]
    fn test_any_model_debug() {
        let model = hexahedron_model();
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("PlatonicSolid"));
    }

    #[test]
    fn test_any_model_clone() {
        let model = hexahedron_model();
        let cloned = model.clone();
        assert_eq!(model, cloned);
    }
}
