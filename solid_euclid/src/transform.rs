//! Transform module for applying geometric transformations to geometry models.
//!
//! This module provides types and functionality for transforming geometry models
//! using various transformation types including matrices, quaternions, translations,
//! rotations, and scaling operations.

use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::polygon_mesh::PolygonMesh;
use glam::{DMat4, DQuat, DVec3, DVec4};
use serde::{Deserialize, Serialize};

/// Represents different types of geometric transformations.
///
/// Each variant defines a specific transformation that can be applied
/// to geometry vertices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Transformation {
    /// An affine 4x4 transformation matrix (last row `[0, 0, 0, 1]`).
    Matrix(DMat4),
    /// A rotation represented as a unit quaternion.
    Quaternion(DQuat),
    /// A translation vector.
    Translation(DVec3),
    /// A uniform scale factor applied equally to all axes.
    UniformScale(f64),
    /// A non-uniform scale with different factors for each axis.
    ScaleXYZ(DVec3),
    /// A rotation around the X-axis in radians.
    RotationX(f64),
    /// A rotation around the Y-axis in radians.
    RotationY(f64),
    /// A rotation around the Z-axis in radians.
    RotationZ(f64),
}

impl Transformation {
    /// Converts the transformation to a 4x4 transformation matrix.
    ///
    /// This allows all transformation types to be applied uniformly
    /// by converting them to their matrix representation.
    pub fn to_matrix(&self) -> DMat4 {
        match self {
            Transformation::Matrix(mat) => *mat,
            Transformation::Quaternion(quat) => DMat4::from_quat(*quat),
            Transformation::Translation(vec) => DMat4::from_translation(*vec),
            Transformation::UniformScale(scale) => {
                DMat4::from_scale(DVec3::new(*scale, *scale, *scale))
            }
            Transformation::ScaleXYZ(scale) => DMat4::from_scale(*scale),
            Transformation::RotationX(angle) => DMat4::from_rotation_x(*angle),
            Transformation::RotationY(angle) => DMat4::from_rotation_y(*angle),
            Transformation::RotationZ(angle) => DMat4::from_rotation_z(*angle),
        }
    }

    /// Transforms a position vector using this transformation.
    ///
    /// The position is treated as a point in 3D space (w=1).
    pub fn transform_position(&self, position: DVec3) -> DVec3 {
        let mat = self.to_matrix();
        let pos4 = DVec4::new(position.x, position.y, position.z, 1.0);
        let result = mat * pos4;
        DVec3::new(result.x, result.y, result.z)
    }

    /// Transforms a normal vector using this transformation.
    ///
    /// Normals are transformed with the cofactor (adjugate) transpose of the
    /// linear part of the transformation matrix, which preserves
    /// perpendicularity like the inverse transpose but, unlike the inverse,
    /// is also defined for singular matrices: a collapsing transform (for
    /// example a zero scale) yields finite normals instead of NaN. For
    /// invertible matrices the adjugate is the determinant times the inverse,
    /// so a negative determinant (mirroring) flips the sign, matching the
    /// flipped winding. Faces that collapse to a line or point have no
    /// geometric normal; they keep their original normal. The result is
    /// normalized.
    pub fn transform_normal(&self, normal: DVec3) -> DVec3 {
        let mat = self.to_matrix();
        let row0 = DVec3::new(mat.x_axis.x, mat.y_axis.x, mat.z_axis.x);
        let row1 = DVec3::new(mat.x_axis.y, mat.y_axis.y, mat.z_axis.y);
        let row2 = DVec3::new(mat.x_axis.z, mat.y_axis.z, mat.z_axis.z);
        let transformed = DVec3::new(
            row1.cross(row2).dot(normal),
            row2.cross(row0).dot(normal),
            row0.cross(row1).dot(normal),
        );
        let length = transformed.length();
        if length > 0.0 && length.is_finite() {
            let mut result = transformed / length;
            if mat.determinant() < 0.0 {
                result = -result;
            }
            result
        } else {
            normal
        }
    }

