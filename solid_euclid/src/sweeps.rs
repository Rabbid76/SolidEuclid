//! Sweep-based geometric primitives.
//!
//! This module provides geometric shapes that are defined by sweeping a profile along a path.

use crate::geometry_error::GeometryError;
use crate::material::Material;
use crate::planar_contour::{ContourSet, SampleContour};
use crate::planar_profile::{Corner, PathCommand, PlanarProfile, ProfileCommand};
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
    fn create_circle_profile(radius: f64) -> PlanarProfile {
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
}
