//! Sweep-based geometric primitives.
//!
//! This module provides geometric shapes that are defined by sweeping a profile along a path.

use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::material::Material;
use crate::planar_contour::{ContourSet, SampleContour};
use crate::planar_profile::{Corner, PathCommand, PlanarProfile, ProfileCommand};
use crate::polygon_mesh::PolygonMesh;
use crate::sweep_model::SweepModel;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Represents a torus (donut shape) defined by major and minor radii.
///
/// A torus is created by sweeping a circular profile (minor circle) along
/// a circular path (major circle). The major radius defines the distance
/// from the center of the torus to the center of the tube, while the minor
/// radius defines the radius of the tube itself.
///
/// # Example
/// ```
/// use solid_euclid::sweeps::Torus;
/// use solid_euclid::material::Material;
///
/// let torus = Torus::new(
///     2.0,  // major_radius
///     0.5,  // minor_radius
///     36,   // major_segments
///     18,   // minor_segments
///     Material::Default,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torus {
    /// The major radius (distance from center to tube center).
    pub major_radius: f64,
    /// The minor radius (radius of the tube).
    pub minor_radius: f64,
    /// The number of segments around the major circle.
    pub major_segments: u32,
    /// The number of segments around the minor circle.
    pub minor_segments: u32,
    /// The material assignment for the torus surface.
    pub material: Material,
}

impl Torus {
    /// Creates a new `Torus` with the given parameters.
    ///
    /// # Arguments
    /// * `major_radius` - The distance from the center to the tube center
    /// * `minor_radius` - The radius of the tube
    /// * `major_segments` - The number of segments around the major circle
    /// * `minor_segments` - The number of segments around the minor circle
    /// * `material` - The material assignment for the surface
    ///
    /// # Panics
    /// This function does not panic, but invalid parameters will result in
    /// errors when sampling the contour.
    pub fn new(
        major_radius: f64,
        minor_radius: f64,
        major_segments: u32,
        minor_segments: u32,
        material: Material,
    ) -> Self {
        Self {
            major_radius,
            minor_radius,
            major_segments,
            minor_segments,
            material,
        }
    }

    /// Creates a circular `PlanarProfile` centered at the origin.
    ///
    /// The circle is defined by a Move command to the start point (radius, 0)
    /// followed by an Ellipse command back to the same point, creating a full circle.
    ///
    /// # Arguments
    /// * `radius` - The radius of the circle
    ///
    /// # Returns
    /// A `PlanarProfile` representing a circle in the XY plane centered at origin
    pub(crate) fn create_circle_profile(radius: f64) -> PlanarProfile {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(radius, 0.0);

        PlanarProfile::with_commands(vec![
            ProfileCommand::new(PathCommand::Move { point: start }, Corner::Edge),
            ProfileCommand::new(
                PathCommand::Ellipse {
                    point: start, // Same as start = full circle
                    center,
                },
                Corner::SmoothEdge,
            ),
        ])
    }

    /// Creates a `SweepModel` from this torus.
    ///
    /// The sweep model uses:
    /// - A circular path with the major radius
    /// - A circular profile with the minor radius
    /// - Sample distances calculated from the circumferences and segment counts
    ///
    /// # Returns
    /// A `SweepModel` that can be used to generate the torus geometry
    fn to_sweep_model(&self) -> SweepModel<PlanarProfile, PlanarProfile> {
        let path = Self::create_circle_profile(self.major_radius);
        let profile = Self::create_circle_profile(self.minor_radius);

        // Calculate sample distances from circumferences and segment counts
        let major_circumference = 2.0 * PI * self.major_radius;
        let minor_circumference = 2.0 * PI * self.minor_radius;

        let path_sample_distance = major_circumference / self.major_segments as f64;
        let profile_sample_distance = minor_circumference / self.minor_segments as f64;

        SweepModel::new(
            path,
            profile,
            path_sample_distance,
            profile_sample_distance,
            self.material.clone(),
        )
    }
}

