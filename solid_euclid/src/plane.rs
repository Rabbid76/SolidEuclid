use glam::{DMat4, DVec3, DVec4};

/// Represents a plane in 3D space defined by an origin, normal, and x-axis direction.
///
/// The plane's coordinate system is constructed as follows:
/// - Z-axis: normalized `normal`
/// - Y-axis: normalized cross product of Z-axis and `x_axis`
/// - X-axis: normalized cross product of Y-axis and Z-axis (recalculated for orthogonality)
#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    /// The origin point of the plane in 3D space.
    pub origin: DVec3,
    /// The normal vector of the plane.
    pub normal: DVec3,
    /// The x-axis direction for the coordinate system on the plane.
    pub x_axis: DVec3,
}

impl Plane {
    /// Creates a new `Plane` with the given origin, normal, and x-axis.
    pub fn new(origin: DVec3, normal: DVec3, x_axis: DVec3) -> Self {
        Self {
            origin,
            normal,
            x_axis,
        }
    }

    /// Creates a transformation matrix from this plane.
    ///
    /// The matrix transforms points from the plane's local coordinate system
    /// (where the plane lies in the XY plane at Z=0) to world coordinates.
    ///
    /// # Returns
    ///
    /// A 4x4 transformation matrix (column-major) where:
    /// - Column 0: X-axis (recalculated for orthogonality)
    /// - Column 1: Y-axis (cross product of Z and input X)
    /// - Column 2: Z-axis (normalized normal)
    /// - Column 3: Translation (origin)
    pub fn to_matrix(&self) -> DMat4 {
        // Build the coordinate system
        let z_axis = self.normal.normalize();
        let y_axis = z_axis.cross(self.x_axis).normalize();
        let x_axis_recalc = y_axis.cross(z_axis).normalize();

        // Create the transformation matrix (column-major)
        DMat4::from_cols(
            DVec4::new(x_axis_recalc.x, x_axis_recalc.y, x_axis_recalc.z, 0.0),
            DVec4::new(y_axis.x, y_axis.y, y_axis.z, 0.0),
            DVec4::new(z_axis.x, z_axis.y, z_axis.z, 0.0),
            DVec4::new(self.origin.x, self.origin.y, self.origin.z, 1.0),
        )
    }

    /// Returns the normalized normal vector of the plane.
    pub fn normalized_normal(&self) -> DVec3 {
        self.normal.normalize()
    }
}