    /// Validates that the transformation parameters are usable.
    ///
    /// NaN or infinite values would produce degenerate geometry, so they are
    /// rejected with [`GeometryError::InvalidParameter`]. Matrices must be
    /// affine (last row `[0, 0, 0, 1]`), because positions are transformed
    /// with the homogeneous coordinate discarded. Quaternions must have unit
    /// length, because [`DMat4::from_quat`] requires one and a non-unit
    /// quaternion would silently introduce scaling.
    pub fn validate(&self) -> Result<(), GeometryError> {
        fn check(parameter: &str, value: f64) -> Result<(), GeometryError> {
            if value.is_finite() {
                Ok(())
            } else {
                Err(GeometryError::InvalidParameter {
                    parameter: parameter.to_string(),
                    message: format!("value must be finite, got {value}"),
                })
            }
        }

        match self {
            Transformation::Matrix(mat) => {
                for (col_index, col) in mat.to_cols_array_2d().iter().enumerate() {
                    for (row_index, value) in col.iter().enumerate() {
                        check(&format!("matrix[{row_index}][{col_index}]"), *value)?;
                    }
                }
                // Positions are transformed with w = 1 and the homogeneous
                // coordinate is discarded, so only affine matrices (last row
                // [0, 0, 0, 1]) are supported.
                if [mat.x_axis.w, mat.y_axis.w, mat.z_axis.w, mat.w_axis.w] != [0.0, 0.0, 0.0, 1.0]
                {
                    return Err(GeometryError::InvalidParameter {
                        parameter: "matrix".to_string(),
                        message: "matrix must be affine (last row [0, 0, 0, 1])".to_string(),
                    });
                }
                Ok(())
            }
            Transformation::Quaternion(quat) => {
                check("quaternion.x", quat.x)?;
                check("quaternion.y", quat.y)?;
                check("quaternion.z", quat.z)?;
                check("quaternion.w", quat.w)?;
                // DMat4::from_quat requires a unit quaternion; a non-unit one
                // would silently introduce scaling instead of a pure rotation.
                // A zero-length quaternion fails this check as well.
                const UNIT_LENGTH_EPSILON: f64 = 1e-9;
                let length = quat.length();
                if (length - 1.0).abs() > UNIT_LENGTH_EPSILON {
                    return Err(GeometryError::InvalidParameter {
                        parameter: "quaternion".to_string(),
                        message: format!("quaternion must have unit length, got length {length}"),
                    });
                }
                Ok(())
            }
            Transformation::Translation(vec) => {
                check("translation.x", vec.x)?;
                check("translation.y", vec.y)?;
                check("translation.z", vec.z)
            }
            Transformation::UniformScale(scale) => check("scale", *scale),
            Transformation::ScaleXYZ(scale) => {
                check("scale.x", scale.x)?;
                check("scale.y", scale.y)?;
                check("scale.z", scale.z)
            }
            Transformation::RotationX(angle) => check("angle", *angle),
            Transformation::RotationY(angle) => check("angle", *angle),
            Transformation::RotationZ(angle) => check("angle", *angle),
        }
    }
}

impl Default for Transformation {
    fn default() -> Self {
        Transformation::Matrix(DMat4::IDENTITY)
    }
}

/// A wrapper that applies a transformation to a geometry model.
///
/// This struct combines a transformation with a geometry model,
/// allowing the transformation to be applied when generating meshes.
///
/// # Type Parameters
///
/// * `T` - The type of the inner geometry model, which must implement `GeometryModel`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform<T>
where
    T: GeometryModel,
{
    /// The transformation to apply to the geometry.
    pub transformation: Transformation,
    /// The inner geometry model to be transformed.
    pub model: T,
}

impl<T> Transform<T>
where
    T: GeometryModel,
{
    /// Creates a new `Transform` with the given transformation and model.
    pub fn new(transformation: Transformation, model: T) -> Self {
        Self {
            transformation,
            model,
        }
    }

    /// Creates a new `Transform` with a matrix transformation.
    pub fn with_matrix(matrix: DMat4, model: T) -> Self {
        Self::new(Transformation::Matrix(matrix), model)
    }

    /// Creates a new `Transform` with a quaternion rotation.
    pub fn with_quaternion(quaternion: DQuat, model: T) -> Self {
        Self::new(Transformation::Quaternion(quaternion), model)
    }

    /// Creates a new `Transform` with a translation.
    pub fn with_translation(translation: DVec3, model: T) -> Self {
        Self::new(Transformation::Translation(translation), model)
    }

    /// Creates a new `Transform` with a uniform scale.
    pub fn with_uniform_scale(scale: f64, model: T) -> Self {
        Self::new(Transformation::UniformScale(scale), model)
    }

    /// Creates a new `Transform` with non-uniform scale.
    pub fn with_scale_xyz(scale: DVec3, model: T) -> Self {
        Self::new(Transformation::ScaleXYZ(scale), model)
    }

    /// Creates a new `Transform` with a rotation around the X-axis.
    pub fn with_rotation_x(angle: f64, model: T) -> Self {
        Self::new(Transformation::RotationX(angle), model)
    }

    /// Creates a new `Transform` with a rotation around the Y-axis.
    pub fn with_rotation_y(angle: f64, model: T) -> Self {
        Self::new(Transformation::RotationY(angle), model)
    }

    /// Creates a new `Transform` with a rotation around the Z-axis.
    pub fn with_rotation_z(angle: f64, model: T) -> Self {
        Self::new(Transformation::RotationZ(angle), model)
    }
}