impl GeometryModel for Torus {
    /// Converts the torus to a polygon mesh.
    ///
    /// This method creates a `SweepModel` from the torus parameters and
    /// delegates to the sweep model's `to_polygon_mesh` implementation.
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the torus,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The major or minor radius is invalid (zero or negative)
    /// - The segment counts are zero
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        // Validate parameters
        if self.major_radius <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "major_radius".to_string(),
                message: "Major radius must be positive".to_string(),
            });
        }
        if self.minor_radius <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "minor_radius".to_string(),
                message: "Minor radius must be positive".to_string(),
            });
        }
        if self.major_segments == 0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "major_segments".to_string(),
                message: "Major segments must be greater than zero".to_string(),
            });
        }
        if self.minor_segments == 0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "minor_segments".to_string(),
                message: "Minor segments must be greater than zero".to_string(),
            });
        }

        let sweep_model = self.to_sweep_model();
        sweep_model.to_polygon_mesh()
    }
}

impl SampleContour for Torus {
    /// Samples the torus contour by creating a sweep model and delegating to its implementation.
    ///
    /// This method creates a `SweepModel` from the torus parameters and then
    /// uses the `SweepModel`'s `SampleContour` implementation to generate the contour.
    ///
    /// # Arguments
    /// * `sample_distance` - The maximum distance between consecutive sample points
    ///                       (this parameter is ignored as the torus uses its own
    ///                       calculated sample distances based on segment counts)
    ///
    /// # Returns
    /// A `Result` containing the `ContourSet` with sampled segments, or a `GeometryError`
    /// if the sampling fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The major or minor radius is invalid (zero or negative)
    /// - The segment counts are zero
    fn sample_contour(&self, _sample_distance: f64) -> Result<ContourSet, GeometryError> {
        // Validate parameters
        if self.major_radius <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "major_radius".to_string(),
                message: "Major radius must be positive".to_string(),
            });
        }
        if self.minor_radius <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "minor_radius".to_string(),
                message: "Minor radius must be positive".to_string(),
            });
        }
        if self.major_segments == 0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "major_segments".to_string(),
                message: "Major segments must be greater than zero".to_string(),
            });
        }
        if self.minor_segments == 0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "minor_segments".to_string(),
                message: "Minor segments must be greater than zero".to_string(),
            });
        }

        let sweep_model = self.to_sweep_model();

        // Use the path sample distance for the sweep model's contour sampling
        let major_circumference = 2.0 * PI * self.major_radius;
        let path_sample_distance = major_circumference / self.major_segments as f64;

        sweep_model.path.sample_contour(path_sample_distance)
    }
}

/// Represents a capsule (stadium of revolution) defined by radius and height.
///
/// A capsule is created by sweeping a capsule profile (quarter circle + line + quarter circle)
/// along a circular path. The capsule is oriented along the Y-axis, with the center
/// of the cylindrical body at the origin.
///
/// The capsule supports 3 material regions:
/// - Face 0: Bottom hemisphere
/// - Face 1: Cylindrical body
/// - Face 2: Top hemisphere
///
/// # Example
/// ```
/// use solid_euclid::sweeps::Capsule;
/// use solid_euclid::material::Material;
///
/// let capsule = Capsule::new(
///     0.5,  // radius
///     2.0,  // height (length of cylindrical body)
///     24,   // segments
///     Material::Default,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capsule {
    /// The radius of the cylinder and hemispheres.
    pub radius: f64,
    /// The height (length) of the cylindrical body.
    pub height: f64,
    /// The number of segments for tessellation.
    pub segments: u32,
    /// The material assignment for the capsule surfaces.
    pub material: Material,
}

impl Capsule {
    /// Creates a new `Capsule` with the given parameters.
    ///
    /// # Arguments
    /// * `radius` - The radius of the cylinder and hemispheres
    /// * `height` - The length of the cylindrical body
    /// * `segments` - The number of segments for tessellation
    /// * `material` - The material assignment for the surfaces
    ///
    /// # Panics
    /// This function does not panic, but invalid parameters will result in
    /// errors when generating the mesh.
    pub fn new(radius: f64, height: f64, segments: u32, material: Material) -> Self {
        Self {
            radius,
            height,
            segments,
            material,
        }
    }

    /// Creates a circular path profile (same as Torus).
    ///
    /// # Arguments
    /// * `radius` - The radius of the circular path
    fn create_circle_path(radius: f64) -> PlanarProfile {
        Torus::create_circle_profile(radius)
    }