impl Default for Plane {
    fn default() -> Self {
        Self {
            origin: DVec3::ZERO,
            normal: DVec3::Z,
            x_axis: DVec3::X,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_new() {
        let origin = DVec3::new(1.0, 2.0, 3.0);
        let normal = DVec3::new(0.0, 0.0, 1.0);
        let x_axis = DVec3::new(1.0, 0.0, 0.0);

        let plane = Plane::new(origin, normal, x_axis);

        assert_eq!(plane.origin, origin);
        assert_eq!(plane.normal, normal);
        assert_eq!(plane.x_axis, x_axis);
    }

    #[test]
    fn test_plane_default() {
        let plane = Plane::default();

        assert_eq!(plane.origin, DVec3::ZERO);
        assert_eq!(plane.normal, DVec3::Z);
        assert_eq!(plane.x_axis, DVec3::X);
    }

    #[test]
    fn test_plane_clone() {
        let plane = Plane::new(DVec3::new(1.0, 2.0, 3.0), DVec3::Y, DVec3::X);
        let cloned = plane.clone();

        assert_eq!(plane, cloned);
    }

    #[test]
    fn test_plane_normalized_normal() {
        let plane = Plane::new(DVec3::ZERO, DVec3::new(0.0, 0.0, 5.0), DVec3::X);

        let normalized = plane.normalized_normal();

        assert!((normalized - DVec3::Z).length() < 1e-10);
    }

    #[test]
    fn test_to_matrix_xy_plane_at_origin() {
        let plane = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::X);

        let matrix = plane.to_matrix();

        // For XY plane at origin with X-axis along X:
        // - Column 0 (X-axis) should be (1, 0, 0, 0)
        // - Column 1 (Y-axis) should be (0, 1, 0, 0)
        // - Column 2 (Z-axis) should be (0, 0, 1, 0)
        // - Column 3 (translation) should be (0, 0, 0, 1)
        assert!((matrix.col(0) - DVec4::new(1.0, 0.0, 0.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(1) - DVec4::new(0.0, 1.0, 0.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(2) - DVec4::new(0.0, 0.0, 1.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(3) - DVec4::new(0.0, 0.0, 0.0, 1.0)).length() < 1e-10);
    }

    #[test]
    fn test_to_matrix_translated() {
        let origin = DVec3::new(10.0, 20.0, 30.0);
        let plane = Plane::new(origin, DVec3::Z, DVec3::X);

        let matrix = plane.to_matrix();

        // Translation should be in column 3
        assert!((matrix.col(3) - DVec4::new(10.0, 20.0, 30.0, 1.0)).length() < 1e-10);
    }

    #[test]
    fn test_to_matrix_xz_plane() {
        // Plane in XZ (normal pointing +Y)
        let plane = Plane::new(DVec3::ZERO, DVec3::Y, DVec3::X);

        let matrix = plane.to_matrix();

        // Z-axis = Y (normal)
        // Y-axis = cross(Y, X) = -Z
        // X-axis = cross(-Z, Y) = X
        assert!((matrix.col(0) - DVec4::new(1.0, 0.0, 0.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(1) - DVec4::new(0.0, 0.0, -1.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(2) - DVec4::new(0.0, 1.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_to_matrix_yz_plane() {
        // Plane in YZ (normal pointing +X)
        let plane = Plane::new(DVec3::ZERO, DVec3::X, DVec3::Y);

        let matrix = plane.to_matrix();

        // Z-axis = X (normal)
        // Y-axis = cross(X, Y) = Z
        // X-axis = cross(Z, X) = Y
        assert!((matrix.col(0) - DVec4::new(0.0, 1.0, 0.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(1) - DVec4::new(0.0, 0.0, 1.0, 0.0)).length() < 1e-10);
        assert!((matrix.col(2) - DVec4::new(1.0, 0.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_to_matrix_orthogonalization() {
        // Use a non-orthogonal x_axis (45 degrees in XY plane)
        let plane = Plane::new(DVec3::ZERO, DVec3::Z, DVec3::new(1.0, 1.0, 0.0));

        let matrix = plane.to_matrix();

        // The recalculated axes should be orthonormal
        let x_col = DVec3::new(matrix.col(0).x, matrix.col(0).y, matrix.col(0).z);
        let y_col = DVec3::new(matrix.col(1).x, matrix.col(1).y, matrix.col(1).z);
        let z_col = DVec3::new(matrix.col(2).x, matrix.col(2).y, matrix.col(2).z);

        // Check orthogonality
        assert!(x_col.dot(y_col).abs() < 1e-10);
        assert!(x_col.dot(z_col).abs() < 1e-10);
        assert!(y_col.dot(z_col).abs() < 1e-10);

        // Check unit length
        assert!((x_col.length() - 1.0).abs() < 1e-10);
        assert!((y_col.length() - 1.0).abs() < 1e-10);
        assert!((z_col.length() - 1.0).abs() < 1e-10);

        // Expected values for 45-degree rotation
        let sqrt2_inv = 1.0 / 2.0_f64.sqrt();
        assert!((x_col - DVec3::new(sqrt2_inv, sqrt2_inv, 0.0)).length() < 1e-10);
        assert!((y_col - DVec3::new(-sqrt2_inv, sqrt2_inv, 0.0)).length() < 1e-10);
        assert!((z_col - DVec3::Z).length() < 1e-10);
    }

    #[test]
    fn test_to_matrix_transforms_point() {
        let origin = DVec3::new(5.0, 10.0, 15.0);
        let plane = Plane::new(origin, DVec3::Z, DVec3::X);

        let matrix = plane.to_matrix();

        // Transform a local point (1, 2, 0) to world coordinates
        let local = DVec4::new(1.0, 2.0, 0.0, 1.0);
        let world = matrix * local;

        // Should be origin + (1, 2, 0)
        assert!((world.x - 6.0).abs() < 1e-10);
        assert!((world.y - 12.0).abs() < 1e-10);
        assert!((world.z - 15.0).abs() < 1e-10);
        assert!((world.w - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_to_matrix_non_unit_normal() {
        // Test with a non-unit normal that should be normalized
        let plane = Plane::new(DVec3::ZERO, DVec3::new(0.0, 0.0, 5.0), DVec3::X);

        let matrix = plane.to_matrix();

        // Z-axis should still be unit length
        let z_col = DVec3::new(matrix.col(2).x, matrix.col(2).y, matrix.col(2).z);
        assert!((z_col.length() - 1.0).abs() < 1e-10);
        assert!((z_col - DVec3::Z).length() < 1e-10);
    }
}
