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

/// Gets the effective radius/distance of an edge corner treatment.
///
/// # Arguments
/// * `corner` - The corner treatment to get the size of
///
/// # Returns
/// The radius for fillets, distance for chamfers, or 0.0 for edges
fn get_edge_radius(corner: &Corner) -> f64 {
    match corner {
        Corner::Edge | Corner::SmoothEdge => 0.0,
        Corner::Chamfer { distance } => *distance,
        Corner::Fillet { radius } => *radius,
    }
}

/// Applies the standard 5-region material mapping to a polygon mesh.
///
/// Maps segment IDs to: bottom cap (0), bottom edge (1), body (2), top edge (3), top cap (4).
///
/// # Arguments
/// * `mesh` - The mesh to update
/// * `material` - The material to apply
fn apply_frustum_material_mapping(mesh: &mut PolygonMesh, material: &Material) {
    for polygon in &mut mesh.polygons {
        let segment_id = polygon.geometry_id;
        polygon.geometry_id = if segment_id >= 5 {
            segment_id % 5
        } else {
            segment_id
        };
    }

    mesh.material_map.clear();
    for face_index in 0..5 {
        let material_name = material.get_material_for_face(face_index, 5);
        mesh.set_material(face_index, material_name);
    }
}

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

/// Represents a cylinder defined by radius, height, and edge treatments.
///
/// A cylinder is created by sweeping a profile (horizontal line + vertical line + horizontal line)
/// along a circular path. The cylinder is oriented along the Y-axis, with the center
/// at the origin.
///
/// The cylinder supports 5 material regions:
/// - Face 0: Bottom cap
/// - Face 1: Bottom edge (chamfer or fillet)
/// - Face 2: Cylindrical body
/// - Face 3: Top edge (chamfer or fillet)
/// - Face 4: Top cap
///
/// # Example
/// ```
/// use solid_euclid::sweeps::Cylinder;
/// use solid_euclid::material::Material;
/// use solid_euclid::planar_profile::Corner;
///
/// let cylinder = Cylinder::new(
///     0.5,           // radius
///     2.0,           // height
///     24,            // segments
///     Corner::Edge,  // bottom_edge
///     Corner::Edge,  // top_edge
///     Material::Default,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cylinder {
    /// The radius of the cylinder.
    pub radius: f64,
    /// The total height of the cylinder.
    pub height: f64,
    /// The number of segments for tessellation around the circumference.
    pub segments: u32,
    /// The corner treatment at the bottom edge.
    pub bottom_edge: Corner,
    /// The corner treatment at the top edge.
    pub top_edge: Corner,
    /// The material assignment for the cylinder surfaces.
    pub material: Material,
}

impl Cylinder {
    /// Creates a new `Cylinder` with the given parameters.
    ///
    /// # Arguments
    /// * `radius` - The radius of the cylinder
    /// * `height` - The total height of the cylinder
    /// * `segments` - The number of segments for tessellation
    /// * `bottom_edge` - The corner treatment at the bottom edge
    /// * `top_edge` - The corner treatment at the top edge
    /// * `material` - The material assignment for the surfaces
    ///
    /// # Panics
    /// This function does not panic, but invalid parameters will result in
    /// errors when generating the mesh.
    pub fn new(
        radius: f64,
        height: f64,
        segments: u32,
        bottom_edge: Corner,
        top_edge: Corner,
        material: Material,
    ) -> Self {
        Self {
            radius,
            height,
            segments,
            bottom_edge,
            top_edge,
            material,
        }
    }

    /// Converts this cylinder to an equivalent `ConicalFrustum`.
    ///
    /// A cylinder is a special case of a conical frustum where both radii are equal.
    ///
    /// # Returns
    /// A `ConicalFrustum` with the same geometry as this cylinder
    pub fn to_conical_frustum(&self) -> ConicalFrustum {
        ConicalFrustum::new(
            self.radius,
            self.radius,
            self.height,
            self.segments,
            self.bottom_edge,
            self.top_edge,
            self.material.clone(),
        )
    }
}

impl GeometryModel for Cylinder {
    /// Converts the cylinder to a polygon mesh.
    ///
    /// This method delegates to `ConicalFrustum::to_polygon_mesh` since a cylinder
    /// is a special case of a conical frustum where both radii are equal.
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the cylinder,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The radius is invalid (zero or negative)
    /// - The height is invalid (zero or negative)
    /// - The segment count is less than 3
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        // Validate that radius is positive (cylinder-specific constraint)
        if self.radius <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "radius".to_string(),
                message: "Radius must be positive".to_string(),
            });
        }
        // Delegate to ConicalFrustum implementation
        self.to_conical_frustum().to_polygon_mesh()
    }
}

/// Represents a conical frustum (truncated cone) defined by bottom radius, top radius, height, and edge treatments.
///
/// A conical frustum is created by sweeping a profile (horizontal line + diagonal line + horizontal line)
/// along a circular path. The frustum is oriented along the Y-axis, with the center at the origin.
///
/// The conical frustum supports 5 material regions:
/// - Face 0: Bottom cap
/// - Face 1: Bottom edge (chamfer or fillet)
/// - Face 2: Conical body
/// - Face 3: Top edge (chamfer or fillet)
/// - Face 4: Top cap
///
/// # Example
/// ```
/// use solid_euclid::sweeps::ConicalFrustum;
/// use solid_euclid::material::Material;
/// use solid_euclid::planar_profile::Corner;
///
/// let frustum = ConicalFrustum::new(
///     0.5,           // bottom_radius
///     0.25,          // top_radius
///     2.0,           // height
///     24,            // segments
///     Corner::Edge,  // bottom_edge
///     Corner::Edge,  // top_edge
///     Material::Default,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConicalFrustum {
    /// The radius at the bottom of the frustum.
    pub bottom_radius: f64,
    /// The radius at the top of the frustum.
    pub top_radius: f64,
    /// The total height of the frustum.
    pub height: f64,
    /// The number of segments for tessellation around the circumference.
    pub segments: u32,
    /// The corner treatment at the bottom edge.
    pub bottom_edge: Corner,
    /// The corner treatment at the top edge.
    pub top_edge: Corner,
    /// The material assignment for the frustum surfaces.
    pub material: Material,
}

impl ConicalFrustum {
    /// Creates a new `ConicalFrustum` with the given parameters.
    ///
    /// # Arguments
    /// * `bottom_radius` - The radius at the bottom of the frustum
    /// * `top_radius` - The radius at the top of the frustum
    /// * `height` - The total height of the frustum
    /// * `segments` - The number of segments for tessellation
    /// * `bottom_edge` - The corner treatment at the bottom edge
    /// * `top_edge` - The corner treatment at the top edge
    /// * `material` - The material assignment for the surfaces
    ///
    /// # Panics
    /// This function does not panic, but invalid parameters will result in
    /// errors when generating the mesh.
    pub fn new(
        bottom_radius: f64,
        top_radius: f64,
        height: f64,
        segments: u32,
        bottom_edge: Corner,
        top_edge: Corner,
        material: Material,
    ) -> Self {
        Self {
            bottom_radius,
            top_radius,
            height,
            segments,
            bottom_edge,
            top_edge,
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

    /// Creates a `SweepModel` from this conical frustum.
    ///
    /// The sweep model uses:
    /// - A circular path (the revolution around Y-axis) using the larger radius
    /// - A frustum profile (horizontal line at bottom + diagonal line + horizontal line at top)
    /// - Sample distances calculated from the geometry and segment count
    ///
    /// # Returns
    /// A `SweepModel` that can be used to generate the frustum geometry
    pub(crate) fn to_sweep_model(&self) -> SweepModel<PlanarProfile, PlanarProfile> {
        let half_height = self.height / 2.0;
        let max_radius = self.bottom_radius.max(self.top_radius);

        // The path creates the revolution around the Y-axis
        // Use the larger radius as the path radius
        let path = Self::create_circle_path(max_radius);

        // Profile is the frustum cross-section with edge treatments
        let profile = self.create_frustum_profile(half_height, max_radius);

        // Calculate sample distances
        let path_circumference = 2.0 * PI * max_radius;
        let path_sample_distance = path_circumference / self.segments as f64;

        // Profile length depends on edges and the diagonal
        let bottom_edge_size = get_edge_radius(&self.bottom_edge);
        let top_edge_size = get_edge_radius(&self.top_edge);
        let body_height = self.height - bottom_edge_size - top_edge_size;
        let radius_diff = (self.bottom_radius - self.top_radius).abs();
        let diagonal_length = (body_height.powi(2) + radius_diff.powi(2)).sqrt();
        let profile_length = self.bottom_radius
            + self.top_radius
            + diagonal_length.max(0.0)
            + bottom_edge_size
            + top_edge_size;
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

    /// Creates the frustum profile with edge treatments.
    ///
    /// The profile is:
    /// - Horizontal line from axis to edge at bottom (bottom cap)
    /// - Bottom edge corner treatment
    /// - Diagonal line up the body (from bottom_radius to top_radius)
    /// - Top edge corner treatment
    /// - Horizontal line from edge to axis at top (top cap)
    ///
    /// # Arguments
    /// * `half_height` - Half the total height of the frustum
    /// * `max_radius` - The larger of the two radii (used as path radius offset)
    pub(crate) fn create_frustum_profile(
        &self,
        half_height: f64,
        max_radius: f64,
    ) -> PlanarProfile {
        // Profile coordinates in sweep model:
        // - Profile X maps to radial direction from path center
        // - Profile Y maps to perpendicular direction (Y-axis in 3D)
        //
        // Since path has radius max_radius, and we want the bottom surface at radial = bottom_radius,
        // we need profile X to go from -(max_radius) (at axis) to -(max_radius - bottom_radius) (at bottom surface)
        // and similarly for top

        let bottom_x = -(max_radius - self.bottom_radius); // surface at bottom (offset from path)
        let top_x = -(max_radius - self.top_radius); // surface at top (offset from path)

        // The 4 corner points of the frustum cross-section
        let bottom_center = DVec2::new(-max_radius, -half_height); // axis at bottom
        let bottom_surface = DVec2::new(bottom_x, -half_height); // surface at bottom
        let top_surface = DVec2::new(top_x, half_height); // surface at top
        let top_center = DVec2::new(-max_radius, half_height); // axis at top

        // Simple 4-command profile:
        // Move to bottom center, then 3 lines forming the cross-section
        // The corner treatments (Edge/Chamfer/Fillet) are specified on the Line commands
        // and will be processed by PlanarProfile::to_curve_set()
        PlanarProfile::with_commands(vec![
            // Start at bottom center (on the axis)
            ProfileCommand::new(
                PathCommand::Move {
                    point: bottom_center,
                },
                Corner::Edge,
            ),
            // Bottom cap: line from axis to surface, with bottom_edge corner treatment
            ProfileCommand::new(
                PathCommand::Line {
                    point: bottom_surface,
                },
                self.bottom_edge,
            ),
            // Body: diagonal line up the surface, with top_edge corner treatment
            ProfileCommand::new(PathCommand::Line { point: top_surface }, self.top_edge),
            // Top cap: line from surface back to axis
            ProfileCommand::new(PathCommand::Line { point: top_center }, Corner::Edge),
        ])
    }
}

impl GeometryModel for ConicalFrustum {
    /// Converts the conical frustum to a polygon mesh.
    ///
    /// This method creates a `SweepModel` from the frustum parameters and
    /// delegates to the sweep model's `to_polygon_mesh` implementation.
    /// After mesh generation, the geometry IDs are remapped to support
    /// 5 material regions: bottom cap (0), bottom edge (1), body (2), top edge (3), top cap (4).
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the frustum,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Either radius is invalid (negative)
    /// - Both radii are zero
    /// - The height is invalid (zero or negative)
    /// - The segment count is less than 3
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        // Validate parameters
        if self.bottom_radius < 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "bottom_radius".to_string(),
                message: "Bottom radius must be non-negative".to_string(),
            });
        }
        if self.top_radius < 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "top_radius".to_string(),
                message: "Top radius must be non-negative".to_string(),
            });
        }
        if self.bottom_radius == 0.0 && self.top_radius == 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "radii".to_string(),
                message: "At least one radius must be positive".to_string(),
            });
        }
        if self.height <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "height".to_string(),
                message: "Height must be positive".to_string(),
            });
        }
        if self.segments < 3 {
            return Err(GeometryError::InvalidParameter {
                parameter: "segments".to_string(),
                message: "Segments must be at least 3".to_string(),
            });
        }

        // Validate edge sizes don't exceed half the height
        let bottom_edge_size = get_edge_radius(&self.bottom_edge);
        let top_edge_size = get_edge_radius(&self.top_edge);
        if bottom_edge_size + top_edge_size > self.height {
            return Err(GeometryError::InvalidParameter {
                parameter: "edges".to_string(),
                message: "Combined edge sizes cannot exceed frustum height".to_string(),
            });
        }

        let sweep_model = self.to_sweep_model();
        let mut mesh = sweep_model.to_polygon_mesh()?;

        // Apply the standard 5-region material mapping
        apply_frustum_material_mapping(&mut mesh, &self.material);

        Ok(mesh)
    }
}