    /// Creates a `SweepModel` from this capsule.
    ///
    /// The sweep model uses:
    /// - A circular path (the revolution around Y-axis)
    /// - A capsule profile (quarter circle + line + quarter circle)
    /// - Sample distances calculated from the geometry and segment count
    ///
    /// Note: The path is the major circle that creates the rotation around the axis.
    /// The profile X coordinate represents the radial distance from the axis,
    /// and profile Y coordinate represents the height along the axis.
    ///
    /// # Returns
    /// A `SweepModel` that can be used to generate the capsule geometry
    fn to_sweep_model(&self) -> SweepModel<PlanarProfile, PlanarProfile> {
        let half_height = self.height / 2.0;

        // The path creates the revolution around the Y-axis
        // We use the capsule radius for the path, but the profile will be
        // adjusted so that X=0 is at the axis (profile X is offset)
        let path = Self::create_circle_path(self.radius);

        // Profile is the capsule cross-section, but we need to offset it
        // so that the axis of revolution is at the path's center
        // Profile X will be relative to the path center, so X=0 in profile
        // maps to X=path_radius in 3D
        let profile = Self::create_capsule_profile_offset(self.radius, half_height);

        // Calculate sample distances
        let path_circumference = 2.0 * PI * self.radius;
        let path_sample_distance = path_circumference / self.segments as f64;

        // Profile length: quarter circle + line + quarter circle
        // = (PI/2 * radius) + height + (PI/2 * radius) = PI * radius + height
        let profile_length = PI * self.radius + self.height;
        let profile_segments = self.segments / 2;
        let profile_sample_distance = profile_length / profile_segments.max(1) as f64;

        SweepModel::new(
            path,
            profile,
            path_sample_distance,
            profile_sample_distance,
            self.material.clone(),
        )
    }

    /// Creates the capsule profile with offset for sweep model.
    ///
    /// The profile is offset so that X=0 in profile space corresponds to X=0 in 3D
    /// (the axis of revolution). Since the path has radius=capsule_radius,
    /// profile X values are offset by -radius.
    ///
    /// # Arguments
    /// * `radius` - The radius of the hemispheres
    /// * `half_height` - Half the height of the cylindrical body
    fn create_capsule_profile_offset(radius: f64, half_height: f64) -> PlanarProfile {
        // Profile coordinates in sweep model:
        // - Profile X maps to radial direction from path center
        // - Profile Y maps to perpendicular direction (Y-axis in 3D)
        //
        // Since path has radius R, and we want:
        // - Capsule axis at world origin (radial=0)
        // - Capsule surface at radial=R
        //
        // We need profile X to go from -R (at axis) to 0 (at surface)
        // This is because final radial = path_radius + profile_x = R + (-R to 0) = 0 to R

        let bottom_pole = DVec2::new(-radius, -half_height - radius);
        let bottom_equator = DVec2::new(0.0, -half_height);
        let top_equator = DVec2::new(0.0, half_height);
        let top_pole = DVec2::new(-radius, half_height + radius);

        // Centers for the quarter circles (on the axis, which is at X=-radius in profile space)
        let bottom_center = DVec2::new(-radius, -half_height);
        let top_center = DVec2::new(-radius, half_height);

        PlanarProfile::with_commands(vec![
            // Start at bottom pole (on the axis)
            ProfileCommand::new(PathCommand::Move { point: bottom_pole }, Corner::Edge),
            // Bottom quarter circle (from bottom pole to bottom equator/surface)
            ProfileCommand::new(
                PathCommand::Ellipse {
                    point: bottom_equator,
                    center: bottom_center,
                },
                Corner::SmoothEdge,
            ),
            // Line up the cylinder body (along the surface)
            ProfileCommand::new(PathCommand::Line { point: top_equator }, Corner::SmoothEdge),
            // Top quarter circle (from top equator/surface to top pole on axis)
            ProfileCommand::new(
                PathCommand::Ellipse {
                    point: top_pole,
                    center: top_center,
                },
                Corner::Edge,
            ),
        ])
    }
}