impl<T> GeometryModel for Transform<T>
where
    T: GeometryModel,
{
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        self.transformation.validate()?;

        // Get the polygon mesh from the inner model
        let mut mesh = self.model.to_polygon_mesh()?;

        // A negative determinant mirrors the geometry, flipping the face winding
        let flip_winding = self.transformation.to_matrix().determinant() < 0.0;

        // Apply the transformation to all vertices
        for vertex in &mut mesh.vertices {
            vertex.position = self.transformation.transform_position(vertex.position);
            vertex.normal = self.transformation.transform_normal(vertex.normal);
        }

        // Update polygon normals
        for polygon in &mut mesh.polygons {
            polygon.normal = self.transformation.transform_normal(polygon.normal);
            if flip_winding {
                polygon.indices.reverse();
            }
        }

        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_mesh::Polygon;
    use crate::vertex::Vertex;
    use glam::DVec2;
    use std::f64::consts::{FRAC_PI_2, PI};

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec3_approx_eq(a: DVec3, b: DVec3) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
    }

    /// A simple test geometry model representing a unit triangle at the origin.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestTriangleModel;

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
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec2::new(0.0, 1.0),
            ));

            mesh.add_polygon(Polygon::new(vec![0, 1, 2], DVec3::Z, 1));
            mesh.set_material(1, "test_material".to_string());

            Ok(mesh)
        }
    }

    /// A test geometry model representing a unit cube centered at the origin.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestCubeModel;

    impl GeometryModel for TestCubeModel {
        fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
            let mut mesh = PolygonMesh::new();

            // Add 8 vertices for a unit cube centered at origin
            let half = 0.5;
            let positions = [
                DVec3::new(-half, -half, -half), // 0
                DVec3::new(half, -half, -half),  // 1
                DVec3::new(half, half, -half),   // 2
                DVec3::new(-half, half, -half),  // 3
                DVec3::new(-half, -half, half),  // 4
                DVec3::new(half, -half, half),   // 5
                DVec3::new(half, half, half),    // 6
                DVec3::new(-half, half, half),   // 7
            ];

            for pos in positions {
                mesh.add_vertex(Vertex::new(pos, pos.normalize(), DVec2::ZERO));
            }

            // Add 6 faces
            mesh.add_polygon(Polygon::new(vec![0, 1, 2, 3], -DVec3::Z, 1)); // back
            mesh.add_polygon(Polygon::new(vec![4, 7, 6, 5], DVec3::Z, 1)); // front
            mesh.add_polygon(Polygon::new(vec![0, 4, 5, 1], -DVec3::Y, 1)); // bottom
            mesh.add_polygon(Polygon::new(vec![2, 6, 7, 3], DVec3::Y, 1)); // top
            mesh.add_polygon(Polygon::new(vec![0, 3, 7, 4], -DVec3::X, 1)); // left
            mesh.add_polygon(Polygon::new(vec![1, 5, 6, 2], DVec3::X, 1)); // right

            mesh.set_material(1, "cube_material".to_string());

            Ok(mesh)
        }
    }

    // ==================== Transformation enum tests ====================

    #[test]
    fn test_transformation_matrix_to_matrix() {
        let mat = DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
        let transformation = Transformation::Matrix(mat);
        assert_eq!(transformation.to_matrix(), mat);
    }

    #[test]
    fn test_transformation_quaternion_to_matrix() {
        let quat = DQuat::from_rotation_z(FRAC_PI_2);
        let transformation = Transformation::Quaternion(quat);
        let expected = DMat4::from_quat(quat);
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_translation_to_matrix() {
        let translation = DVec3::new(1.0, 2.0, 3.0);
        let transformation = Transformation::Translation(translation);
        let expected = DMat4::from_translation(translation);
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_uniform_scale_to_matrix() {
        let scale = 2.0;
        let transformation = Transformation::UniformScale(scale);
        let expected = DMat4::from_scale(DVec3::new(scale, scale, scale));
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_scale_xyz_to_matrix() {
        let scale = DVec3::new(1.0, 2.0, 3.0);
        let transformation = Transformation::ScaleXYZ(scale);
        let expected = DMat4::from_scale(scale);
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_rotation_x_to_matrix() {
        let angle = FRAC_PI_2;
        let transformation = Transformation::RotationX(angle);
        let expected = DMat4::from_rotation_x(angle);
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_rotation_y_to_matrix() {
        let angle = FRAC_PI_2;
        let transformation = Transformation::RotationY(angle);
        let expected = DMat4::from_rotation_y(angle);
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_rotation_z_to_matrix() {
        let angle = FRAC_PI_2;
        let transformation = Transformation::RotationZ(angle);
        let expected = DMat4::from_rotation_z(angle);
        assert_eq!(transformation.to_matrix(), expected);
    }

    #[test]
    fn test_transformation_default() {
        let transformation = Transformation::default();
        assert_eq!(transformation, Transformation::Matrix(DMat4::IDENTITY));
    }

    #[test]
    fn test_transformation_clone() {
        let transformation = Transformation::Translation(DVec3::new(1.0, 2.0, 3.0));
        let cloned = transformation.clone();
        assert_eq!(transformation, cloned);
    }

    #[test]
    fn test_transformation_debug() {
        let transformation = Transformation::UniformScale(2.0);
        let debug_str = format!("{:?}", transformation);
        assert!(debug_str.contains("UniformScale"));
        assert!(debug_str.contains("2.0"));
    }

    // ==================== Transform position tests ====================

    #[test]
    fn test_transform_position_identity() {
        let transformation = Transformation::Matrix(DMat4::IDENTITY);
        let position = DVec3::new(1.0, 2.0, 3.0);
        let result = transformation.transform_position(position);
        assert!(vec3_approx_eq(result, position));
    }

    #[test]
    fn test_transform_position_translation() {
        let translation = DVec3::new(10.0, 20.0, 30.0);
        let transformation = Transformation::Translation(translation);
        let position = DVec3::new(1.0, 2.0, 3.0);
        let result = transformation.transform_position(position);
        assert!(vec3_approx_eq(result, position + translation));
    }

    #[test]
    fn test_transform_position_uniform_scale() {
        let transformation = Transformation::UniformScale(2.0);
        let position = DVec3::new(1.0, 2.0, 3.0);
        let result = transformation.transform_position(position);
        assert!(vec3_approx_eq(result, DVec3::new(2.0, 4.0, 6.0)));
    }

    #[test]
    fn test_transform_position_scale_xyz() {
        let transformation = Transformation::ScaleXYZ(DVec3::new(2.0, 3.0, 4.0));
        let position = DVec3::new(1.0, 1.0, 1.0);
        let result = transformation.transform_position(position);
        assert!(vec3_approx_eq(result, DVec3::new(2.0, 3.0, 4.0)));
    }

    #[test]
    fn test_transform_position_rotation_x_90() {
        let transformation = Transformation::RotationX(FRAC_PI_2);
        let position = DVec3::new(0.0, 1.0, 0.0);
        let result = transformation.transform_position(position);
        // Y-axis rotated 90 degrees around X should become Z-axis
        assert!(vec3_approx_eq(result, DVec3::new(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_transform_position_rotation_y_90() {
        let transformation = Transformation::RotationY(FRAC_PI_2);
        let position = DVec3::new(1.0, 0.0, 0.0);
        let result = transformation.transform_position(position);
        // X-axis rotated 90 degrees around Y should become -Z-axis
        assert!(vec3_approx_eq(result, DVec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn test_transform_position_rotation_z_90() {
        let transformation = Transformation::RotationZ(FRAC_PI_2);
        let position = DVec3::new(1.0, 0.0, 0.0);
        let result = transformation.transform_position(position);
        // X-axis rotated 90 degrees around Z should become Y-axis
        assert!(vec3_approx_eq(result, DVec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn test_transform_position_quaternion() {
        let quat = DQuat::from_rotation_z(FRAC_PI_2);
        let transformation = Transformation::Quaternion(quat);
        let position = DVec3::new(1.0, 0.0, 0.0);
        let result = transformation.transform_position(position);
        assert!(vec3_approx_eq(result, DVec3::new(0.0, 1.0, 0.0)));
    }

    // ==================== Transform normal tests ====================

    #[test]
    fn test_transform_normal_identity() {
        let transformation = Transformation::Matrix(DMat4::IDENTITY);
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let result = transformation.transform_normal(normal);
        assert!(vec3_approx_eq(result, normal));
    }

    #[test]
    fn test_transform_normal_translation() {
        // Translation should not affect normals
        let transformation = Transformation::Translation(DVec3::new(100.0, 200.0, 300.0));
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let result = transformation.transform_normal(normal);
        assert!(vec3_approx_eq(result, normal));
    }

    #[test]
    fn test_transform_normal_uniform_scale() {
        // Uniform scale should not change the direction of normals
        let transformation = Transformation::UniformScale(5.0);
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let result = transformation.transform_normal(normal);
        assert!(vec3_approx_eq(result, normal));
    }

    #[test]
    fn test_transform_normal_rotation_z_90() {
        let transformation = Transformation::RotationZ(FRAC_PI_2);
        let normal = DVec3::new(1.0, 0.0, 0.0);
        let result = transformation.transform_normal(normal);
        assert!(vec3_approx_eq(result, DVec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn test_transform_normal_singular_uniform_scale() {
        // A zero scale is singular: the inverse does not exist, but the
        // collapsed geometry keeps its original finite normal
        let transformation = Transformation::UniformScale(0.0);
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let result = transformation.transform_normal(normal);
        assert!(result.is_finite());
        assert!(vec3_approx_eq(result, normal));
    }

    #[test]
    fn test_transform_normal_singular_scale_xyz() {
        // The x axis collapses; faces normal to it survive, the rest collapse
        let transformation = Transformation::ScaleXYZ(DVec3::new(0.0, 1.0, 1.0));

        let result = transformation.transform_normal(DVec3::X);
        assert!(vec3_approx_eq(result, DVec3::X));

        for normal in [DVec3::NEG_X, DVec3::Y, DVec3::Z] {
            let result = transformation.transform_normal(normal);
            assert!(result.is_finite());
            assert!(vec3_approx_eq(result, normal));
        }
    }

    #[test]
    fn test_transform_normal_matches_inverse_transpose() {
        // For invertible matrices the cofactor transpose is the determinant
        // times the inverse transpose, so the normalized result is unchanged
        for transformation in [
            Transformation::ScaleXYZ(DVec3::new(2.0, 3.0, 4.0)),
            Transformation::ScaleXYZ(DVec3::new(-2.0, 3.0, 4.0)),
            Transformation::RotationX(0.7),
            Transformation::Matrix(
                DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0))
                    * DMat4::from_rotation_y(1.1)
                    * DMat4::from_scale(DVec3::new(2.0, 0.5, 3.0)),
            ),
        ] {
            let normal = DVec3::new(1.0, 2.0, 3.0).normalize();
            let mat = transformation.to_matrix();
            let expected = (mat.inverse().transpose()
                * DVec4::new(normal.x, normal.y, normal.z, 0.0))
            .truncate()
            .normalize();
            let result = transformation.transform_normal(normal);
            assert!(vec3_approx_eq(result, expected));
        }
    }

    // ==================== Transform struct tests ====================

    #[test]
    fn test_transform_new() {
        let model = TestTriangleModel;
        let transformation = Transformation::Translation(DVec3::new(1.0, 2.0, 3.0));
        let transform = Transform::new(transformation.clone(), model.clone());

        assert_eq!(transform.transformation, transformation);
        assert_eq!(transform.model, model);
    }

    #[test]
    fn test_transform_with_matrix() {
        let model = TestTriangleModel;
        let matrix = DMat4::from_scale(DVec3::splat(2.0));
        let transform = Transform::with_matrix(matrix, model);

        assert_eq!(transform.transformation, Transformation::Matrix(matrix));
    }

    #[test]
    fn test_transform_with_quaternion() {
        let model = TestTriangleModel;
        let quat = DQuat::from_rotation_z(PI);
        let transform = Transform::with_quaternion(quat, model);

        assert_eq!(transform.transformation, Transformation::Quaternion(quat));
    }

    #[test]
    fn test_transform_with_translation() {
        let model = TestTriangleModel;
        let translation = DVec3::new(5.0, 6.0, 7.0);
        let transform = Transform::with_translation(translation, model);

        assert_eq!(
            transform.transformation,
            Transformation::Translation(translation)
        );
    }

    #[test]
    fn test_transform_with_uniform_scale() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(3.0, model);

        assert_eq!(transform.transformation, Transformation::UniformScale(3.0));
    }

    #[test]
    fn test_transform_with_scale_xyz() {
        let model = TestTriangleModel;
        let scale = DVec3::new(1.0, 2.0, 3.0);
        let transform = Transform::with_scale_xyz(scale, model);

        assert_eq!(transform.transformation, Transformation::ScaleXYZ(scale));
    }

    #[test]
    fn test_transform_with_rotation_x() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_x(FRAC_PI_2, model);

        assert_eq!(
            transform.transformation,
            Transformation::RotationX(FRAC_PI_2)
        );
    }

    #[test]
    fn test_transform_with_rotation_y() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_y(FRAC_PI_2, model);

        assert_eq!(
            transform.transformation,
            Transformation::RotationY(FRAC_PI_2)
        );
    }

    #[test]
    fn test_transform_with_rotation_z() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_z(FRAC_PI_2, model);

        assert_eq!(
            transform.transformation,
            Transformation::RotationZ(FRAC_PI_2)
        );
    }

    #[test]
    fn test_transform_clone() {
        let model = TestTriangleModel;
        let transform = Transform::with_translation(DVec3::new(1.0, 2.0, 3.0), model);
        let cloned = transform.clone();

        assert_eq!(transform, cloned);
    }

    #[test]
    fn test_transform_debug() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(2.0, model);
        let debug_str = format!("{:?}", transform);

        assert!(debug_str.contains("Transform"));
        assert!(debug_str.contains("UniformScale"));
    }

    // ==================== GeometryModel implementation tests ====================

    #[test]
    fn test_geometry_model_identity_transform() {
        let model = TestTriangleModel;
        let transform = Transform::new(Transformation::Matrix(DMat4::IDENTITY), model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        assert_eq!(
            original_mesh.vertex_count(),
            transformed_mesh.vertex_count()
        );
        assert_eq!(
            original_mesh.polygon_count(),
            transformed_mesh.polygon_count()
        );

        for (orig, trans) in original_mesh
            .vertices
            .iter()
            .zip(transformed_mesh.vertices.iter())
        {
            assert!(vec3_approx_eq(orig.position, trans.position));
            assert!(vec3_approx_eq(orig.normal, trans.normal));
        }
    }

    #[test]
    fn test_geometry_model_translation_transform() {
        let model = TestTriangleModel;
        let translation = DVec3::new(10.0, 20.0, 30.0);
        let transform = Transform::with_translation(translation, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Check that positions are translated
        for (orig, trans) in original_mesh
            .vertices
            .iter()
            .zip(transformed_mesh.vertices.iter())
        {
            assert!(vec3_approx_eq(trans.position, orig.position + translation));
            // Normals should be unchanged
            assert!(vec3_approx_eq(trans.normal, orig.normal));
        }
    }

    #[test]
    fn test_geometry_model_uniform_scale_transform() {
        let model = TestTriangleModel;
        let scale = 2.0;
        let transform = Transform::with_uniform_scale(scale, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Check that positions are scaled
        for (orig, trans) in original_mesh
            .vertices
            .iter()
            .zip(transformed_mesh.vertices.iter())
        {
            assert!(vec3_approx_eq(trans.position, orig.position * scale));
            // Normals should be unchanged (direction-wise) for uniform scale
            assert!(vec3_approx_eq(trans.normal, orig.normal));
        }
    }

    #[test]
    fn test_geometry_model_rotation_z_transform() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_z(FRAC_PI_2, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Vertex at (1, 0, 0) should become (0, 1, 0)
        let orig_pos = original_mesh.vertices[1].position;
        let trans_pos = transformed_mesh.vertices[1].position;

        assert!(approx_eq(orig_pos.x, 1.0));
        assert!(approx_eq(orig_pos.y, 0.0));
        assert!(approx_eq(trans_pos.x, 0.0));
        assert!(approx_eq(trans_pos.y, 1.0));
    }

    #[test]
    fn test_geometry_model_cube_scale_xyz() {
        let model = TestCubeModel;
        let scale = DVec3::new(2.0, 3.0, 4.0);
        let transform = Transform::with_scale_xyz(scale, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Check that positions are scaled correctly per axis
        for (orig, trans) in original_mesh
            .vertices
            .iter()
            .zip(transformed_mesh.vertices.iter())
        {
            assert!(approx_eq(trans.position.x, orig.position.x * scale.x));
            assert!(approx_eq(trans.position.y, orig.position.y * scale.y));
            assert!(approx_eq(trans.position.z, orig.position.z * scale.z));
        }
    }

    #[test]
    fn test_geometry_model_polygon_normals_updated() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_x(FRAC_PI_2, model);

        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Original polygon normal is (0, 0, 1), after 90° rotation around X it should be (0, -1, 0)
        let polygon_normal = transformed_mesh.polygons[0].normal;
        assert!(vec3_approx_eq(polygon_normal, DVec3::new(0.0, -1.0, 0.0)));
    }

    #[test]
    fn test_geometry_model_material_preserved() {
        let model = TestTriangleModel;
        let transform = Transform::with_translation(DVec3::new(1.0, 2.0, 3.0), model);

        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        assert_eq!(
            transformed_mesh.get_material(1),
            Some(&"test_material".to_string())
        );
    }

    #[test]
    fn test_geometry_model_to_triangular_mesh() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(2.0, model);

        let result = transform.to_triangular_mesh();
        assert!(result.is_ok());

        let tri_mesh = result.unwrap();
        assert_eq!(tri_mesh.len(), 1);

        let geometry = tri_mesh.get("test_material").unwrap();
        assert_eq!(geometry.vertex_count(), 3);
        assert_eq!(geometry.indices.len(), 3);
    }

    #[test]
    fn test_geometry_model_cube_to_triangular_mesh() {
        let model = TestCubeModel;
        let transform = Transform::with_rotation_y(PI / 4.0, model);

        let result = transform.to_triangular_mesh();
        assert!(result.is_ok());

        let tri_mesh = result.unwrap();
        let geometry = tri_mesh.get("cube_material").unwrap();

        // 6 faces × 2 triangles × 3 indices = 36 indices
        assert_eq!(geometry.indices.len(), 36);
    }

    // ==================== Serialization tests ====================

    #[test]
    fn test_transformation_serialize_deserialize() {
        let transformations = vec![
            Transformation::Matrix(DMat4::from_scale(DVec3::splat(2.0))),
            Transformation::Quaternion(DQuat::from_rotation_z(PI)),
            Transformation::Translation(DVec3::new(1.0, 2.0, 3.0)),
            Transformation::UniformScale(5.0),
            Transformation::ScaleXYZ(DVec3::new(1.0, 2.0, 3.0)),
            Transformation::RotationX(FRAC_PI_2),
            Transformation::RotationY(FRAC_PI_2),
            Transformation::RotationZ(FRAC_PI_2),
        ];

        for transformation in transformations {
            let json = serde_json::to_string(&transformation).unwrap();
            let deserialized: Transformation = serde_json::from_str(&json).unwrap();
            assert_eq!(transformation, deserialized);
        }
    }

    #[test]
    fn test_transform_serialize_deserialize() {
        let model = TestTriangleModel;
        let transform = Transform::with_translation(DVec3::new(1.0, 2.0, 3.0), model);

        let json = serde_json::to_string(&transform).unwrap();
        let deserialized: Transform<TestTriangleModel> = serde_json::from_str(&json).unwrap();

        assert_eq!(transform, deserialized);
    }

    // ==================== Edge cases and special scenarios ====================

    #[test]
    fn test_transform_zero_scale() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(0.0, model);

        let mesh = transform.to_polygon_mesh().unwrap();

        // All positions should be at origin
        for vertex in &mesh.vertices {
            assert!(vec3_approx_eq(vertex.position, DVec3::ZERO));
        }

        // The collapsed geometry has no geometric normal; the original finite
        // normals are kept instead of NaN from inverting a singular matrix
        for vertex in &mesh.vertices {
            assert!(vertex.normal.is_finite());
            assert!(vec3_approx_eq(vertex.normal, DVec3::Z));
        }
        for polygon in &mesh.polygons {
            assert!(polygon.normal.is_finite());
            assert!(vec3_approx_eq(polygon.normal, DVec3::Z));
        }
    }

    #[test]
    fn test_transform_singular_scale_xyz_keeps_finite_normals() {
        let model = TestCubeModel;
        // The x axis collapses; the cube flattens into the y-z plane
        let transform = Transform::with_scale_xyz(DVec3::new(0.0, 1.0, 1.0), model);

        let mesh = transform.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            assert!(approx_eq(vertex.position.x, 0.0));
            assert!(vertex.position.is_finite());
            assert!(vertex.normal.is_finite());
        }
        for polygon in &mesh.polygons {
            assert!(polygon.normal.is_finite());
        }

        // The faces normal to the preserved x axis survive with their normals
        assert!(vec3_approx_eq(mesh.polygons[4].normal, -DVec3::X)); // left
        assert!(vec3_approx_eq(mesh.polygons[5].normal, DVec3::X)); // right
    }

    #[test]
    fn test_transform_zero_scale_triangular_mesh_has_finite_normals() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(0.0, model);

        let tri_mesh = transform.to_triangular_mesh().unwrap();
        let geometry = tri_mesh.get("test_material").unwrap();

        assert!(!geometry.normals.is_empty());
        for component in &geometry.normals {
            assert!(component.is_finite());
        }
    }

    #[test]
    fn test_transform_negative_scale() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(-1.0, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Positions should be negated
        for (orig, trans) in original_mesh
            .vertices
            .iter()
            .zip(transformed_mesh.vertices.iter())
        {
            assert!(vec3_approx_eq(trans.position, -orig.position));
        }
    }

    #[test]
    fn test_transform_180_degree_rotation() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_z(PI, model);

        let mesh = transform.to_polygon_mesh().unwrap();

        // Vertex at (1, 0, 0) should become (-1, 0, 0)
        let pos = mesh.vertices[1].position;
        assert!(approx_eq(pos.x, -1.0));
        assert!(approx_eq(pos.y, 0.0));
    }

    #[test]
    fn test_transform_360_degree_rotation() {
        let model = TestTriangleModel;
        let transform = Transform::with_rotation_z(2.0 * PI, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        // Should be back to original positions
        for (orig, trans) in original_mesh
            .vertices
            .iter()
            .zip(transformed_mesh.vertices.iter())
        {
            assert!(vec3_approx_eq(trans.position, orig.position));
        }
    }

    #[test]
    fn test_transform_quaternion_arbitrary_axis() {
        let model = TestTriangleModel;
        let axis = DVec3::new(1.0, 1.0, 1.0).normalize();
        let quat = DQuat::from_axis_angle(axis, FRAC_PI_2);
        let transform = Transform::with_quaternion(quat, model);

        let mesh = transform.to_polygon_mesh().unwrap();

        // Just verify it doesn't crash and produces valid geometry
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.polygon_count(), 1);
    }

    #[test]
    fn test_transform_combined_matrix() {
        let model = TestTriangleModel;
        // Create a combined transformation matrix: scale, then rotate, then translate
        let matrix = DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0))
            * DMat4::from_rotation_z(FRAC_PI_2)
            * DMat4::from_scale(DVec3::splat(2.0));

        let transform = Transform::with_matrix(matrix, model);

        let mesh = transform.to_polygon_mesh().unwrap();

        // Vertex at (1, 0, 0) scaled by 2 -> (2, 0, 0), rotated 90° -> (0, 2, 0), translated -> (10, 2, 0)
        let pos = mesh.vertices[1].position;
        assert!(approx_eq(pos.x, 10.0));
        assert!(approx_eq(pos.y, 2.0));
        assert!(approx_eq(pos.z, 0.0));
    }

    // ==================== Winding fix tests ====================

    #[test]
    fn test_negative_scale_flips_winding() {
        let model = TestCubeModel;
        let transform = Transform::with_uniform_scale(-1.0, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        for (orig, trans) in original_mesh
            .polygons
            .iter()
            .zip(transformed_mesh.polygons.iter())
        {
            let mut reversed = orig.indices.clone();
            reversed.reverse();
            assert_eq!(trans.indices, reversed);
        }
    }

    #[test]
    fn test_positive_scale_preserves_winding() {
        let model = TestCubeModel;
        let transform = Transform::with_uniform_scale(2.0, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        for (orig, trans) in original_mesh
            .polygons
            .iter()
            .zip(transformed_mesh.polygons.iter())
        {
            assert_eq!(trans.indices, orig.indices);
        }
    }

    #[test]
    fn test_rotation_preserves_winding() {
        let model = TestCubeModel;
        let transform = Transform::with_rotation_z(FRAC_PI_2, model.clone());

        let original_mesh = model.to_polygon_mesh().unwrap();
        let transformed_mesh = transform.to_polygon_mesh().unwrap();

        for (orig, trans) in original_mesh
            .polygons
            .iter()
            .zip(transformed_mesh.polygons.iter())
        {
            assert_eq!(trans.indices, orig.indices);
        }
    }

    // ==================== Validation tests ====================

    #[test]
    fn test_validate_valid_transformations() {
        let transformations = vec![
            Transformation::Matrix(DMat4::IDENTITY),
            Transformation::Quaternion(DQuat::IDENTITY),
            Transformation::Translation(DVec3::new(1.0, 2.0, 3.0)),
            Transformation::UniformScale(0.0),
            Transformation::UniformScale(-1.0),
            Transformation::ScaleXYZ(DVec3::new(-1.0, 2.0, 3.0)),
            Transformation::RotationX(FRAC_PI_2),
            Transformation::RotationY(FRAC_PI_2),
            Transformation::RotationZ(FRAC_PI_2),
        ];

        for transformation in transformations {
            assert!(transformation.validate().is_ok());
        }
    }

    #[test]
    fn test_validate_nan_matrix() {
        let mut mat = DMat4::IDENTITY;
        mat.x_axis.x = f64::NAN;
        let transformation = Transformation::Matrix(mat);

        let result = transformation.validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_infinite_uniform_scale() {
        let transformation = Transformation::UniformScale(f64::INFINITY);

        let result = transformation.validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_nan_rotation() {
        let transformation = Transformation::RotationX(f64::NAN);

        let result = transformation.validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_nan_translation() {
        let transformation = Transformation::Translation(DVec3::new(1.0, f64::NAN, 3.0));

        let result = transformation.validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_nan_quaternion() {
        let transformation = Transformation::Quaternion(DQuat::from_xyzw(f64::NAN, 0.0, 0.0, 1.0));

        let result = transformation.validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_geometry_model_rejects_nan_transformation() {
        let model = TestTriangleModel;
        let transform = Transform::with_uniform_scale(f64::NAN, model);

        let result = transform.to_polygon_mesh();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_accepts_affine_matrix() {
        let mat = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0))
            * DMat4::from_rotation_z(FRAC_PI_2)
            * DMat4::from_scale(DVec3::splat(2.0));
        assert!(Transformation::Matrix(mat).validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_non_affine_matrix() {
        // Finite, but the last row is [0, 0, 0, -1]
        let mat = DMat4::from_diagonal(DVec4::new(1.0, 1.0, 1.0, -1.0));
        let result = Transformation::Matrix(mat).validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));

        // Projective: the last row is [0, 0, 1, 1]
        let mat = DMat4::from_cols(DVec4::X, DVec4::Y, DVec4::new(0.0, 0.0, 1.0, 1.0), DVec4::W);
        let result = Transformation::Matrix(mat).validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_accepts_unit_quaternion() {
        let quat = DQuat::from_axis_angle(DVec3::new(1.0, 2.0, 3.0).normalize(), 0.7);
        assert!(Transformation::Quaternion(quat).validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_non_unit_quaternion() {
        // Finite, but length sqrt(2): DMat4::from_quat would introduce scaling
        let quat = DQuat::from_xyzw(0.0, 0.0, 1.0, 1.0);
        let result = Transformation::Quaternion(quat).validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_validate_rejects_zero_length_quaternion() {
        let quat = DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0);
        let result = Transformation::Quaternion(quat).validate();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_geometry_model_rejects_non_affine_matrix() {
        let model = TestTriangleModel;
        // Finite but non-affine: the negative determinant would flip the
        // winding while the discarded homogeneous coordinate leaves the
        // positions unchanged
        let mat = DMat4::from_diagonal(DVec4::new(1.0, 1.0, 1.0, -1.0));
        let transform = Transform::with_matrix(mat, model);

        let result = transform.to_polygon_mesh();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_geometry_model_rejects_non_unit_quaternion() {
        let model = TestTriangleModel;
        let quat = DQuat::from_xyzw(0.0, 0.0, 1.0, 1.0);
        let transform = Transform::with_quaternion(quat, model);

        let result = transform.to_polygon_mesh();
        assert!(matches!(
            result,
            Err(GeometryError::InvalidParameter { .. })
        ));
    }
}