/// Represents an extruded shape created by sweeping a 2D profile along a linear path.
///
/// An extrude is created by taking a 2D planar profile and extruding it vertically
/// along the Y-axis. The extrusion goes from -height/2 to +height/2, centered at the origin.
///
/// The extrude supports separate materials for:
/// - Bottom cap (geometry_id based on profile segment IDs with cap_bottom offset)
/// - Top cap (geometry_id based on profile segment IDs with cap_top offset)
/// - Side faces for each profile segment (geometry_id = profile_segment_id)
///
/// Profile segments (edges and corners) each get their own geometry_id, allowing
/// separate materials to be applied to different parts of the extruded shape.
///
/// # Example
/// ```
/// use solid_euclid::sweeps::Extrude;
/// use solid_euclid::planar_profile::{PlanarProfile, ProfileCommand, PathCommand, Corner};
/// use solid_euclid::material::Material;
/// use glam::DVec2;
///
/// // Create a simple square profile
/// let profile = PlanarProfile::with_commands(vec![
///     ProfileCommand::new(PathCommand::Move { point: DVec2::new(0.0, 0.0) }, Corner::Edge),
///     ProfileCommand::new(PathCommand::Line { point: DVec2::new(1.0, 0.0) }, Corner::Edge),
///     ProfileCommand::new(PathCommand::Line { point: DVec2::new(1.0, 1.0) }, Corner::Edge),
///     ProfileCommand::new(PathCommand::Line { point: DVec2::new(0.0, 1.0) }, Corner::Edge),
///     ProfileCommand::new(PathCommand::Line { point: DVec2::new(0.0, 0.0) }, Corner::Edge),
/// ]);
///
/// let extrude = Extrude::new(
///     profile,
///     2.0,   // height
///     0.1,   // sample_distance
///     Material::Default,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extrude {
    /// The 2D profile to extrude.
    pub profile: PlanarProfile,
    /// The total height of the extrusion (from -height/2 to +height/2).
    pub height: f64,
    /// The sample distance for sampling the profile curves.
    pub sample_distance: f64,
    /// The material assignment for the extrude surfaces.
    /// - For uniform material: applies to all faces
    /// - For array material: [body_material, bottom_cap_material, top_cap_material]
    /// - For map material: profile segment IDs for sides, -1 for bottom cap, -2 for top cap
    pub material: Material,
}

/// Geometry ID constants for extrude caps
const EXTRUDE_BOTTOM_CAP_ID: i32 = -1;
const EXTRUDE_TOP_CAP_ID: i32 = -2;

impl Extrude {
    /// Creates a new `Extrude` with the given parameters.
    ///
    /// # Arguments
    /// * `profile` - The 2D profile to extrude
    /// * `height` - The total height of the extrusion
    /// * `sample_distance` - The sample distance for profile curves
    /// * `material` - The material assignment for the surfaces
    ///
    /// # Panics
    /// This function does not panic, but invalid parameters will result in
    /// errors when generating the mesh.
    pub fn new(
        profile: PlanarProfile,
        height: f64,
        sample_distance: f64,
        material: Material,
    ) -> Self {
        Self {
            profile,
            height,
            sample_distance,
            material,
        }
    }

    /// Creates the linear extrusion path.
    ///
    /// The path is a simple vertical line from (0, -height/2) to (0, +height/2).
    /// This represents the Y-axis direction for the extrusion.
    fn create_extrusion_path(&self) -> PlanarProfile {
        let half_height = self.height / 2.0;
        PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, -half_height),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, half_height),
                },
                Corner::Edge,
            ),
        ])
    }

    /// Creates a `SweepModel` from this extrude using a pre-sampled profile contour.
    ///
    /// The sweep model uses:
    /// - A linear vertical path from -height/2 to +height/2
    /// - The pre-sampled profile contour as the swept cross-section
    ///
    /// # Arguments
    /// * `profile_contour` - The pre-sampled profile contour
    ///
    /// # Returns
    /// A `SweepModel` that can be used to generate the extrude side geometry
    fn create_sweep_model_with_contour(
        &self,
        profile_contour: ContourSet,
    ) -> SweepModel<PlanarProfile, ContourSet> {
        let path = self.create_extrusion_path();

        // Path sample distance should produce a single step for the linear extrusion
        // Use height * 2 to ensure we only get start and end points
        let path_sample_distance = self.height * 2.0;

        // Profile sample distance is ignored since ContourSet returns itself
        SweepModel::new(
            path,
            profile_contour,
            path_sample_distance,
            self.sample_distance, // This will be ignored by ContourSet::sample_contour
            self.material.clone(),
        )
    }

    /// Generates a cap mesh from the pre-sampled profile contour.
    ///
    /// # Arguments
    /// * `contour_set` - The pre-sampled profile contour
    /// * `y_position` - The Y coordinate for the cap (negative for bottom, positive for top)
    /// * `is_bottom` - Whether this is the bottom cap (affects normal direction and winding)
    ///
    /// # Returns
    /// A `PolygonMesh` representing the cap
    fn generate_cap(
        contour_set: &ContourSet,
        y_position: f64,
        is_bottom: bool,
    ) -> Result<PolygonMesh, GeometryError> {
        use glam::DVec3;

        let mut cap_mesh = contour_set.to_polygon_mesh()?;

        let normal_y = if is_bottom { -1.0 } else { 1.0 };
        let normal = DVec3::new(0.0, normal_y, 0.0);
        let geometry_id = if is_bottom {
            EXTRUDE_BOTTOM_CAP_ID
        } else {
            EXTRUDE_TOP_CAP_ID
        };

        // Transform vertices: the ContourSet mesh is in XY plane at Z=0
        // Profile's X stays as X, Y becomes Z, and the cap is placed at y_position
        // UV coordinates need different handling for top vs bottom cap:
        // - Top cap (viewed from +Y): flip V to match texture orientation
        // - Bottom cap (viewed from -Y): keep V as-is for correct orientation
        for vertex in &mut cap_mesh.vertices {
            let x = vertex.position.x;
            let z = vertex.position.y; // Profile's Y becomes Z
            vertex.position = DVec3::new(x, y_position, z);
            vertex.normal = normal;
            // Only flip V coordinate for top cap
            if !is_bottom {
                vertex.uv.y = -vertex.uv.y;
            }
        }

        // Update polygon normals and geometry IDs
        // Reverse winding for top cap (normals point up)
        // The ContourSet mesh is CCW when viewed from +Z. After transform (Y->Z),
        // looking from +Y down, we need to reverse to get correct CCW winding.
        // For bottom cap (looking from -Y up), the original order is correct.
        for polygon in &mut cap_mesh.polygons {
            if !is_bottom {
                polygon.indices.reverse();
            }
            polygon.normal = normal;
            polygon.geometry_id = geometry_id;
        }

        Ok(cap_mesh)
    }

    /// Merges another polygon mesh into the target mesh, offsetting vertex indices.
    fn merge_mesh(target: &mut PolygonMesh, source: &PolygonMesh) {
        let vertex_offset = target.vertices.len() as i32;

        // Add all vertices from source
        target.vertices.extend(source.vertices.clone());

        // Add all polygons with offset indices
        for polygon in &source.polygons {
            let offset_indices: Vec<i32> =
                polygon.indices.iter().map(|&i| i + vertex_offset).collect();
            target.polygons.push(crate::polygon_mesh::Polygon::new(
                offset_indices,
                polygon.normal,
                polygon.geometry_id,
            ));
        }

        // Merge material maps
        for (geometry_id, material_name) in &source.material_map {
            target
                .material_map
                .insert(*geometry_id, material_name.clone());
        }
    }
}