impl GeometryModel for Capsule {
    /// Converts the capsule to a polygon mesh.
    ///
    /// This method creates a `SweepModel` from the capsule parameters and
    /// delegates to the sweep model's `to_polygon_mesh` implementation.
    /// After mesh generation, the geometry IDs are remapped to support
    /// 3 material regions: bottom hemisphere (0), body (1), top hemisphere (2).
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the capsule,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The radius is invalid (zero or negative)
    /// - The height is negative
    /// - The segment count is less than 3
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        // Validate parameters
        if self.radius <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "radius".to_string(),
                message: "Radius must be positive".to_string(),
            });
        }
        if self.height < 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "height".to_string(),
                message: "Height must be non-negative".to_string(),
            });
        }
        if self.segments < 3 {
            return Err(GeometryError::InvalidParameter {
                parameter: "segments".to_string(),
                message: "Segments must be at least 3".to_string(),
            });
        }

        let sweep_model = self.to_sweep_model();
        let mut mesh = sweep_model.to_polygon_mesh()?;

        // Remap geometry IDs based on profile segment:
        // Segment 0 (bottom quarter circle) -> geometry_id 0
        // Segment 1 (cylinder body line) -> geometry_id 1
        // Segment 2 (top quarter circle) -> geometry_id 2
        for polygon in &mut mesh.polygons {
            // The sweep model assigns geometry_id based on profile segment
            // We need to map these to our 3 regions
            let segment_id = polygon.geometry_id;
            polygon.geometry_id = match segment_id {
                0 => 0, // Bottom hemisphere
                1 => 1, // Body
                2 => 2, // Top hemisphere
                _ => segment_id % 3,
            };
        }

        // Update material map for the 3 regions
        mesh.material_map.clear();
        for face_index in 0..3 {
            let material_name = self.material.get_material_for_face(face_index, 3);
            mesh.set_material(face_index, material_name);
        }

        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Torus construction tests
    // ============================================================================

    #[test]
    fn test_torus_new() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);

        assert_eq!(torus.major_radius, 2.0);
        assert_eq!(torus.minor_radius, 0.5);
        assert_eq!(torus.major_segments, 36);
        assert_eq!(torus.minor_segments, 18);
        assert_eq!(torus.material, Material::Default);
    }

    #[test]
    fn test_torus_with_uniform_material() {
        let torus = Torus::new(3.0, 1.0, 24, 12, Material::Uniform("gold".to_string()));

        assert_eq!(torus.major_radius, 3.0);
        assert_eq!(torus.minor_radius, 1.0);
        if let Material::Uniform(name) = &torus.material {
            assert_eq!(name, "gold");
        } else {
            panic!("Expected Material::Uniform");
        }
    }

    #[test]
    fn test_torus_clone() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);
        let cloned = torus.clone();

        assert_eq!(torus, cloned);
    }

    // ============================================================================
    // Serialization tests
    // ============================================================================

    #[test]
    fn test_torus_serialize_deserialize() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);

        let json = serde_json::to_string(&torus).expect("Failed to serialize");
        let deserialized: Torus = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(torus, deserialized);
    }

    #[test]
    fn test_torus_serialize_with_uniform_material() {
        let torus = Torus::new(2.5, 0.75, 32, 16, Material::Uniform("copper".to_string()));

        let json = serde_json::to_string(&torus).expect("Failed to serialize");
        let deserialized: Torus = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(torus, deserialized);
    }

    #[test]
    fn test_torus_json_format() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);
        let json = serde_json::to_string_pretty(&torus).expect("Failed to serialize");

        assert!(json.contains("major_radius"));
        assert!(json.contains("minor_radius"));
        assert!(json.contains("major_segments"));
        assert!(json.contains("minor_segments"));
        assert!(json.contains("material"));
    }

    // ============================================================================
    // Circle profile creation tests
    // ============================================================================

    #[test]
    fn test_create_circle_profile() {
        let profile = Torus::create_circle_profile(1.0);

        assert_eq!(profile.len(), 2);
    }

    #[test]
    fn test_create_circle_profile_different_radius() {
        let profile = Torus::create_circle_profile(2.5);

        // Verify the start point matches the radius
        if let PathCommand::Move { point } = &profile.commands[0].path {
            assert!((point.x - 2.5).abs() < 1e-10);
            assert!(point.y.abs() < 1e-10);
        } else {
            panic!("Expected Move command");
        }
    }

    // ============================================================================
    // SampleContour implementation tests
    // ============================================================================

    #[test]
    fn test_sample_contour_basic() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok(), "Error: {:?}", result.err());

        let contour_set = result.unwrap();
        assert!(!contour_set.is_empty());
    }

    #[test]
    fn test_sample_contour_returns_closed_contour() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
    }

    #[test]
    fn test_sample_contour_segment_count() {
        let torus = Torus::new(2.0, 0.5, 12, 8, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        // The path is sampled, so we should have segments based on the major circle
        assert!(contour_set.contours[0].len() >= 12);
    }

    // ============================================================================
    // Error handling tests
    // ============================================================================

    #[test]
    fn test_sample_contour_zero_major_radius() {
        let torus = Torus::new(0.0, 0.5, 36, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "major_radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_sample_contour_negative_major_radius() {
        let torus = Torus::new(-1.0, 0.5, 36, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "major_radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_sample_contour_zero_minor_radius() {
        let torus = Torus::new(2.0, 0.0, 36, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "minor_radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_sample_contour_negative_minor_radius() {
        let torus = Torus::new(2.0, -0.5, 36, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "minor_radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_sample_contour_zero_major_segments() {
        let torus = Torus::new(2.0, 0.5, 0, 18, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "major_segments");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_sample_contour_zero_minor_segments() {
        let torus = Torus::new(2.0, 0.5, 36, 0, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "minor_segments");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    // ============================================================================
    // Sweep model creation tests
    // ============================================================================

    #[test]
    fn test_to_sweep_model() {
        let torus = Torus::new(2.0, 0.5, 36, 18, Material::Default);
        let sweep_model = torus.to_sweep_model();

        // Verify the sample distances are calculated correctly
        let expected_path_distance = 2.0 * PI * 2.0 / 36.0;
        let expected_profile_distance = 2.0 * PI * 0.5 / 18.0;

        assert!((sweep_model.path_sample_distance - expected_path_distance).abs() < 1e-10);
        assert!((sweep_model.profile_sample_distance - expected_profile_distance).abs() < 1e-10);
    }

    #[test]
    fn test_to_sweep_model_material_preserved() {
        let torus = Torus::new(
            2.0,
            0.5,
            36,
            18,
            Material::Uniform("test_material".to_string()),
        );
        let sweep_model = torus.to_sweep_model();

        assert_eq!(
            sweep_model.material,
            Material::Uniform("test_material".to_string())
        );
    }

    // ============================================================================
    // Integration tests
    // ============================================================================

    #[test]
    fn test_torus_with_low_segment_counts() {
        // Test with minimum reasonable segment counts
        let torus = Torus::new(2.0, 0.5, 3, 3, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_torus_with_high_segment_counts() {
        // Test with high segment counts
        let torus = Torus::new(2.0, 0.5, 72, 36, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert!(contour_set.contours[0].len() >= 72);
    }

    #[test]
    fn test_torus_thin_ring() {
        // Test a torus where minor radius is much smaller than major radius
        let torus = Torus::new(10.0, 0.1, 48, 12, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_torus_fat_ring() {
        // Test a torus where minor radius approaches major radius
        let torus = Torus::new(1.0, 0.9, 24, 24, Material::Default);

        let result = torus.sample_contour(0.1);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Torus mesh and normal tests
    // ============================================================================

    #[test]
    fn test_torus_to_polygon_mesh() {
        let torus = Torus::new(2.0, 0.5, 12, 8, Material::Uniform("test".to_string()));

        let result = torus.to_polygon_mesh();
        assert!(result.is_ok(), "Error: {:?}", result.err());

        let mesh = result.unwrap();
        assert!(!mesh.is_empty());
        assert!(mesh.polygon_count() > 0);
        assert!(mesh.vertex_count() > 0);
    }

    #[test]
    fn test_torus_normals_point_outward() {
        use glam::DVec3;

        // Create a torus centered at origin
        let major_radius = 2.0;
        let minor_radius = 0.5;
        let torus = Torus::new(
            major_radius,
            minor_radius,
            12,
            8,
            Material::Uniform("test".to_string()),
        );

        let mesh = torus.to_polygon_mesh().expect("Failed to create mesh");

        // For each vertex, check that the normal points outward from the torus surface
        for vertex in &mesh.vertices {
            let pos = vertex.position;
            let normal = vertex.normal;

            // The torus is in the XY plane, centered at origin
            // For each point, find the closest point on the major circle (the tube center)
            let pos_xy = DVec3::new(pos.x, pos.y, 0.0);
            let pos_xy_len = pos_xy.length();

            if pos_xy_len > 1e-10 {
                // Direction from origin to the point projected onto XY plane
                let to_tube_center = pos_xy.normalize() * major_radius;

                // Vector from tube center to the vertex (should be along minor radius direction)
                let from_tube_center = pos - to_tube_center;
                let from_tube_center_normalized = from_tube_center.normalize();

                // The normal should point in the same general direction as from_tube_center
                // (i.e., outward from the tube surface)
                let dot = normal.dot(from_tube_center_normalized);

                assert!(
                    dot > 0.5, // Allow some tolerance for interpolation
                    "Normal at position {:?} should point outward. Normal: {:?}, expected direction: {:?}, dot: {}",
                    pos,
                    normal,
                    from_tube_center_normalized,
                    dot
                );
            }
        }
    }

    #[test]
    fn test_torus_normals_are_normalized() {
        let torus = Torus::new(2.0, 0.5, 12, 8, Material::Uniform("test".to_string()));
        let mesh = torus.to_polygon_mesh().expect("Failed to create mesh");

        for vertex in &mesh.vertices {
            let normal = vertex.normal;
            let length = normal.length();

            assert!(
                (length - 1.0).abs() < 1e-6,
                "Normal should be normalized, but length is {}",
                length
            );
        }
    }

    #[test]
    fn test_torus_outer_edge_normals_point_away_from_center() {
        use glam::DVec3;

        // Create a torus and check that normals on the outer edge point away from the Z axis
        let major_radius = 2.0;
        let minor_radius = 0.5;
        let torus = Torus::new(
            major_radius,
            minor_radius,
            12,
            8,
            Material::Uniform("test".to_string()),
        );

        let mesh = torus.to_polygon_mesh().expect("Failed to create mesh");

        // Find vertices on the outer edge (distance from Z axis > major_radius)
        let outer_threshold = major_radius + minor_radius * 0.5;

        for vertex in &mesh.vertices {
            let pos = vertex.position;
            let normal = vertex.normal;

            // Distance from Z axis in the XY plane
            let dist_from_z_axis = (pos.x * pos.x + pos.y * pos.y).sqrt();

            if dist_from_z_axis > outer_threshold {
                // For outer edge vertices, the normal's XY component should point away from Z axis
                let normal_xy = DVec3::new(normal.x, normal.y, 0.0);
                let pos_xy = DVec3::new(pos.x, pos.y, 0.0).normalize();

                if normal_xy.length() > 0.1 {
                    let dot = normal_xy.normalize().dot(pos_xy);
                    assert!(
                        dot > 0.0,
                        "Outer edge normal XY component should point away from center. Position: {:?}, Normal: {:?}, dot: {}",
                        pos,
                        normal,
                        dot
                    );
                }
            }
        }
    }

    // ============================================================================
    // Capsule construction tests
    // ============================================================================

    #[test]
    fn test_capsule_new() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Default);

        assert_eq!(capsule.radius, 0.5);
        assert_eq!(capsule.height, 2.0);
        assert_eq!(capsule.segments, 24);
        assert_eq!(capsule.material, Material::Default);
    }

    #[test]
    fn test_capsule_with_uniform_material() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Uniform("gold".to_string()));

        assert_eq!(capsule.radius, 0.5);
        assert_eq!(capsule.height, 2.0);
        if let Material::Uniform(name) = &capsule.material {
            assert_eq!(name, "gold");
        } else {
            panic!("Expected Material::Uniform");
        }
    }

    #[test]
    fn test_capsule_clone() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Default);
        let cloned = capsule.clone();

        assert_eq!(capsule, cloned);
    }

    // ============================================================================
    // Capsule serialization tests
    // ============================================================================

    #[test]
    fn test_capsule_serialize_deserialize() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Default);

        let json = serde_json::to_string(&capsule).expect("Failed to serialize");
        let deserialized: Capsule = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(capsule, deserialized);
    }

    #[test]
    fn test_capsule_serialize_with_uniform_material() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Uniform("copper".to_string()));

        let json = serde_json::to_string(&capsule).expect("Failed to serialize");
        let deserialized: Capsule = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(capsule, deserialized);
    }

    #[test]
    fn test_capsule_json_format() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Default);
        let json = serde_json::to_string_pretty(&capsule).expect("Failed to serialize");

        assert!(json.contains("radius"));
        assert!(json.contains("height"));
        assert!(json.contains("segments"));
        assert!(json.contains("material"));
    }

    // ============================================================================
    // Capsule mesh generation tests
    // ============================================================================

    #[test]
    fn test_capsule_to_polygon_mesh() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Uniform("test".to_string()));

        let result = capsule.to_polygon_mesh();
        assert!(result.is_ok(), "Error: {:?}", result.err());

        let mesh = result.unwrap();
        assert!(!mesh.is_empty());
        assert!(mesh.polygon_count() > 0);
        assert!(mesh.vertex_count() > 0);
    }

    #[test]
    fn test_capsule_with_zero_height() {
        // A capsule with zero height is just a sphere
        let capsule = Capsule::new(0.5, 0.0, 12, Material::Default);

        let result = capsule.to_polygon_mesh();
        assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    #[test]
    fn test_capsule_has_three_material_regions() {
        let capsule = Capsule::new(
            0.5,
            2.0,
            12,
            Material::Array(vec![
                "bottom".to_string(),
                "body".to_string(),
                "top".to_string(),
            ]),
        );

        let mesh = capsule.to_polygon_mesh().expect("Failed to create mesh");

        // Check that we have 3 materials
        assert_eq!(mesh.material_map.len(), 3);
        assert_eq!(mesh.get_material(0), Some(&"bottom".to_string()));
        assert_eq!(mesh.get_material(1), Some(&"body".to_string()));
        assert_eq!(mesh.get_material(2), Some(&"top".to_string()));
    }

    #[test]
    fn test_capsule_default_material_creates_three_regions() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Default);

        let mesh = capsule.to_polygon_mesh().expect("Failed to create mesh");

        // Default material should create 3 unique materials
        assert_eq!(mesh.material_map.len(), 3);
        assert_eq!(mesh.get_material(0), Some(&"material_0".to_string()));
        assert_eq!(mesh.get_material(1), Some(&"material_1".to_string()));
        assert_eq!(mesh.get_material(2), Some(&"material_2".to_string()));
    }

    #[test]
    fn test_capsule_normals_are_normalized() {
        let capsule = Capsule::new(0.5, 2.0, 12, Material::Default);
        let mesh = capsule.to_polygon_mesh().expect("Failed to create mesh");

        for vertex in &mesh.vertices {
            let normal = vertex.normal;
            let length = normal.length();

            assert!(
                (length - 1.0).abs() < 1e-6,
                "Normal should be normalized, but length is {}",
                length
            );
        }
    }

    // ============================================================================
    // Capsule error handling tests
    // ============================================================================

    #[test]
    fn test_capsule_zero_radius() {
        let capsule = Capsule::new(0.0, 2.0, 24, Material::Default);

        let result = capsule.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_capsule_negative_radius() {
        let capsule = Capsule::new(-0.5, 2.0, 24, Material::Default);

        let result = capsule.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_capsule_negative_height() {
        let capsule = Capsule::new(0.5, -2.0, 24, Material::Default);

        let result = capsule.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "height");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_capsule_insufficient_segments() {
        let capsule = Capsule::new(0.5, 2.0, 2, Material::Default);

        let result = capsule.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "segments");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    // ============================================================================
    // Capsule sweep model tests
    // ============================================================================

    #[test]
    fn test_capsule_to_sweep_model() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Default);
        let sweep_model = capsule.to_sweep_model();

        // Verify the material is preserved
        assert_eq!(sweep_model.material, Material::Default);
    }

    #[test]
    fn test_capsule_to_sweep_model_material_preserved() {
        let capsule = Capsule::new(0.5, 2.0, 24, Material::Uniform("test_material".to_string()));
        let sweep_model = capsule.to_sweep_model();

        assert_eq!(
            sweep_model.material,
            Material::Uniform("test_material".to_string())
        );
    }

    // ============================================================================
    // Capsule profile tests
    // ============================================================================

    #[test]
    fn test_capsule_profile_has_four_commands() {
        let profile = Capsule::create_capsule_profile_offset(0.5, 1.0);

        // Profile should have: Move, Ellipse (bottom), Line, Ellipse (top)
        assert_eq!(profile.len(), 4);
    }

    #[test]
    fn test_capsule_profile_zero_height() {
        // With zero height, we still have the profile but line has zero length
        let profile = Capsule::create_capsule_profile_offset(0.5, 0.0);

        assert_eq!(profile.len(), 4);
    }
}