impl GeometryModel for Extrude {
    /// Converts the extrude to a polygon mesh.
    ///
    /// This method creates:
    /// 1. Side faces using a `SweepModel` (profile extruded along a linear path)
    /// 2. Bottom cap at Y = -height/2
    /// 3. Top cap at Y = +height/2
    ///
    /// The profile is sampled once and reused for both the sweep model and the caps.
    ///
    /// The geometry IDs are assigned as:
    /// - Side faces: profile segment ID (0, 1, 2, etc. for each profile segment)
    /// - Bottom cap: EXTRUDE_BOTTOM_CAP_ID (-1)
    /// - Top cap: EXTRUDE_TOP_CAP_ID (-2)
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the extrude,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The height is invalid (zero or negative)
    /// - The sample distance is invalid (zero or negative)
    /// - The profile is invalid or cannot be sampled
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        // Validate parameters
        if self.height <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "height".to_string(),
                message: "Height must be positive".to_string(),
            });
        }
        if self.sample_distance <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "sample_distance".to_string(),
                message: "Sample distance must be positive".to_string(),
            });
        }
        if self.profile.is_empty() {
            return Err(GeometryError::InvalidParameter {
                parameter: "profile".to_string(),
                message: "Profile cannot be empty".to_string(),
            });
        }

        // Sample the profile once and reuse for sweep model and caps
        let profile_contour = self.profile.sample_contour(self.sample_distance)?;

        // Generate side faces using sweep model with pre-sampled contour
        let sweep_model = self.create_sweep_model_with_contour(profile_contour.clone());
        let mut mesh = sweep_model.to_polygon_mesh()?;

        // Remap geometry IDs for side faces: extract just the profile segment ID
        // The sweep model encodes: (path_segment << 16) | profile_segment
        // Since our path has only 1 segment (id=0), we just need the profile segment
        for polygon in &mut mesh.polygons {
            let profile_segment = polygon.geometry_id & 0xFFFF;
            polygon.geometry_id = profile_segment;
        }

        let half_height = self.height / 2.0;

        // Generate and merge bottom cap using the same sampled contour
        let bottom_cap = Self::generate_cap(&profile_contour, -half_height, true)?;
        Self::merge_mesh(&mut mesh, &bottom_cap);

        // Generate and merge top cap using the same sampled contour
        let top_cap = Self::generate_cap(&profile_contour, half_height, false)?;
        Self::merge_mesh(&mut mesh, &top_cap);

        // Apply material mapping
        // Get all unique geometry IDs from the mesh
        let mut geometry_ids: Vec<i32> = mesh.polygons.iter().map(|p| p.geometry_id).collect();
        geometry_ids.sort();
        geometry_ids.dedup();

        let total_faces = geometry_ids.len() as i32;
        for geometry_id in geometry_ids {
            let material_name = self
                .material
                .get_material_for_face(geometry_id, total_faces);
            mesh.set_material(geometry_id, material_name);
        }

        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

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

    // ============================================================================
    // Cylinder construction tests
    // ============================================================================

    #[test]
    fn test_cylinder_new() {
        let cylinder = Cylinder::new(0.5, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);

        assert_eq!(cylinder.radius, 0.5);
        assert_eq!(cylinder.height, 2.0);
        assert_eq!(cylinder.segments, 24);
        assert_eq!(cylinder.bottom_edge, Corner::Edge);
        assert_eq!(cylinder.top_edge, Corner::Edge);
        assert_eq!(cylinder.material, Material::Default);
    }

    #[test]
    fn test_cylinder_with_uniform_material() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("metal".to_string()),
        );

        if let Material::Uniform(name) = &cylinder.material {
            assert_eq!(name, "metal");
        } else {
            panic!("Expected Material::Uniform");
        }
    }

    #[test]
    fn test_cylinder_with_chamfer_edges() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );

        assert_eq!(cylinder.bottom_edge, Corner::Chamfer { distance: 0.1 });
        assert_eq!(cylinder.top_edge, Corner::Chamfer { distance: 0.1 });
    }

    #[test]
    fn test_cylinder_with_fillet_edges() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );

        assert_eq!(cylinder.bottom_edge, Corner::Fillet { radius: 0.1 });
        assert_eq!(cylinder.top_edge, Corner::Fillet { radius: 0.1 });
    }

    #[test]
    fn test_cylinder_with_mixed_edges() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Fillet { radius: 0.15 },
            Material::Default,
        );

        assert_eq!(cylinder.bottom_edge, Corner::Chamfer { distance: 0.1 });
        assert_eq!(cylinder.top_edge, Corner::Fillet { radius: 0.15 });
    }

    #[test]
    fn test_cylinder_clone() {
        let cylinder = Cylinder::new(0.5, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);
        let cloned = cylinder.clone();

        assert_eq!(cylinder, cloned);
    }

    // ============================================================================
    // Cylinder serialization tests
    // ============================================================================

    #[test]
    fn test_cylinder_serialize_deserialize() {
        let cylinder = Cylinder::new(0.5, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);
        let json = serde_json::to_string(&cylinder).expect("Failed to serialize");
        let deserialized: Cylinder = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(cylinder, deserialized);
    }

    #[test]
    fn test_cylinder_serialize_with_uniform_material() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("test".to_string()),
        );
        let json = serde_json::to_string(&cylinder).expect("Failed to serialize");
        let deserialized: Cylinder = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(cylinder, deserialized);
    }

    #[test]
    fn test_cylinder_serialize_with_chamfer() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Edge,
            Material::Default,
        );
        let json = serde_json::to_string(&cylinder).expect("Failed to serialize");
        let deserialized: Cylinder = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(cylinder, deserialized);
    }

    #[test]
    fn test_cylinder_serialize_with_fillet() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.15 },
            Material::Default,
        );
        let json = serde_json::to_string(&cylinder).expect("Failed to serialize");
        let deserialized: Cylinder = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(cylinder, deserialized);
    }

    // ============================================================================
    // Cylinder mesh generation tests
    // ============================================================================

    #[test]
    fn test_cylinder_to_polygon_mesh() {
        let cylinder = Cylinder::new(0.5, 2.0, 12, Corner::Edge, Corner::Edge, Material::Default);

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_ok(), "Error: {:?}", result.err());

        let mesh = result.unwrap();
        assert!(!mesh.is_empty());
        assert!(mesh.polygon_count() > 0);
        assert!(mesh.vertex_count() > 0);
    }

    #[test]
    fn test_cylinder_with_chamfer_to_polygon_mesh() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    #[test]
    fn test_cylinder_with_fillet_to_polygon_mesh() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_ok(), "Error: {:?}", result.err());
    }

    #[test]
    fn test_cylinder_has_five_material_regions() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Array(vec![
                "bottom_cap".to_string(),
                "bottom_edge".to_string(),
                "body".to_string(),
                "top_edge".to_string(),
                "top_cap".to_string(),
            ]),
        );

        let mesh = cylinder.to_polygon_mesh().expect("Failed to create mesh");

        // Check that we have 5 materials
        assert_eq!(mesh.get_material(0), Some(&"bottom_cap".to_string()));
        assert_eq!(mesh.get_material(1), Some(&"bottom_edge".to_string()));
        assert_eq!(mesh.get_material(2), Some(&"body".to_string()));
        assert_eq!(mesh.get_material(3), Some(&"top_edge".to_string()));
        assert_eq!(mesh.get_material(4), Some(&"top_cap".to_string()));
    }

    #[test]
    fn test_cylinder_no_edges_material_regions() {
        // Without edge treatments, we only have bottom_cap, body, top_cap
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Edge,
            Corner::Edge,
            Material::Array(vec![
                "bottom_cap".to_string(),
                "bottom_edge".to_string(),
                "body".to_string(),
                "top_edge".to_string(),
                "top_cap".to_string(),
            ]),
        );

        let mesh = cylinder.to_polygon_mesh().expect("Failed to create mesh");

        // Geometry IDs should be mapped to 0, 2, 4 (no edge regions when using Edge corners)
        let geometry_ids: std::collections::HashSet<_> =
            mesh.polygons.iter().map(|p| p.geometry_id).collect();

        // With Edge corners, we should have bottom cap (0), body (2), and top cap (4)
        assert!(
            geometry_ids.contains(&0),
            "Should have bottom cap (0), got: {:?}",
            geometry_ids
        );
        assert!(
            geometry_ids.contains(&4),
            "Should have top cap (4), got: {:?}",
            geometry_ids
        );
        // Body should be present - it maps to geometry_id 2
        assert!(
            geometry_ids.contains(&2),
            "Should have body (2), got: {:?}",
            geometry_ids
        );
    }

    #[test]
    fn test_cylinder_default_material_creates_five_regions() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );

        let mesh = cylinder.to_polygon_mesh().expect("Failed to create mesh");

        // Default material should create unique names for each face
        assert!(mesh.get_material(0).is_some());
        assert!(mesh.get_material(1).is_some());
        assert!(mesh.get_material(2).is_some());
        assert!(mesh.get_material(3).is_some());
        assert!(mesh.get_material(4).is_some());
    }

    #[test]
    fn test_cylinder_normals_are_normalized() {
        let cylinder = Cylinder::new(0.5, 2.0, 12, Corner::Edge, Corner::Edge, Material::Default);
        let mesh = cylinder.to_polygon_mesh().expect("Failed to create mesh");

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
    // Cylinder error handling tests
    // ============================================================================

    #[test]
    fn test_cylinder_zero_radius() {
        let cylinder = Cylinder::new(0.0, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_cylinder_negative_radius() {
        let cylinder = Cylinder::new(-0.5, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "radius");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_cylinder_zero_height() {
        let cylinder = Cylinder::new(0.5, 0.0, 24, Corner::Edge, Corner::Edge, Material::Default);

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "height");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_cylinder_negative_height() {
        let cylinder = Cylinder::new(0.5, -2.0, 24, Corner::Edge, Corner::Edge, Material::Default);

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "height");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_cylinder_insufficient_segments() {
        let cylinder = Cylinder::new(0.5, 2.0, 2, Corner::Edge, Corner::Edge, Material::Default);

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "segments");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_cylinder_edges_exceed_height() {
        let cylinder = Cylinder::new(
            0.5,
            1.0,
            24,
            Corner::Chamfer { distance: 0.6 },
            Corner::Chamfer { distance: 0.6 },
            Material::Default,
        );

        let result = cylinder.to_polygon_mesh();
        assert!(result.is_err());

        if let Err(GeometryError::InvalidParameter { parameter, .. }) = result {
            assert_eq!(parameter, "edges");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    // ============================================================================
    // Cylinder sweep model tests
    // ============================================================================

    #[test]
    fn test_cylinder_to_sweep_model() {
        let cylinder = Cylinder::new(0.5, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);
        // Cylinder delegates to ConicalFrustum internally
        let sweep_model = cylinder.to_conical_frustum().to_sweep_model();

        assert_eq!(sweep_model.material, Material::Default);
    }

    #[test]
    fn test_cylinder_to_conical_frustum_conversion() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Fillet { radius: 0.05 },
            Material::Uniform("test".to_string()),
        );
        let frustum = cylinder.to_conical_frustum();

        // Verify conversion produces correct values
        assert_eq!(frustum.bottom_radius, 0.5);
        assert_eq!(frustum.top_radius, 0.5); // Same as bottom for cylinder
        assert_eq!(frustum.height, 2.0);
        assert_eq!(frustum.segments, 24);
        assert_eq!(frustum.bottom_edge, Corner::Chamfer { distance: 0.1 });
        assert_eq!(frustum.top_edge, Corner::Fillet { radius: 0.05 });
        assert_eq!(frustum.material, Material::Uniform("test".to_string()));
    }

    #[test]
    fn test_cylinder_to_sweep_model_material_preserved() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("test_material".to_string()),
        );
        // Cylinder delegates to ConicalFrustum internally
        let sweep_model = cylinder.to_conical_frustum().to_sweep_model();

        assert_eq!(
            sweep_model.material,
            Material::Uniform("test_material".to_string())
        );
    }

    // ============================================================================
    // Cylinder profile tests
    // ============================================================================

    #[test]
    fn test_cylinder_profile_no_edges() {
        let cylinder = Cylinder::new(0.5, 2.0, 24, Corner::Edge, Corner::Edge, Material::Default);
        let frustum = cylinder.to_conical_frustum();
        let profile = frustum.create_frustum_profile(1.0, 0.5); // half_height = 1.0, max_radius = 0.5

        // Profile should have exactly 4 commands: Move, Line, Line, Line
        assert_eq!(profile.len(), 4);
    }

    #[test]
    fn test_cylinder_profile_with_chamfers() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );
        let frustum = cylinder.to_conical_frustum();
        let profile = frustum.create_frustum_profile(1.0, 0.5);

        // Profile should still have exactly 4 commands - corners are encoded in the Line commands
        assert_eq!(profile.len(), 4);

        // Verify the corner types
        assert!(matches!(profile.commands[1].corner, Corner::Chamfer { .. }));
        assert!(matches!(profile.commands[2].corner, Corner::Chamfer { .. }));
    }

    #[test]
    fn test_cylinder_get_edge_radius() {
        // Tests the module-level get_edge_radius function (shared by Cylinder and ConicalFrustum)
        assert_eq!(get_edge_radius(&Corner::Edge), 0.0);
        assert_eq!(get_edge_radius(&Corner::SmoothEdge), 0.0);
        assert_eq!(get_edge_radius(&Corner::Chamfer { distance: 0.5 }), 0.5);
        assert_eq!(get_edge_radius(&Corner::Fillet { radius: 0.3 }), 0.3);
    }

    /// Test with exact parameters from vitest that was hanging
    #[test]
    fn test_cylinder_fillet_vitest_parameters() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );

        // This should complete without hanging
        let mesh = cylinder.to_polygon_mesh();
        assert!(
            mesh.is_ok(),
            "Cylinder mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.polygons.is_empty());

        // Should have 5 geometry regions (0=bottom cap, 1=bottom fillet, 2=body, 3=top fillet, 4=top cap)
        let geometry_ids: std::collections::HashSet<_> =
            mesh.polygons.iter().map(|p| p.geometry_id).collect();
        assert_eq!(
            geometry_ids.len(),
            5,
            "Expected 5 geometry regions, got {:?}",
            geometry_ids
        );
    }

    /// Test triangular mesh generation with exact vitest parameters
    #[test]
    fn test_cylinder_fillet_triangular_mesh_vitest_parameters() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );

        // This should complete without hanging
        let mesh = cylinder.to_triangular_mesh();
        assert!(
            mesh.is_ok(),
            "Triangular mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert_eq!(mesh.geometries.len(), 5, "Expected 5 material regions");
    }

    /// Test with SMALL fillet radius (0.05) - this was hanging in vitest
    #[test]
    fn test_cylinder_small_fillet_radius() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Fillet { radius: 0.05 },
            Corner::Fillet { radius: 0.05 },
            Material::Uniform("default".to_string()),
        );

        // This should complete without hanging
        let mesh = cylinder.to_triangular_mesh();
        assert!(
            mesh.is_ok(),
            "Triangular mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert_eq!(
            mesh.geometries.len(),
            1,
            "Expected 1 material region with Uniform"
        );
    }

    /// Test the profile creation with small fillet radius
    #[test]
    fn test_cylinder_small_fillet_profile_only() {
        let cylinder = Cylinder::new(
            0.5, // radius
            2.0, // height
            12,
            Corner::Fillet { radius: 0.05 },
            Corner::Fillet { radius: 0.05 },
            Material::Default,
        );

        // Create the profile using the ConicalFrustum conversion
        let frustum = cylinder.to_conical_frustum();
        let profile = frustum.create_frustum_profile(1.0, 0.5); // half_height = 1.0, max_radius = 0.5

        // Convert to curve set
        let curve_set = profile.to_curve_set();
        assert!(
            curve_set.is_ok(),
            "Curve set creation failed: {:?}",
            curve_set.err()
        );

        let curve_set = curve_set.unwrap();
        assert_eq!(curve_set.curves.len(), 1, "Expected 1 curve");

        // Verify all segment lengths are reasonable
        for curve in curve_set.curves.iter() {
            for seg in curve.segments.iter() {
                let len: f64 = seg.length();
                assert!(
                    len < 10.0,
                    "Segment {} has unreasonable length: {}",
                    seg.segment_id,
                    len
                );
            }
        }
    }

    /// Test with LARGE fillet radius (0.2) - test for the vitest
    #[test]
    fn test_cylinder_large_fillet_radius() {
        let cylinder = Cylinder::new(
            0.5,
            2.0,
            12,
            Corner::Fillet { radius: 0.2 },
            Corner::Fillet { radius: 0.2 },
            Material::Uniform("default".to_string()),
        );

        // This should complete without hanging
        let mesh = cylinder.to_triangular_mesh();
        assert!(
            mesh.is_ok(),
            "Triangular mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert_eq!(
            mesh.geometries.len(),
            1,
            "Expected 1 material region with Uniform"
        );
    }

    // ============================================================================
    // ConicalFrustum tests
    // ============================================================================

    #[test]
    fn test_conical_frustum_new() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        assert_eq!(frustum.bottom_radius, 0.5);
        assert_eq!(frustum.top_radius, 0.25);
        assert_eq!(frustum.height, 2.0);
        assert_eq!(frustum.segments, 24);
        assert_eq!(frustum.bottom_edge, Corner::Edge);
        assert_eq!(frustum.top_edge, Corner::Edge);
        assert_eq!(frustum.material, Material::Default);
    }

    #[test]
    fn test_conical_frustum_to_polygon_mesh_basic() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );

        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_ok(), "Mesh creation failed: {:?}", mesh.err());

        let mesh = mesh.unwrap();
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.polygons.is_empty());
    }

    #[test]
    fn test_conical_frustum_with_chamfered_edges() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );

        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_ok(), "Mesh creation failed: {:?}", mesh.err());

        let mesh = mesh.unwrap();
        // Should have 5 geometry regions
        let geometry_ids: std::collections::HashSet<_> =
            mesh.polygons.iter().map(|p| p.geometry_id).collect();
        assert_eq!(
            geometry_ids.len(),
            5,
            "Expected 5 geometry regions, got {:?}",
            geometry_ids
        );
    }

    #[test]
    fn test_conical_frustum_with_filleted_edges() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );

        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_ok(), "Mesh creation failed: {:?}", mesh.err());

        let mesh = mesh.unwrap();
        // Should have 5 geometry regions
        let geometry_ids: std::collections::HashSet<_> =
            mesh.polygons.iter().map(|p| p.geometry_id).collect();
        assert_eq!(
            geometry_ids.len(),
            5,
            "Expected 5 geometry regions, got {:?}",
            geometry_ids
        );
    }

    #[test]
    fn test_conical_frustum_invalid_negative_bottom_radius() {
        let frustum = ConicalFrustum::new(
            -0.5, // invalid
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_err());
    }

    #[test]
    fn test_conical_frustum_invalid_negative_top_radius() {
        let frustum = ConicalFrustum::new(
            0.5,
            -0.25, // invalid
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_err());
    }

    #[test]
    fn test_conical_frustum_invalid_zero_radii() {
        let frustum = ConicalFrustum::new(
            0.0,
            0.0, // both zero is invalid
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_err());
    }

    #[test]
    fn test_conical_frustum_invalid_zero_height() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            0.0, // invalid
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_err());
    }

    #[test]
    fn test_conical_frustum_invalid_segments() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            2, // invalid - must be at least 3
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_err());
    }

    #[test]
    fn test_conical_frustum_invalid_edge_sizes() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            0.5,
            24,
            Corner::Chamfer { distance: 0.3 }, // too large
            Corner::Chamfer { distance: 0.3 }, // too large
            Material::Default,
        );
        let mesh = frustum.to_polygon_mesh();
        assert!(mesh.is_err());
    }

    #[test]
    fn test_conical_frustum_triangular_mesh() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );

        let mesh = frustum.to_triangular_mesh();
        assert!(mesh.is_ok(), "Mesh creation failed: {:?}", mesh.err());

        let mesh = mesh.unwrap();
        // Sharp edges: 3 geometry regions (bottom_cap, body, top_cap)
        assert_eq!(mesh.geometries.len(), 3);
    }

    #[test]
    fn test_conical_frustum_triangular_mesh_with_5_materials() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Array(vec![
                "bottom_cap".to_string(),
                "bottom_edge".to_string(),
                "body".to_string(),
                "top_edge".to_string(),
                "top_cap".to_string(),
            ]),
        );

        let mesh = frustum.to_triangular_mesh();
        assert!(mesh.is_ok(), "Mesh creation failed: {:?}", mesh.err());

        let mesh = mesh.unwrap();
        assert_eq!(mesh.geometries.len(), 5);
    }

    #[test]
    fn test_conical_frustum_cone_shape() {
        // Test a cone (top_radius = 0)
        let cone = ConicalFrustum::new(
            0.5,
            0.0, // cone tip
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );

        let mesh = cone.to_polygon_mesh();
        assert!(mesh.is_ok(), "Cone mesh creation failed: {:?}", mesh.err());
    }

    #[test]
    fn test_conical_frustum_inverted_cone_shape() {
        // Test an inverted cone (bottom_radius = 0)
        let cone = ConicalFrustum::new(
            0.0, // point at bottom
            0.5,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );

        let mesh = cone.to_polygon_mesh();
        assert!(
            mesh.is_ok(),
            "Inverted cone mesh creation failed: {:?}",
            mesh.err()
        );
    }

    #[test]
    fn test_conical_frustum_equal_radii_is_cylinder() {
        // When both radii are equal, it should behave like a cylinder
        let frustum = ConicalFrustum::new(
            0.5,
            0.5, // same as bottom
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );

        let mesh = frustum.to_polygon_mesh();
        assert!(
            mesh.is_ok(),
            "Equal radii mesh creation failed: {:?}",
            mesh.err()
        );
    }

    #[test]
    fn test_conical_frustum_sweep_model_material() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Uniform("test_material".to_string()),
        );
        let sweep_model = frustum.to_sweep_model();

        assert_eq!(
            sweep_model.material,
            Material::Uniform("test_material".to_string())
        );
    }

    #[test]
    fn test_conical_frustum_profile_no_edges() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Edge,
            Corner::Edge,
            Material::Default,
        );
        let profile = frustum.create_frustum_profile(1.0, 0.5);

        // Profile should have exactly 4 commands: Move, Line, Line, Line
        assert_eq!(profile.len(), 4);
    }

    #[test]
    fn test_conical_frustum_profile_with_chamfers() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            24,
            Corner::Chamfer { distance: 0.1 },
            Corner::Chamfer { distance: 0.1 },
            Material::Default,
        );
        let profile = frustum.create_frustum_profile(1.0, 0.5);

        // Profile should still have exactly 4 commands - corners are encoded in the Line commands
        assert_eq!(profile.len(), 4);

        // Verify the corner types
        assert!(matches!(profile.commands[1].corner, Corner::Chamfer { .. }));
        assert!(matches!(profile.commands[2].corner, Corner::Chamfer { .. }));
    }

    #[test]
    fn test_conical_frustum_get_edge_radius() {
        assert_eq!(get_edge_radius(&Corner::Edge), 0.0);
        assert_eq!(get_edge_radius(&Corner::SmoothEdge), 0.0);
        assert_eq!(get_edge_radius(&Corner::Chamfer { distance: 0.5 }), 0.5);
        assert_eq!(get_edge_radius(&Corner::Fillet { radius: 0.3 }), 0.3);
    }

    #[test]
    fn test_conical_frustum_fillet_parameters() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Fillet { radius: 0.1 },
            Corner::Fillet { radius: 0.1 },
            Material::Default,
        );

        let mesh = frustum.to_polygon_mesh();
        assert!(
            mesh.is_ok(),
            "ConicalFrustum mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.polygons.is_empty());

        // Should have 5 geometry regions
        let geometry_ids: std::collections::HashSet<_> =
            mesh.polygons.iter().map(|p| p.geometry_id).collect();
        assert_eq!(
            geometry_ids.len(),
            5,
            "Expected 5 geometry regions, got {:?}",
            geometry_ids
        );
    }

    #[test]
    fn test_conical_frustum_small_fillet_radius() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Fillet { radius: 0.05 },
            Corner::Fillet { radius: 0.05 },
            Material::Uniform("default".to_string()),
        );

        let mesh = frustum.to_triangular_mesh();
        assert!(
            mesh.is_ok(),
            "Triangular mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert_eq!(
            mesh.geometries.len(),
            1,
            "Expected 1 material region with Uniform"
        );
    }

    #[test]
    fn test_conical_frustum_large_fillet_radius() {
        let frustum = ConicalFrustum::new(
            0.5,
            0.25,
            2.0,
            12,
            Corner::Fillet { radius: 0.2 },
            Corner::Fillet { radius: 0.2 },
            Material::Uniform("default".to_string()),
        );

        let mesh = frustum.to_triangular_mesh();
        assert!(
            mesh.is_ok(),
            "Triangular mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert_eq!(
            mesh.geometries.len(),
            1,
            "Expected 1 material region with Uniform"
        );
    }

    // ============================================================================
    // Extrude tests
    // ============================================================================

    #[test]
    fn test_extrude_new() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile.clone(), 2.0, 0.1, Material::Default);

        assert_eq!(extrude.profile, profile);
        assert_eq!(extrude.height, 2.0);
        assert_eq!(extrude.sample_distance, 0.1);
        assert_eq!(extrude.material, Material::Default);
    }

    #[test]
    fn test_extrude_square_to_polygon_mesh() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);

        let mesh = extrude.to_polygon_mesh();
        assert!(mesh.is_ok(), "Mesh creation failed: {:?}", mesh.err());

        let mesh = mesh.unwrap();
        assert!(!mesh.vertices.is_empty(), "Mesh should have vertices");
        assert!(!mesh.polygons.is_empty(), "Mesh should have polygons");
    }

    #[test]
    fn test_extrude_height_bounds() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let height = 4.0;
        let extrude = Extrude::new(profile, height, 0.1, Material::Default);

        let mesh = extrude.to_polygon_mesh().unwrap();

        // Check that vertices span from -height/2 to +height/2
        let min_y = mesh
            .vertices
            .iter()
            .map(|v| v.position.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = mesh
            .vertices
            .iter()
            .map(|v| v.position.y)
            .fold(f64::NEG_INFINITY, f64::max);

        let half_height = height / 2.0;
        assert!(
            (min_y - (-half_height)).abs() < 1e-10,
            "Minimum Y should be -height/2, got {}",
            min_y
        );
        assert!(
            (max_y - half_height).abs() < 1e-10,
            "Maximum Y should be +height/2, got {}",
            max_y
        );
    }

    #[test]
    fn test_extrude_invalid_height() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile.clone(), 0.0, 0.1, Material::Default);
        assert!(
            extrude.to_polygon_mesh().is_err(),
            "Zero height should fail"
        );

        let extrude = Extrude::new(profile.clone(), -1.0, 0.1, Material::Default);
        assert!(
            extrude.to_polygon_mesh().is_err(),
            "Negative height should fail"
        );
    }

    #[test]
    fn test_extrude_invalid_sample_distance() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile.clone(), 1.0, 0.0, Material::Default);
        assert!(
            extrude.to_polygon_mesh().is_err(),
            "Zero sample distance should fail"
        );

        let extrude = Extrude::new(profile.clone(), 1.0, -0.1, Material::Default);
        assert!(
            extrude.to_polygon_mesh().is_err(),
            "Negative sample distance should fail"
        );
    }

    #[test]
    fn test_extrude_empty_profile() {
        let profile = PlanarProfile::new();
        let extrude = Extrude::new(profile, 1.0, 0.1, Material::Default);
        assert!(
            extrude.to_polygon_mesh().is_err(),
            "Empty profile should fail"
        );
    }

    #[test]
    fn test_extrude_with_fillet_corners() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.05, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(
            mesh.is_ok(),
            "Mesh creation with fillets failed: {:?}",
            mesh.err()
        );
        let mesh = mesh.unwrap();

        // With fillets, we should have more vertices than a simple square extrusion
        assert!(
            mesh.vertices.len() > 8,
            "Filleted extrude should have more than 8 vertices"
        );
    }

    #[test]
    fn test_extrude_with_chamfer_corners() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(
            mesh.is_ok(),
            "Mesh creation with chamfers failed: {:?}",
            mesh.err()
        );
        let mesh = mesh.unwrap();

        // With chamfers, we should have more vertices than a simple square extrusion
        assert!(
            mesh.vertices.len() > 8,
            "Chamfered extrude should have more than 8 vertices"
        );
    }

    #[test]
    fn test_extrude_uniform_material() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(
            profile,
            2.0,
            0.1,
            Material::Uniform("test_material".to_string()),
        );

        let mesh = extrude.to_polygon_mesh().unwrap();

        // All materials should be "test_material"
        for (_, material) in &mesh.material_map {
            assert_eq!(material, "test_material");
        }
    }

    #[test]
    fn test_extrude_cap_geometry_ids() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh().unwrap();

        // Check that bottom cap has geometry_id = -1 and top cap has geometry_id = -2
        let bottom_cap_polygons: Vec<_> = mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
            .collect();
        let top_cap_polygons: Vec<_> = mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
            .collect();

        assert!(
            !bottom_cap_polygons.is_empty(),
            "Should have bottom cap polygons"
        );
        assert!(!top_cap_polygons.is_empty(), "Should have top cap polygons");
    }

    #[test]
    fn test_extrude_cap_normals() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh().unwrap();

        // Check bottom cap normals point down (negative Y)
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
        {
            assert!(
                polygon.normal.y < -0.9,
                "Bottom cap normal should point down, got {:?}",
                polygon.normal
            );
        }

        // Check top cap normals point up (positive Y)
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
        {
            assert!(
                polygon.normal.y > 0.9,
                "Top cap normal should point up, got {:?}",
                polygon.normal
            );
        }
    }

    #[test]
    fn test_extrude_cap_winding_order() {
        // This test verifies that the winding order of cap polygons is correct
        // by computing the face normal from vertex positions and checking it
        // matches the expected direction.
        //
        // For correct single-sided rendering:
        // - Top cap: vertices should be CCW when viewed from +Y (above)
        // - Bottom cap: vertices should be CCW when viewed from -Y (below)

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh().unwrap();

        // Helper function to compute face normal from first 3 vertices of a polygon
        let compute_face_normal = |polygon: &crate::polygon_mesh::Polygon| {
            if polygon.indices.len() < 3 {
                return DVec3::ZERO;
            }
            let v0 = mesh.vertices[polygon.indices[0] as usize].position;
            let v1 = mesh.vertices[polygon.indices[1] as usize].position;
            let v2 = mesh.vertices[polygon.indices[2] as usize].position;
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            edge1.cross(edge2).normalize()
        };

        // Check bottom cap: computed normal should point down (-Y)
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
        {
            let computed_normal = compute_face_normal(polygon);
            assert!(
                computed_normal.y < -0.9,
                "Bottom cap computed normal should point down (-Y), got {:?}. \
                 This indicates incorrect winding order.",
                computed_normal
            );
        }

        // Check top cap: computed normal should point up (+Y)
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
        {
            let computed_normal = compute_face_normal(polygon);
            assert!(
                computed_normal.y > 0.9,
                "Top cap computed normal should point up (+Y), got {:?}. \
                 This indicates incorrect winding order.",
                computed_normal
            );
        }
    }

    #[test]
    fn test_extrude_cap_visibility_from_outside() {
        // This test ensures caps are visible from outside the extrusion
        // (top cap visible from above, bottom cap visible from below)
        // by checking that cap vertices are at the correct Y positions
        // and their normals face outward.

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(-0.5, -0.5),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, -0.5),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 0.5),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(-0.5, 0.5),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(-0.5, -0.5),
                },
                Corner::Edge,
            ),
        ]);

        let height = 2.0;
        let half_height = height / 2.0;
        let extrude = Extrude::new(profile, height, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh().unwrap();

        // Check bottom cap vertices are at Y = -half_height
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
        {
            for &idx in &polygon.indices {
                let vertex = &mesh.vertices[idx as usize];
                assert!(
                    (vertex.position.y - (-half_height)).abs() < 1e-6,
                    "Bottom cap vertex should be at Y = {}, got {}",
                    -half_height,
                    vertex.position.y
                );
                // Vertex normal should also point down
                assert!(
                    vertex.normal.y < -0.9,
                    "Bottom cap vertex normal should point down"
                );
            }
        }

        // Check top cap vertices are at Y = +half_height
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
        {
            for &idx in &polygon.indices {
                let vertex = &mesh.vertices[idx as usize];
                assert!(
                    (vertex.position.y - half_height).abs() < 1e-6,
                    "Top cap vertex should be at Y = {}, got {}",
                    half_height,
                    vertex.position.y
                );
                // Vertex normal should also point up
                assert!(
                    vertex.normal.y > 0.9,
                    "Top cap vertex normal should point up"
                );
            }
        }
    }

    #[test]
    fn test_extrude_triangle_profile() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(mesh.is_ok(), "Triangle extrude should succeed");
        let mesh = mesh.unwrap();

        // Should have side faces plus top and bottom caps
        assert!(!mesh.polygons.is_empty(), "Should have polygons");
    }

    #[test]
    fn test_extrude_circle_profile() {
        let center = DVec2::new(0.0, 0.0);
        let radius = 1.0;
        let start = DVec2::new(radius, 0.0);

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(PathCommand::Move { point: start }, Corner::Edge),
            ProfileCommand::new(
                PathCommand::Ellipse {
                    point: start,
                    center,
                },
                Corner::SmoothEdge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(mesh.is_ok(), "Circle extrude should succeed");
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty(), "Should have vertices");
        assert!(!mesh.polygons.is_empty(), "Should have polygons");
    }

    #[test]
    fn test_extrude_to_triangular_mesh() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Uniform("test".to_string()));

        let mesh = extrude.to_triangular_mesh();
        assert!(
            mesh.is_ok(),
            "Triangular mesh creation failed: {:?}",
            mesh.err()
        );

        let mesh = mesh.unwrap();
        assert_eq!(
            mesh.geometries.len(),
            1,
            "Expected 1 material region with Uniform"
        );
    }

    #[test]
    fn test_extrude_material_map() {
        use std::collections::HashMap;

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let mut face_map = HashMap::new();
        face_map.insert(EXTRUDE_BOTTOM_CAP_ID, "bottom_material".to_string());
        face_map.insert(EXTRUDE_TOP_CAP_ID, "top_material".to_string());
        face_map.insert(0, "side_material".to_string());

        let extrude = Extrude::new(
            profile,
            2.0,
            0.1,
            Material::Map {
                default: "default".to_string(),
                face_map,
            },
        );

        let mesh = extrude.to_polygon_mesh().unwrap();

        // Check that the material map was applied correctly
        assert_eq!(
            mesh.material_map.get(&EXTRUDE_BOTTOM_CAP_ID),
            Some(&"bottom_material".to_string())
        );
        assert_eq!(
            mesh.material_map.get(&EXTRUDE_TOP_CAP_ID),
            Some(&"top_material".to_string())
        );
    }

    #[test]
    fn test_extrude_serialization() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 1.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
        ]);

        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);

        // Test serialization
        let serialized = serde_json::to_string(&extrude);
        assert!(serialized.is_ok(), "Serialization should succeed");

        // Test deserialization
        let deserialized: Result<Extrude, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok(), "Deserialization should succeed");

        let deserialized = deserialized.unwrap();
        assert_eq!(deserialized.height, 2.0);
        assert_eq!(deserialized.sample_distance, 0.1);
    }

    /// Helper to create a regular polygon profile with n points
    fn create_regular_polygon_profile(n: usize, radius: f64) -> PlanarProfile {
        use std::f64::consts::PI;
        let mut commands = Vec::new();

        for i in 0..n {
            let angle = 2.0 * PI * (i as f64) / (n as f64);
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            let point = DVec2::new(x, y);

            if i == 0 {
                commands.push(ProfileCommand::new(
                    PathCommand::Move { point },
                    Corner::Edge,
                ));
            } else {
                commands.push(ProfileCommand::new(
                    PathCommand::Line { point },
                    Corner::Edge,
                ));
            }
        }

        // Close the polygon
        let first_point = DVec2::new(radius, 0.0); // angle = 0
        commands.push(ProfileCommand::new(
            PathCommand::Line { point: first_point },
            Corner::Edge,
        ));

        PlanarProfile::with_commands(commands)
    }

    #[test]
    fn test_extrude_heptagon_profile() {
        // Create a regular heptagon (7 sides)
        let profile = create_regular_polygon_profile(7, 1.0);
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(mesh.is_ok(), "Heptagon extrude should succeed");
        let mesh = mesh.unwrap();

        // Should have polygons for sides and caps
        assert!(!mesh.polygons.is_empty(), "Should have polygons");

        // Count polygons by geometry_id
        let mut bottom_cap_count = 0;
        let mut top_cap_count = 0;
        let mut side_count = 0;

        for polygon in &mesh.polygons {
            if polygon.geometry_id == EXTRUDE_BOTTOM_CAP_ID {
                bottom_cap_count += 1;
            } else if polygon.geometry_id == EXTRUDE_TOP_CAP_ID {
                top_cap_count += 1;
            } else {
                side_count += 1;
            }
        }

        println!(
            "Heptagon: bottom_cap={}, top_cap={}, sides={}",
            bottom_cap_count, top_cap_count, side_count
        );

        // For a heptagon with 7 edges:
        // - Should have at least 1 polygon for each cap
        // - Should have 7 side faces (one per edge)
        assert!(
            bottom_cap_count >= 1,
            "Should have at least 1 bottom cap polygon, got {}",
            bottom_cap_count
        );
        assert!(
            top_cap_count >= 1,
            "Should have at least 1 top cap polygon, got {}",
            top_cap_count
        );
        assert!(
            side_count >= 7,
            "Should have at least 7 side polygons, got {}",
            side_count
        );
    }

    #[test]
    fn test_extrude_regular_polygons_cap_vertex_count() {
        // Test regular polygons from 3 to 10 sides
        // This test verifies the bug fix for missing cap geometry on pentagons and heptagons
        for n in 3..=10 {
            let profile = create_regular_polygon_profile(n, 1.0);
            let extrude = Extrude::new(profile.clone(), 2.0, 0.1, Material::Default);
            let mesh = extrude.to_polygon_mesh().unwrap();

            // Count polygons by geometry_id
            let bottom_cap_vertex_indices: Vec<i32> = mesh
                .polygons
                .iter()
                .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
                .flat_map(|p| p.indices.clone())
                .collect();

            let top_cap_vertex_indices: Vec<i32> = mesh
                .polygons
                .iter()
                .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
                .flat_map(|p| p.indices.clone())
                .collect();

            // Count unique vertices used in caps
            let mut bottom_unique: std::collections::HashSet<i32> =
                std::collections::HashSet::new();
            for idx in &bottom_cap_vertex_indices {
                bottom_unique.insert(*idx);
            }

            let mut top_unique: std::collections::HashSet<i32> = std::collections::HashSet::new();
            for idx in &top_cap_vertex_indices {
                top_unique.insert(*idx);
            }

            // A regular n-gon should have n vertices in each cap
            assert!(
                bottom_unique.len() >= n,
                "For n={}, bottom cap should have at least {} vertices, got {}",
                n,
                n,
                bottom_unique.len()
            );
            assert!(
                top_unique.len() >= n,
                "For n={}, top cap should have at least {} vertices, got {}",
                n,
                n,
                top_unique.len()
            );
        }
    }

    /// Helper to create a profile with an outer polygon and optional inner hole
    fn create_polygon_with_hole_profile(
        outer_points: usize,
        outer_radius: f64,
        inner_points: Option<usize>,
        inner_radius: f64,
    ) -> PlanarProfile {
        use std::f64::consts::PI;
        let mut commands = Vec::new();

        // Outer polygon (CCW)
        for i in 0..outer_points {
            let angle = 2.0 * PI * (i as f64) / (outer_points as f64);
            let x = outer_radius * angle.cos();
            let y = outer_radius * angle.sin();
            let point = DVec2::new(x, y);

            if i == 0 {
                commands.push(ProfileCommand::new(
                    PathCommand::Move { point },
                    Corner::Edge,
                ));
            } else {
                commands.push(ProfileCommand::new(
                    PathCommand::Line { point },
                    Corner::Edge,
                ));
            }
        }
        // Close outer polygon
        let first_outer = DVec2::new(outer_radius, 0.0);
        commands.push(ProfileCommand::new(
            PathCommand::Line { point: first_outer },
            Corner::Edge,
        ));

        // Inner hole (CW - clockwise for holes)
        if let Some(inner_n) = inner_points {
            // Start hole with Move
            let first_inner_angle: f64 = 0.0;
            let first_inner = DVec2::new(
                inner_radius * first_inner_angle.cos(),
                inner_radius * first_inner_angle.sin(),
            );
            commands.push(ProfileCommand::new(
                PathCommand::Move { point: first_inner },
                Corner::Edge,
            ));

            // Add hole points in CW order
            for i in 1..inner_n {
                let angle = -2.0 * PI * (i as f64) / (inner_n as f64); // Negative for CW
                let x = inner_radius * angle.cos();
                let y = inner_radius * angle.sin();
                let point = DVec2::new(x, y);
                commands.push(ProfileCommand::new(
                    PathCommand::Line { point },
                    Corner::Edge,
                ));
            }
            // Close inner polygon
            commands.push(ProfileCommand::new(
                PathCommand::Line { point: first_inner },
                Corner::Edge,
            ));
        }

        PlanarProfile::with_commands(commands)
    }

    #[test]
    fn test_extrude_pentagon_with_square_hole() {
        // This tests the specific case: 5-point outline with 4-point hole
        let profile = create_polygon_with_hole_profile(5, 2.0, Some(4), 0.5);
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(
            mesh.is_ok(),
            "Pentagon with square hole extrude should succeed"
        );
        let mesh = mesh.unwrap();

        // Count polygons by geometry_id
        let bottom_cap_polygons: Vec<_> = mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
            .collect();

        let top_cap_polygons: Vec<_> = mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
            .collect();

        println!(
            "Bottom cap polygons: {}, Top cap polygons: {}",
            bottom_cap_polygons.len(),
            top_cap_polygons.len()
        );

        // Count unique vertices in caps
        let bottom_cap_vertex_indices: Vec<i32> = bottom_cap_polygons
            .iter()
            .flat_map(|p| p.indices.clone())
            .collect();

        let top_cap_vertex_indices: Vec<i32> = top_cap_polygons
            .iter()
            .flat_map(|p| p.indices.clone())
            .collect();

        let mut bottom_unique: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for idx in &bottom_cap_vertex_indices {
            bottom_unique.insert(*idx);
        }

        let mut top_unique: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for idx in &top_cap_vertex_indices {
            top_unique.insert(*idx);
        }

        println!(
            "Bottom cap unique vertices: {}, Top cap unique vertices: {}",
            bottom_unique.len(),
            top_unique.len()
        );

        // 5 outer + 4 inner = 9 vertices should be in each cap
        let expected_vertices = 5 + 4;
        assert!(
            bottom_cap_polygons.len() >= 1,
            "Should have at least 1 bottom cap polygon, got {}",
            bottom_cap_polygons.len()
        );
        assert!(
            top_cap_polygons.len() >= 1,
            "Should have at least 1 top cap polygon, got {}",
            top_cap_polygons.len()
        );
        assert!(
            bottom_unique.len() >= expected_vertices,
            "Bottom cap should have at least {} vertices, got {}",
            expected_vertices,
            bottom_unique.len()
        );
        assert!(
            top_unique.len() >= expected_vertices,
            "Top cap should have at least {} vertices, got {}",
            expected_vertices,
            top_unique.len()
        );
    }

    #[test]
    fn test_extrude_various_outline_hole_combinations() {
        // Test various combinations of outline and hole point counts
        let test_cases = vec![
            (3, 0), // Triangle, no hole
            (4, 0), // Square, no hole
            (5, 0), // Pentagon, no hole
            (6, 0), // Hexagon, no hole
            (7, 0), // Heptagon, no hole
            (4, 3), // Square with triangle hole
            (5, 3), // Pentagon with triangle hole
            (5, 4), // Pentagon with square hole (the reported failing case)
            (6, 3), // Hexagon with triangle hole
            (6, 4), // Hexagon with square hole
            (7, 4), // Heptagon with square hole
            (7, 5), // Heptagon with pentagon hole
            (8, 5), // Octagon with pentagon hole
        ];

        for (outer_n, inner_n) in test_cases {
            let inner_opt = if inner_n > 0 { Some(inner_n) } else { None };
            let profile = create_polygon_with_hole_profile(outer_n, 2.0, inner_opt, 0.5);
            let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
            let mesh = extrude.to_polygon_mesh();

            assert!(
                mesh.is_ok(),
                "Extrude with {} outer points and {} hole points should succeed",
                outer_n,
                inner_n
            );
            let mesh = mesh.unwrap();

            // Count cap polygons
            let bottom_cap_count = mesh
                .polygons
                .iter()
                .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
                .count();

            let top_cap_count = mesh
                .polygons
                .iter()
                .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
                .count();

            let expected_vertices = outer_n + inner_n;

            // Count unique vertices in caps
            let bottom_unique: std::collections::HashSet<i32> = mesh
                .polygons
                .iter()
                .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
                .flat_map(|p| p.indices.clone())
                .collect();

            let top_unique: std::collections::HashSet<i32> = mesh
                .polygons
                .iter()
                .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
                .flat_map(|p| p.indices.clone())
                .collect();

            println!(
                "outer={}, inner={}: bottom_polys={}, top_polys={}, bottom_verts={}, top_verts={}",
                outer_n,
                inner_n,
                bottom_cap_count,
                top_cap_count,
                bottom_unique.len(),
                top_unique.len()
            );

            assert!(
                bottom_cap_count >= 1,
                "outer={}, inner={}: Should have at least 1 bottom cap polygon, got {}",
                outer_n,
                inner_n,
                bottom_cap_count
            );
            assert!(
                top_cap_count >= 1,
                "outer={}, inner={}: Should have at least 1 top cap polygon, got {}",
                outer_n,
                inner_n,
                top_cap_count
            );
            assert!(
                bottom_unique.len() >= expected_vertices,
                "outer={}, inner={}: Bottom cap should have at least {} vertices, got {}",
                outer_n,
                inner_n,
                expected_vertices,
                bottom_unique.len()
            );
            assert!(
                top_unique.len() >= expected_vertices,
                "outer={}, inner={}: Top cap should have at least {} vertices, got {}",
                outer_n,
                inner_n,
                expected_vertices,
                top_unique.len()
            );
        }
    }

    #[test]
    fn test_extrude_pentagon_with_square_hole_winding() {
        // This test verifies that the winding order of cap polygons is correct
        // for the specific case of a pentagon with a square hole.
        // This ensures faces are visible from the correct direction.

        let profile = create_polygon_with_hole_profile(5, 2.0, Some(4), 0.5);
        let extrude = Extrude::new(profile, 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh().unwrap();

        // Helper function to compute face normal from first 3 vertices of a polygon
        let compute_face_normal = |polygon: &crate::polygon_mesh::Polygon| -> DVec3 {
            if polygon.indices.len() < 3 {
                return DVec3::ZERO;
            }
            let v0 = mesh.vertices[polygon.indices[0] as usize].position;
            let v1 = mesh.vertices[polygon.indices[1] as usize].position;
            let v2 = mesh.vertices[polygon.indices[2] as usize].position;
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            edge1.cross(edge2).normalize()
        };

        // Check bottom cap: computed normal should point down (-Y)
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_BOTTOM_CAP_ID)
        {
            let computed_normal = compute_face_normal(polygon);
            assert!(
                computed_normal.y < -0.9,
                "Bottom cap computed normal should point down (-Y), got {:?}. \
                 Polygon indices: {:?}. This indicates incorrect winding order.",
                computed_normal,
                polygon.indices
            );
        }

        // Check top cap: computed normal should point up (+Y)
        for polygon in mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id == EXTRUDE_TOP_CAP_ID)
        {
            let computed_normal = compute_face_normal(polygon);
            assert!(
                computed_normal.y > 0.9,
                "Top cap computed normal should point up (+Y), got {:?}. \
                 Polygon indices: {:?}. This indicates incorrect winding order.",
                computed_normal,
                polygon.indices
            );
        }
    }

    #[test]
    fn test_extrude_triangle_with_chamfer_corners() {
        // Create a triangle profile with chamfer corners
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 0.866),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
        ]);

        let extrude = Extrude::new(profile.clone(), 2.0, 0.1, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(
            mesh.is_ok(),
            "Triangle extrude with chamfer should succeed: {:?}",
            mesh.err()
        );
        let mesh = mesh.unwrap();

        // With chamfers, we should have 6 edges on the side faces (3 edges + 3 chamfers)
        // Check cap vertex count matches number of profile segments
        let contour_set = profile.sample_contour(0.1).unwrap();
        let total_profile_points: usize = contour_set
            .contours
            .iter()
            .flat_map(|c| c.segments.iter())
            .count();

        // With 3 chamfers on a triangle, we expect 6 profile segments
        assert_eq!(
            total_profile_points, 6,
            "Triangle with 3 chamfers should have 6 profile segments"
        );

        // All side face polygons (geometry_id >= 0)
        let side_polygons: Vec<_> = mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id >= 0)
            .collect();

        // Should have 6 side face polygons (one for each segment)
        assert_eq!(
            side_polygons.len(),
            6,
            "Should have 6 side face polygons for chamfered triangle"
        );

        // Collect all unique geometry IDs for side faces
        let mut geometry_ids: Vec<i32> = side_polygons.iter().map(|p| p.geometry_id).collect();
        geometry_ids.sort();
        geometry_ids.dedup();

        // Should have 6 different geometry IDs (0, 1, 2, 3, 4, 5)
        println!("Geometry IDs found: {:?}", geometry_ids);
        assert_eq!(
            geometry_ids.len(),
            6,
            "Should have 6 unique geometry IDs for chamfered triangle: {:?}",
            geometry_ids
        );

        // Verify side face normals are horizontal (perpendicular to Y axis)
        for polygon in &side_polygons {
            // Get polygon vertices
            let _v0 = mesh.vertices[polygon.indices[0] as usize].position;
            let _v1 = mesh.vertices[polygon.indices[1] as usize].position;
            let _v2 = mesh.vertices[polygon.indices[2] as usize].position;

            // Check that the vertices span both the top and bottom of the extrude
            let y_coords: Vec<f64> = polygon
                .indices
                .iter()
                .map(|&i| mesh.vertices[i as usize].position.y)
                .collect();
            let min_y = y_coords.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_y = y_coords.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            // Side faces should span from bottom (-1) to top (+1)
            assert!(
                (min_y - (-1.0)).abs() < 0.01 && (max_y - 1.0).abs() < 0.01,
                "Side face should span from bottom to top: min_y={}, max_y={}, geometry_id={}",
                min_y,
                max_y,
                polygon.geometry_id
            );
        }

        println!(
            "Triangle chamfer extrude: {} vertices, {} polygons, {} profile points",
            mesh.vertices.len(),
            mesh.polygons.len(),
            total_profile_points
        );
    }

    #[test]
    fn test_extrude_triangle_with_fillet_corners() {
        // Create a triangle profile with fillet corners
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 0.866),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
        ]);

        let extrude = Extrude::new(profile.clone(), 2.0, 0.05, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(
            mesh.is_ok(),
            "Triangle extrude with fillet should succeed: {:?}",
            mesh.err()
        );
        let mesh = mesh.unwrap();

        // All side face polygons (geometry_id >= 0)
        let side_polygons: Vec<_> = mesh
            .polygons
            .iter()
            .filter(|p| p.geometry_id >= 0)
            .collect();

        assert!(
            !side_polygons.is_empty(),
            "Should have side face polygons for filleted triangle"
        );

        // Verify side face normals are horizontal (perpendicular to Y axis)
        for polygon in &side_polygons {
            // Check that the vertices span both the top and bottom of the extrude
            let y_coords: Vec<f64> = polygon
                .indices
                .iter()
                .map(|&i| mesh.vertices[i as usize].position.y)
                .collect();
            let min_y = y_coords.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_y = y_coords.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            // Side faces should span from bottom (-1) to top (+1)
            assert!(
                (min_y - (-1.0)).abs() < 0.01 && (max_y - 1.0).abs() < 0.01,
                "Side face should span from bottom to top: min_y={}, max_y={}, geometry_id={}",
                min_y,
                max_y,
                polygon.geometry_id
            );
        }

        println!(
            "Triangle fillet extrude: {} vertices, {} polygons",
            mesh.vertices.len(),
            mesh.polygons.len()
        );
    }

    #[test]
    fn test_extrude_triangle_with_chamfer_and_triangle_hole_with_chamfer() {
        // This test matches the exact user scenario:
        // 3-point outline with chamfer corners + 3-point hole with chamfer corners
        let chamfer_distance = 0.1;
        let hole_chamfer_distance = 0.03;

        // Outer triangle (CCW winding) - radius 0.8
        let outer_radius = 0.8;
        let outer_points: Vec<DVec2> = (0..3)
            .map(|i| {
                let angle =
                    (i as f64 / 3.0) * std::f64::consts::PI * 2.0 - std::f64::consts::PI / 2.0;
                DVec2::new(angle.cos() * outer_radius, angle.sin() * outer_radius)
            })
            .collect();

        // Inner triangle (CW winding for hole) - radius 0.3
        let inner_radius = 0.3;
        let inner_points: Vec<DVec2> = (0..3)
            .map(|i| {
                let angle =
                    (i as f64 / 3.0) * std::f64::consts::PI * 2.0 - std::f64::consts::PI / 2.0;
                DVec2::new(angle.cos() * inner_radius, angle.sin() * inner_radius)
            })
            .collect();

        // Build profile commands
        let commands = vec![
            // Outer triangle
            ProfileCommand::new(
                PathCommand::Move {
                    point: outer_points[0],
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: outer_points[1],
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: outer_points[2],
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: outer_points[0],
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            // Inner triangle (reversed for hole - CW winding)
            ProfileCommand::new(
                PathCommand::Move {
                    point: inner_points[0],
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: inner_points[2],
                }, // Reversed
                Corner::Chamfer {
                    distance: hole_chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: inner_points[1],
                }, // Reversed
                Corner::Chamfer {
                    distance: hole_chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: inner_points[0],
                },
                Corner::Chamfer {
                    distance: hole_chamfer_distance,
                },
            ),
        ];

        let profile = PlanarProfile::with_commands(commands);

        // Verify profile converts to curve set correctly
        let curve_set = profile.to_curve_set();
        assert!(
            curve_set.is_ok(),
            "Profile should convert to curve set: {:?}",
            curve_set.err()
        );
        let curve_set = curve_set.unwrap();

        // Should have 2 curves (outer + hole)
        assert_eq!(
            curve_set.curves.len(),
            2,
            "Should have 2 curves (outline + hole)"
        );

        // Print segment details for debugging BEFORE assertions
        println!("\n=== Outer curve segments (expected 6) ===");
        for (i, seg) in curve_set.curves[0].segments.iter().enumerate() {
            let start = seg.nurbs.evaluate(0.0);
            let end = seg.nurbs.evaluate(1.0);
            println!(
                "  Segment {} (id={}): ({:.4}, {:.4}) -> ({:.4}, {:.4})",
                i, seg.segment_id, start.x, start.y, end.x, end.y
            );
        }

        println!("\n=== Inner curve (hole) segments (expected 6) ===");
        for (i, seg) in curve_set.curves[1].segments.iter().enumerate() {
            let start = seg.nurbs.evaluate(0.0);
            let end = seg.nurbs.evaluate(1.0);
            println!(
                "  Segment {} (id={}): ({:.4}, {:.4}) -> ({:.4}, {:.4})",
                i, seg.segment_id, start.x, start.y, end.x, end.y
            );
        }

        // Outer curve should have 6 segments (3 edges + 3 chamfers)
        assert_eq!(
            curve_set.curves[0].segments.len(),
            6,
            "Outer triangle with 3 chamfers should have 6 segments, got {}",
            curve_set.curves[0].segments.len()
        );

        // Create extrude
        let extrude = Extrude::new(profile.clone(), 1.0, 0.05, Material::Default);
        let mesh = extrude.to_polygon_mesh();

        assert!(
            mesh.is_ok(),
            "Extrude with triangle outline + triangle hole should succeed: {:?}",
            mesh.err()
        );
        let mesh = mesh.unwrap();

        // Count side polygons by geometry_id
        let mut side_polygon_count_by_id: std::collections::HashMap<i32, usize> =
            std::collections::HashMap::new();
        for polygon in &mesh.polygons {
            if polygon.geometry_id >= 0 {
                *side_polygon_count_by_id
                    .entry(polygon.geometry_id)
                    .or_insert(0) += 1;
            }
        }

        println!("\n=== Side polygon counts by geometry_id ===");
        let mut ids: Vec<_> = side_polygon_count_by_id.keys().collect();
        ids.sort();
        for id in &ids {
            println!(
                "  geometry_id {}: {} polygons",
                id, side_polygon_count_by_id[id]
            );
        }

        // The extrude should have side faces for both outer and inner curves
        // Each curve has 6 segments, so we expect 12 side face polygons total
        // (geometry IDs may overlap between curves since they're separate contours)
        let total_side_polygons: usize = side_polygon_count_by_id.values().sum();
        assert!(
            total_side_polygons >= 12,
            "Should have at least 12 side face polygons (6 outer + 6 inner), got {}",
            total_side_polygons
        );

        // Count cap polygons
        let bottom_cap_count = mesh.polygons.iter().filter(|p| p.geometry_id == -1).count();
        let top_cap_count = mesh.polygons.iter().filter(|p| p.geometry_id == -2).count();

        println!("\n=== Cap polygon counts ===");
        println!("  Bottom cap (id=-1): {} polygons", bottom_cap_count);
        println!("  Top cap (id=-2): {} polygons", top_cap_count);

        assert!(bottom_cap_count > 0, "Should have bottom cap polygons");
        assert!(top_cap_count > 0, "Should have top cap polygons");

        println!(
            "\nTotal: {} vertices, {} polygons",
            mesh.vertices.len(),
            mesh.polygons.len()
        );
    }
}
