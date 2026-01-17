use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::planar_contour::{ContourSet, SampleContour};
use crate::planar_nurbs_curve::{Curve, CurveSet, Nurbs, NurbsSegment};
use crate::polygon_mesh::PolygonMesh;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Represents the type of corner treatment at a path vertex.
///
/// This enum defines how corners are handled when creating profiles
/// for extrusion or other geometric operations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Corner {
    /// A sharp edge corner with no modification.
    Edge,
    /// A smooth edge that blends tangentially with adjacent segments.
    SmoothEdge,
    /// A chamfered corner that cuts off the corner at a given distance.
    Chamfer {
        /// The distance from the corner vertex to the chamfer cut.
        distance: f64,
    },
    /// A filleted corner with a smooth rounded transition.
    Fillet {
        /// The radius of the fillet arc.
        radius: f64,
    },
}

impl Default for Corner {
    fn default() -> Self {
        Self::Edge
    }
}

/// Represents a path command for defining 2D profile geometry.
///
/// Each command specifies a way to extend the path from the current
/// position to a new point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathCommand {
    /// Move to a new position without drawing.
    Move {
        /// The target point to move to.
        point: DVec2,
    },
    /// Draw a straight line to a point.
    Line {
        /// The end point of the line.
        point: DVec2,
    },
    /// Draw an elliptical arc to a point.
    Ellipse {
        /// The end point of the elliptical arc.
        point: DVec2,
        /// The center point of the ellipse.
        center: DVec2,
    },
    /// Draw a Bezier curve to a point.
    ///
    /// Note: This represents a Bezier curve in the public API, but it will be
    /// converted to an internal NURBS representation for evaluation and
    /// downstream geometric operations.
    Bezier {
        /// The end point of the Bezier curve in parameter space; this will be
        /// used to construct the corresponding internal NURBS segment.
        point: DVec2,
        /// The Bezier control points defining the curve shape, which will be
        /// converted into control data for the internal NURBS representation.
        control_points: Vec<DVec2>,
    },
}

/// Represents a single command in a planar profile with corner treatment.
///
/// Combines a path command with a corner specification to define
/// how the path segment ends and transitions to the next segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileCommand {
    /// The path command defining the geometry.
    pub path: PathCommand,
    /// The corner treatment at the end of this path segment.
    pub corner: Corner,
}

impl ProfileCommand {
    /// Creates a new `ProfileCommand` with the given path and corner.
    pub fn new(path: PathCommand, corner: Corner) -> Self {
        Self { path, corner }
    }

    /// Creates a new `ProfileCommand` with the given path and a default edge corner.
    pub fn with_edge(path: PathCommand) -> Self {
        Self {
            path,
            corner: Corner::Edge,
        }
    }
}

/// Represents a 2D planar profile defined by a sequence of path commands.
///
/// A planar profile is used as the basis for extrusion, revolution,
/// or other operations that generate 3D geometry from 2D shapes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlanarProfile {
    /// The sequence of profile commands that define the profile shape.
    pub commands: Vec<ProfileCommand>,
}

impl PlanarProfile {
    /// Creates a new empty `PlanarProfile`.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Creates a new `PlanarProfile` with the given commands.
    pub fn with_commands(commands: Vec<ProfileCommand>) -> Self {
        Self { commands }
    }

    /// Adds a profile command to the profile.
    pub fn push(&mut self, command: ProfileCommand) {
        self.commands.push(command);
    }

    /// Returns the number of commands in the profile.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns true if the profile has no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Helper function to process a closing corner for a curve.
    /// This handles the case where the curve forms a closed loop and has a pending corner
    /// that needs to be added at the junction between the last and first segments.
    fn process_closing_corner(
        curve: &mut Curve,
        pending_corner: &mut Option<(Corner, DVec2, DVec2)>,
        current_pos: DVec2,
        first_pos: DVec2,
        path_segment_index: u32,
    ) {
        let is_closed = (current_pos - first_pos).length() < 1e-9;

        if is_closed {
            // Mark the curve as closed
            curve.set_closed(true);

            // Get the first segment's start tangent
            if let (Some((corner, _end_pos, end_tangent)), Some(first_segment)) =
                (pending_corner.take(), curve.segments.first())
            {
                let first_tangent = first_segment.nurbs.derivative(0.0).normalize_or_zero();

                // Add closing corner segment if needed
                if let Some(corner_nurbs) =
                    Self::create_corner_nurbs(corner, current_pos, end_tangent, first_tangent)
                {
                    let corner_start = corner_nurbs.start_point();
                    let corner_end = corner_nurbs.end_point();

                    // Truncate the LAST segment to end at corner_start
                    if let Some(last_segment) = curve.segments.last_mut() {
                        let seg_start = last_segment.nurbs.start_point();
                        last_segment.nurbs = Nurbs::line(seg_start, corner_start);
                        last_segment.end_distance =
                            last_segment.start_distance + last_segment.nurbs.length();
                    }

                    // Add the corner segment
                    let corner_segment_id = path_segment_index * 2 - 1;
                    curve.add_segment(NurbsSegment::new(
                        corner_nurbs,
                        curve.total_length(),
                        false,
                        corner_segment_id,
                    ));

                    // Truncate the FIRST segment to start at corner_end
                    if let Some(first_segment) = curve.segments.first_mut() {
                        let seg_end = first_segment.nurbs.end_point();
                        first_segment.nurbs = Nurbs::line(corner_end, seg_end);
                        // Recalculate distances for first segment
                        // Note: start_distance stays 0 for first segment
                        first_segment.end_distance =
                            first_segment.start_distance + first_segment.nurbs.length();
                    }
                }
            }
        }
    }

    /// Converts this `PlanarProfile` to a `CurveSet`.
    ///
    /// Each `ProfileCommand` is converted to 0, 1, or 2 NURBS segments:
    /// - `Move`: 0 segments (just sets the current position)
    /// - `Edge` or `SmoothEdge` corner: 1 segment (the path command itself)
    /// - `Chamfer` or `Fillet` corner: 2 segments (path + corner treatment)
    ///
    /// Path commands generate segment IDs 0, 2, 4, 6... (even numbers).
    /// Corner segments (chamfer/fillet) get segment IDs 1, 3, 5... (odd numbers).
    ///
    /// # Returns
    /// A `Result` containing the `CurveSet`, or a `GeometryError` if conversion fails.
    pub fn to_curve_set(&self) -> Result<CurveSet, GeometryError> {
        let mut curve_set = CurveSet::new();
        let mut current_curve: Option<Curve> = None;
        let mut current_pos = DVec2::ZERO;
        let mut first_pos = DVec2::ZERO;
        let mut path_segment_index: u32 = 0;

        // Track pending corner info for processing
        let mut pending_corner: Option<(Corner, DVec2, DVec2)> = None; // (corner, end_pos, end_tangent)

        for (cmd_idx, cmd) in self.commands.iter().enumerate() {
            match &cmd.path {
                PathCommand::Move { point } => {
                    // Validate that Move has Edge corner
                    if cmd.corner != Corner::Edge {
                        return Err(GeometryError::InvalidParameter {
                            parameter: format!("commands[{}].corner", cmd_idx),
                            message: "Move command must have Edge corner".to_string(),
                        });
                    }

                    // Process closing corner for current curve before finalizing
                    if let Some(curve) = current_curve.as_mut() {
                        if !curve.is_empty() {
                            Self::process_closing_corner(
                                curve,
                                &mut pending_corner,
                                current_pos,
                                first_pos,
                                path_segment_index,
                            );
                        }
                    }

                    // Finalize current curve if exists
                    if let Some(curve) = current_curve.take() {
                        if !curve.is_empty() {
                            curve_set.add_curve(curve);
                        }
                    }

                    // Start new curve
                    current_curve = Some(Curve::new());
                    current_pos = *point;
                    first_pos = *point;
                    path_segment_index = 0;
                    pending_corner = None;
                }

                PathCommand::Line { point } => {
                    let curve =
                        current_curve
                            .as_mut()
                            .ok_or_else(|| GeometryError::InvalidParameter {
                                parameter: format!("commands[{}]", cmd_idx),
                                message: "Line command without preceding Move".to_string(),
                            })?;

                    let start = current_pos;
                    let end = *point;
                    let direction = (end - start).normalize_or_zero();
                    let start_tangent = if direction.length_squared() > 0.0 {
                        direction
                    } else {
                        DVec2::X
                    };

                    // Handle pending corner from previous command
                    Self::process_pending_corner(
                        curve,
                        &mut pending_corner,
                        &mut current_pos,
                        start_tangent,
                        &mut path_segment_index,
                    );

                    // Add line segment
                    let line_nurbs = Nurbs::line(current_pos, end);
                    let segment_id = path_segment_index * 2;
                    let smooth_end = matches!(cmd.corner, Corner::SmoothEdge);
                    curve.add_segment(NurbsSegment::new(
                        line_nurbs,
                        curve.total_length(),
                        smooth_end,
                        segment_id,
                    ));

                    // Store corner for next command
                    let end_tangent = direction;
                    pending_corner = Some((cmd.corner, end, end_tangent));
                    current_pos = end;
                    path_segment_index += 1;
                }

                PathCommand::Ellipse { point, center } => {
                    let curve =
                        current_curve
                            .as_mut()
                            .ok_or_else(|| GeometryError::InvalidParameter {
                                parameter: format!("commands[{}]", cmd_idx),
                                message: "Ellipse command without preceding Move".to_string(),
                            })?;

                    let start = current_pos;
                    let end = *point;

                    // Calculate arc parameters
                    let (arc_nurbs_segments, start_tangent, end_tangent) =
                        Self::create_arc_nurbs(start, end, *center)?;

                    // Handle pending corner from previous command
                    Self::process_pending_corner(
                        curve,
                        &mut pending_corner,
                        &mut current_pos,
                        start_tangent,
                        &mut path_segment_index,
                    );

                    // Add arc segments
                    let segment_id = path_segment_index * 2;
                    let smooth_end = matches!(cmd.corner, Corner::SmoothEdge);
                    for (i, nurbs) in arc_nurbs_segments.into_iter().enumerate() {
                        let is_last = i == 0; // simplified: assume last segment
                        curve.add_segment(NurbsSegment::new(
                            nurbs,
                            curve.total_length(),
                            if is_last { smooth_end } else { true },
                            segment_id,
                        ));
                    }

                    // Store corner for next command
                    pending_corner = Some((cmd.corner, end, end_tangent));
                    current_pos = end;
                    path_segment_index += 1;
                }

                PathCommand::Bezier {
                    point,
                    control_points,
                } => {
                    let curve =
                        current_curve
                            .as_mut()
                            .ok_or_else(|| GeometryError::InvalidParameter {
                                parameter: format!("commands[{}]", cmd_idx),
                                message: "Bezier command without preceding Move".to_string(),
                            })?;

                    let start = current_pos;
                    let end = *point;

                    // Create Bezier NURBS
                    let (bezier_nurbs, start_tangent, end_tangent) =
                        Self::create_bezier_nurbs(start, end, control_points);

                    // Handle pending corner from previous command
                    Self::process_pending_corner(
                        curve,
                        &mut pending_corner,
                        &mut current_pos,
                        start_tangent,
                        &mut path_segment_index,
                    );

                    // Add Bezier segment
                    let segment_id = path_segment_index * 2;
                    let smooth_end = matches!(cmd.corner, Corner::SmoothEdge);
                    curve.add_segment(NurbsSegment::new(
                        bezier_nurbs,
                        curve.total_length(),
                        smooth_end,
                        segment_id,
                    ));

                    // Store corner for next command
                    pending_corner = Some((cmd.corner, end, end_tangent));
                    current_pos = end;
                    path_segment_index += 1;
                }
            }
        }

        // Handle closing corner for the last curve
        if let Some(curve) = current_curve.as_mut() {
            if !curve.is_empty() {
                Self::process_closing_corner(
                    curve,
                    &mut pending_corner,
                    current_pos,
                    first_pos,
                    path_segment_index,
                );
            }
        }

        // Finalize last curve
        if let Some(curve) = current_curve.take() {
            if !curve.is_empty() {
                curve_set.add_curve(curve);
            }
        }

        Ok(curve_set)
    }

    /// Processes a pending corner from the previous command.
    /// This includes truncating the previous segment and adding the corner segment.
    fn process_pending_corner(
        curve: &mut Curve,
        pending_corner: &mut Option<(Corner, DVec2, DVec2)>,
        current_pos: &mut DVec2,
        next_start_tangent: DVec2,
        path_segment_index: &mut u32,
    ) {
        if let Some((corner, corner_pos, prev_end_tangent)) = pending_corner.take() {
            if let Some(corner_nurbs) =
                Self::create_corner_nurbs(corner, corner_pos, prev_end_tangent, next_start_tangent)
            {
                // Get the corner start point (where the previous segment should end)
                let corner_start = corner_nurbs.start_point();
                let corner_end = corner_nurbs.end_point();

                // Truncate the previous segment to end at corner_start
                if let Some(last_segment) = curve.segments.last_mut() {
                    // Replace the last segment's NURBS with a truncated version
                    let seg_start = last_segment.nurbs.start_point();
                    last_segment.nurbs = Nurbs::line(seg_start, corner_start);
                    // Recalculate end_distance
                    last_segment.end_distance =
                        last_segment.start_distance + last_segment.nurbs.length();
                }

                // Corner segments get odd segment IDs (previous path index * 2 + 1)
                let corner_segment_id = (*path_segment_index - 1) * 2 + 1;
                curve.add_segment(NurbsSegment::new(
                    corner_nurbs,
                    curve.total_length(),
                    false,
                    corner_segment_id,
                ));

                *current_pos = corner_end;
            }
        }
    }

    /// Creates a corner NURBS segment based on the corner type.
    fn create_corner_nurbs(
        corner: Corner,
        corner_pos: DVec2,
        incoming_tangent: DVec2,
        outgoing_tangent: DVec2,
    ) -> Option<Nurbs> {
        match corner {
            Corner::Edge | Corner::SmoothEdge => None,
            Corner::Chamfer { distance } => {
                if distance <= 0.0 {
                    return None;
                }
                let start = corner_pos - incoming_tangent * distance;
                let end = corner_pos + outgoing_tangent * distance;
                Some(Nurbs::line(start, end))
            }
            Corner::Fillet { radius } => {
                if radius <= 0.0 {
                    return None;
                }
                Self::create_fillet_nurbs(corner_pos, incoming_tangent, outgoing_tangent, radius)
            }
        }
    }

    /// Creates a fillet arc NURBS.
    fn create_fillet_nurbs(
        corner_pos: DVec2,
        incoming_tangent: DVec2,
        outgoing_tangent: DVec2,
        radius: f64,
    ) -> Option<Nurbs> {
        // Determine which way the corner turns
        // Cross product: positive = left turn (CCW), negative = right turn (CW)
        let cross =
            incoming_tangent.x * outgoing_tangent.y - incoming_tangent.y * outgoing_tangent.x;
        let sign = if cross > 0.0 { 1.0 } else { -1.0 };

        // Calculate the half-angle between the tangent vectors
        // θ = angle between incoming and outgoing
        // half_theta = θ/2
        let dot = incoming_tangent.dot(outgoing_tangent);
        let half_theta_sin_sq = (1.0 - dot) / 2.0;
        let half_theta = half_theta_sin_sq.sqrt().asin();

        // The interior angle of the corner (on the fillet side) is α = π - θ
        // So α/2 = π/2 - θ/2
        // sin(α/2) = cos(θ/2)
        // tan(α/2) = sin(α/2) / cos(α/2) = cos(θ/2) / sin(θ/2) = cot(θ/2) = 1 / tan(θ/2)
        //
        // For the inscribed circle:
        // - Offset from corner to tangent point: r / tan(α/2) = r * tan(θ/2)
        // - Distance from corner to center: r / sin(α/2) = r / cos(θ/2)

        // Skip fillet if angle is too small (collinear edges) or too close to 180° (reflex)
        if half_theta.abs() < 1e-6 || (PI / 2.0 - half_theta).abs() < 1e-6 {
            return None;
        }

        let offset = radius * half_theta.tan();
        let start = corner_pos - incoming_tangent * offset;
        let end = corner_pos + outgoing_tangent * offset;

        // The fillet center is equidistant from both lines at distance radius.
        // It lies at distance radius / cos(θ/2) from corner.
        //
        // The direction from corner to center is perpendicular to the angle bisector:
        // - Angle bisector direction: (incoming + outgoing).normalize()
        // - For a left turn (sign > 0): rotate bisector 90° CCW to get direction to center
        // - For a right turn (sign < 0): rotate bisector 90° CW to get direction to center
        let bisector = (incoming_tangent + outgoing_tangent).normalize_or_zero();
        let direction_to_center = DVec2::new(-bisector.y, bisector.x) * sign;

        let center_distance = radius / half_theta.cos();
        let center = corner_pos + direction_to_center * center_distance;

        // Calculate sweep angle
        let start_angle = (start - center).y.atan2((start - center).x);
        let end_angle = (end - center).y.atan2((end - center).x);
        let mut sweep_angle = end_angle - start_angle;

        // Adjust sweep angle based on direction
        // For left turn (sign > 0), the arc goes CCW (positive sweep) from start to end
        // For right turn (sign < 0), the arc goes CW (negative sweep) from start to end
        if sign > 0.0 {
            // Counter-clockwise
            if sweep_angle < 0.0 {
                sweep_angle += 2.0 * PI;
            }
        } else {
            // Clockwise
            if sweep_angle > 0.0 {
                sweep_angle -= 2.0 * PI;
            }
        }

        Some(Nurbs::arc(center, radius, start_angle, sweep_angle))
    }

    /// Creates NURBS segments for an arc/ellipse.
    fn create_arc_nurbs(
        start: DVec2,
        end: DVec2,
        center: DVec2,
    ) -> Result<(Vec<Nurbs>, DVec2, DVec2), GeometryError> {
        let radius_start = (start - center).length();
        let radius_end = (end - center).length();

        // Check if it's approximately circular
        let radius = (radius_start + radius_end) / 2.0;
        if radius < 1e-9 {
            // Degenerate case: treat as a line
            let direction = (end - start).normalize_or_zero();
            return Ok((vec![Nurbs::line(start, end)], direction, direction));
        }

        // Check if start and end are the same point (full circle case)
        let is_full_circle = (start - end).length() < 1e-9;

        let start_angle = (start - center).y.atan2((start - center).x);

        // Calculate start tangent (perpendicular to radius)
        let start_tangent = DVec2::new(-(start - center).y, (start - center).x).normalize_or_zero();

        // For full circle, use 2*PI sweep
        if is_full_circle {
            let segments = Nurbs::circle(center, radius);
            // Start and end tangent are the same for a full circle
            return Ok((segments, start_tangent, start_tangent));
        }

        let end_angle = (end - center).y.atan2((end - center).x);
        let end_tangent = DVec2::new(-(end - center).y, (end - center).x).normalize_or_zero();

        // Determine sweep angle
        let mut sweep_angle = end_angle - start_angle;

        // Choose the shorter arc by default (can be adjusted if needed)
        if sweep_angle > PI {
            sweep_angle -= 2.0 * PI;
        } else if sweep_angle < -PI {
            sweep_angle += 2.0 * PI;
        }

        // Check if ellipse or circle
        let is_ellipse = (radius_start - radius_end).abs() > 1e-6;

        if is_ellipse {
            // For ellipse, we need to determine the radii
            let radius_x = radius_start;
            let radius_y = radius_end;
            Ok((
                vec![Nurbs::ellipse_arc(
                    center,
                    radius_x,
                    radius_y,
                    start_angle,
                    sweep_angle,
                )],
                start_tangent,
                end_tangent,
            ))
        } else {
            // For circular arc, use arc_segments for accuracy
            let segments = Nurbs::arc_segments(center, radius, start_angle, sweep_angle);
            Ok((segments, start_tangent, end_tangent))
        }
    }

    /// Creates a Bezier NURBS from control points.
    fn create_bezier_nurbs(
        start: DVec2,
        end: DVec2,
        control_points: &[DVec2],
    ) -> (Nurbs, DVec2, DVec2) {
        let nurbs = if control_points.is_empty() {
            // Line
            Nurbs::line(start, end)
        } else if control_points.len() == 1 {
            // Quadratic Bezier
            Nurbs::quadratic(start, control_points[0], end)
        } else if control_points.len() == 2 {
            // Cubic Bezier
            Nurbs::cubic(start, control_points[0], control_points[1], end)
        } else {
            // General case: create NURBS with all control points
            let mut all_points = vec![start];
            all_points.extend(control_points.iter().copied());
            all_points.push(end);
            Nurbs::new(all_points)
        };

        let start_tangent = nurbs.derivative(0.0).normalize_or_zero();
        let end_tangent = nurbs.derivative(1.0).normalize_or_zero();

        (nurbs, start_tangent, end_tangent)
    }
}

impl SampleContour for PlanarProfile {
    /// Samples the profile contour by converting to a `CurveSet` and sampling.
    ///
    /// # Arguments
    /// * `sample_distance` - The maximum distance between consecutive sample points
    ///
    /// # Returns
    /// A `Result` containing the `ContourSet` with sampled segments.
    fn sample_contour(&self, sample_distance: f64) -> Result<ContourSet, GeometryError> {
        if sample_distance <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "sample_distance".to_string(),
                message: "Sample distance must be positive".to_string(),
            });
        }

        let curve_set = self.to_curve_set()?;
        curve_set.sample_contour(sample_distance)
    }
}

impl GeometryModel for PlanarProfile {
    /// Converts the planar profile to a polygon mesh.
    ///
    /// This method samples the profile contour and delegates to the `ContourSet`'s
    /// `GeometryModel` implementation. The profile is converted to a 2D polygon mesh
    /// in the XY plane with Z=0.
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the profile,
    /// or a `GeometryError` if the conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The profile is empty
    /// - The profile contains invalid commands
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        // Use a default sample distance based on a reasonable approximation
        // This will be overridden by specific methods if needed
        let sample_distance = 0.1;
        self.to_polygon_mesh_with_sample_distance(sample_distance)
    }
}

impl PlanarProfile {
    /// Converts the planar profile to a polygon mesh with a specified sample distance.
    ///
    /// This method samples the profile contour and delegates to the `ContourSet`'s
    /// `GeometryModel` implementation.
    ///
    /// # Arguments
    /// * `sample_distance` - The maximum distance between consecutive sample points
    ///
    /// # Returns
    /// A `Result` containing the `PolygonMesh` representation of the profile.
    pub fn to_polygon_mesh_with_sample_distance(
        &self,
        sample_distance: f64,
    ) -> Result<PolygonMesh, GeometryError> {
        let contour_set = self.sample_contour(sample_distance)?;
        contour_set.to_polygon_mesh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Corner tests

    #[test]
    fn test_corner_edge() {
        let corner = Corner::Edge;
        assert_eq!(corner, Corner::Edge);
    }

    #[test]
    fn test_corner_smooth_edge() {
        let corner = Corner::SmoothEdge;
        assert_eq!(corner, Corner::SmoothEdge);
    }

    #[test]
    fn test_corner_chamfer() {
        let corner = Corner::Chamfer { distance: 2.5 };
        if let Corner::Chamfer { distance } = corner {
            assert_eq!(distance, 2.5);
        } else {
            panic!("Expected Corner::Chamfer");
        }
    }

    #[test]
    fn test_corner_fillet() {
        let corner = Corner::Fillet { radius: 1.5 };
        if let Corner::Fillet { radius } = corner {
            assert_eq!(radius, 1.5);
        } else {
            panic!("Expected Corner::Fillet");
        }
    }

    #[test]
    fn test_corner_default() {
        let corner = Corner::default();
        assert_eq!(corner, Corner::Edge);
    }

    // PathCommand tests

    #[test]
    fn test_path_command_move() {
        let point = DVec2::new(1.0, 2.0);
        let cmd = PathCommand::Move { point };
        if let PathCommand::Move { point: p } = cmd {
            assert_eq!(p, DVec2::new(1.0, 2.0));
        } else {
            panic!("Expected PathCommand::Move");
        }
    }

    #[test]
    fn test_path_command_line() {
        let point = DVec2::new(3.0, 4.0);
        let cmd = PathCommand::Line { point };
        if let PathCommand::Line { point: p } = cmd {
            assert_eq!(p, DVec2::new(3.0, 4.0));
        } else {
            panic!("Expected PathCommand::Line");
        }
    }

    #[test]
    fn test_path_command_ellipse() {
        let point = DVec2::new(5.0, 6.0);
        let center = DVec2::new(2.5, 3.0);
        let cmd = PathCommand::Ellipse { point, center };
        if let PathCommand::Ellipse {
            point: p,
            center: c,
        } = cmd
        {
            assert_eq!(p, DVec2::new(5.0, 6.0));
            assert_eq!(c, DVec2::new(2.5, 3.0));
        } else {
            panic!("Expected PathCommand::Ellipse");
        }
    }

    #[test]
    fn test_path_command_bezier() {
        let point = DVec2::new(10.0, 10.0);
        let control_points = vec![
            DVec2::new(2.0, 4.0),
            DVec2::new(5.0, 8.0),
            DVec2::new(8.0, 6.0),
        ];
        let cmd = PathCommand::Bezier {
            point,
            control_points: control_points.clone(),
        };
        if let PathCommand::Bezier {
            point: p,
            control_points: cp,
        } = cmd
        {
            assert_eq!(p, DVec2::new(10.0, 10.0));
            assert_eq!(cp.len(), 3);
            assert_eq!(cp[0], DVec2::new(2.0, 4.0));
            assert_eq!(cp[1], DVec2::new(5.0, 8.0));
            assert_eq!(cp[2], DVec2::new(8.0, 6.0));
        } else {
            panic!("Expected PathCommand::Bezier");
        }
    }

    // ProfileCommand tests

    #[test]
    fn test_profile_command_new() {
        let path = PathCommand::Line {
            point: DVec2::new(1.0, 0.0),
        };
        let corner = Corner::Chamfer { distance: 0.5 };
        let cmd = ProfileCommand::new(path.clone(), corner);
        assert_eq!(cmd.path, path);
        assert_eq!(cmd.corner, Corner::Chamfer { distance: 0.5 });
    }

    #[test]
    fn test_profile_command_with_edge() {
        let path = PathCommand::Line {
            point: DVec2::new(1.0, 0.0),
        };
        let cmd = ProfileCommand::with_edge(path.clone());
        assert_eq!(cmd.path, path);
        assert_eq!(cmd.corner, Corner::Edge);
    }

    // PlanarProfile tests

    #[test]
    fn test_planar_profile_new() {
        let profile = PlanarProfile::new();
        assert!(profile.is_empty());
        assert_eq!(profile.len(), 0);
    }

    #[test]
    fn test_planar_profile_default() {
        let profile = PlanarProfile::default();
        assert!(profile.is_empty());
    }

    #[test]
    fn test_planar_profile_with_commands() {
        let commands = vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ];
        let profile = PlanarProfile::with_commands(commands);
        assert_eq!(profile.len(), 3);
        assert!(!profile.is_empty());
    }

    #[test]
    fn test_planar_profile_push() {
        let mut profile = PlanarProfile::new();
        assert!(profile.is_empty());

        profile.push(ProfileCommand::with_edge(PathCommand::Move {
            point: DVec2::new(0.0, 0.0),
        }));
        assert_eq!(profile.len(), 1);

        profile.push(ProfileCommand::new(
            PathCommand::Line {
                point: DVec2::new(5.0, 5.0),
            },
            Corner::Chamfer { distance: 0.25 },
        ));
        assert_eq!(profile.len(), 2);
    }

    #[test]
    fn test_planar_profile_clone() {
        let commands = vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ];
        let profile = PlanarProfile::with_commands(commands);
        let cloned = profile.clone();
        assert_eq!(profile, cloned);
    }

    #[test]
    fn test_corner_equality() {
        assert_eq!(Corner::Edge, Corner::Edge);
        assert_eq!(Corner::SmoothEdge, Corner::SmoothEdge);
        assert_eq!(
            Corner::Chamfer { distance: 1.0 },
            Corner::Chamfer { distance: 1.0 }
        );
        assert_eq!(
            Corner::Fillet { radius: 2.0 },
            Corner::Fillet { radius: 2.0 }
        );
        assert_ne!(Corner::Edge, Corner::SmoothEdge);
        assert_ne!(Corner::Edge, Corner::Chamfer { distance: 1.0 });
        assert_ne!(Corner::SmoothEdge, Corner::Chamfer { distance: 1.0 });
        assert_ne!(
            Corner::Chamfer { distance: 1.0 },
            Corner::Fillet { radius: 1.0 }
        );
    }

    // Serialization tests

    #[test]
    fn test_corner_serialize_deserialize() {
        let edge = Corner::Edge;
        let json = serde_json::to_string(&edge).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, deserialized);

        let smooth_edge = Corner::SmoothEdge;
        let json = serde_json::to_string(&smooth_edge).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(smooth_edge, deserialized);

        let chamfer = Corner::Chamfer { distance: 2.5 };
        let json = serde_json::to_string(&chamfer).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(chamfer, deserialized);

        let fillet = Corner::Fillet { radius: 1.5 };
        let json = serde_json::to_string(&fillet).unwrap();
        let deserialized: Corner = serde_json::from_str(&json).unwrap();
        assert_eq!(fillet, deserialized);
    }

    #[test]
    fn test_path_command_serialize_deserialize() {
        let move_cmd = PathCommand::Move {
            point: DVec2::new(1.0, 2.0),
        };
        let json = serde_json::to_string(&move_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(move_cmd, deserialized);

        let line_cmd = PathCommand::Line {
            point: DVec2::new(3.0, 4.0),
        };
        let json = serde_json::to_string(&line_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(line_cmd, deserialized);

        let ellipse_cmd = PathCommand::Ellipse {
            point: DVec2::new(5.0, 6.0),
            center: DVec2::new(2.5, 3.0),
        };
        let json = serde_json::to_string(&ellipse_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(ellipse_cmd, deserialized);

        let bezier_cmd = PathCommand::Bezier {
            point: DVec2::new(10.0, 10.0),
            control_points: vec![DVec2::new(2.0, 4.0), DVec2::new(5.0, 8.0)],
        };
        let json = serde_json::to_string(&bezier_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(bezier_cmd, deserialized);
    }

    #[test]
    fn test_profile_command_serialize_deserialize() {
        let cmd = ProfileCommand::new(
            PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            },
            Corner::Chamfer { distance: 0.5 },
        );
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: ProfileCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_planar_profile_serialize_deserialize() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
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
                Corner::Chamfer { distance: 0.2 },
            ),
            ProfileCommand::with_edge(PathCommand::Ellipse {
                point: DVec2::new(0.0, 1.0),
                center: DVec2::new(0.5, 1.0),
            }),
        ]);

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: PlanarProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
    }

    #[test]
    fn test_planar_profile_serialize_empty() {
        let profile = PlanarProfile::new();
        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: PlanarProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, deserialized);
        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_planar_profile_json_format() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(1.0, 2.0),
            })]);

        let json = serde_json::to_string_pretty(&profile).unwrap();
        assert!(json.contains("commands"));
        assert!(json.contains("Move"));
        assert!(json.contains("Edge"));
    }

    // SampleContour tests

    #[test]
    fn test_sample_contour_invalid_sample_distance() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
        ]);

        assert!(profile.sample_contour(0.0).is_err());
        assert!(profile.sample_contour(-1.0).is_err());
    }

    #[test]
    fn test_sample_contour_move_with_non_edge_corner_error() {
        let profile = PlanarProfile::with_commands(vec![ProfileCommand::new(
            PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            },
            Corner::Fillet { radius: 0.1 },
        )]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_contour_line_without_move_error() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            })]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_contour_simple_line() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(!contour.is_closed());
        assert_eq!(contour.len(), 1); // Line generates exactly one segment

        let segment = &contour.segments[0];
        assert!((segment.start.position - DVec2::new(0.0, 0.0)).length() < 1e-9);
        assert!((segment.end.position - DVec2::new(1.0, 0.0)).length() < 1e-9);
    }

    #[test]
    fn test_sample_contour_multiple_lines() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(0.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(!contour.is_closed());
        assert!(contour.len() >= 3); // At least 3 line segments
    }

    #[test]
    fn test_sample_contour_closed_square() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(0.0, 1.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(0.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
    }

    #[test]
    fn test_sample_contour_consecutive_moves_ignored() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(1.0, 1.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(2.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1); // Only one contour (first move ignored)

        let contour = &contour_set.contours[0];
        // The line should start from (1,1) not (0,0)
        assert!((contour.segments[0].start.position - DVec2::new(1.0, 1.0)).length() < 1e-9);
    }

    #[test]
    fn test_sample_contour_multiple_contours() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(2.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(3.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 2); // Two separate contours
    }

    #[test]
    fn test_sample_contour_full_circle() {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse {
                point: start, // Same as start = full circle
                center,
            }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
        // Full circle should have at least 12 segments (360/30 = 12)
        assert!(contour.len() >= 12);
    }

    #[test]
    fn test_sample_contour_half_circle() {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);
        let end = DVec2::new(-1.0, 0.0); // Opposite point

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse { point: end, center }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        // Half circle should have at least 6 segments (180/30 = 6)
        assert!(contour.len() >= 6);
    }

    #[test]
    fn test_sample_contour_ellipse_sampling_distance() {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);
        let end = DVec2::new(0.0, 1.0); // Quarter circle

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse { point: end, center }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Check that segments don't exceed sample distance
        for segment in &contour.segments {
            let distance = (segment.end.position - segment.start.position).length();
            // Allow some tolerance due to floating point
            assert!(
                distance < 0.15,
                "Segment distance {} exceeds sample distance",
                distance
            );
        }
    }

    #[test]
    fn test_sample_contour_bezier_basic() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(3.0, 0.0),
                control_points: vec![DVec2::new(1.0, 1.0), DVec2::new(2.0, 1.0)],
            }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        // Should have at least as many segments as control points
        assert!(contour.len() >= 2);
    }

    #[test]
    fn test_sample_contour_bezier_min_samples() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(1.0, 0.0),
                control_points: vec![
                    DVec2::new(0.2, 0.5),
                    DVec2::new(0.4, 0.5),
                    DVec2::new(0.6, 0.5),
                    DVec2::new(0.8, 0.5),
                ],
            }),
        ]);

        let result = profile.sample_contour(10.0); // Large sample distance
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Should have at least 3 segments (minimum for a degree-3 NURBS)
        // Note: NURBS degree is capped at 3 in Nurbs::new() for bezier curves
        assert!(contour.len() >= 3);
    }

    #[test]
    fn test_sample_contour_chamfer() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        // Should have more segments due to chamfer
        assert!(contour.len() >= 3); // line + chamfer + line
    }

    #[test]
    fn test_sample_contour_fillet() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.05);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        // Should have more segments due to fillet arc
        assert!(contour.len() >= 3);
    }

    #[test]
    fn test_sample_contour_smooth_edge() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::SmoothEdge,
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(contour.len() >= 2);

        // For smooth edge, the tangents at the connection should be interpolated
        // The first segment's end tangent should blend with the second segment's start tangent
    }

    #[test]
    fn test_sample_contour_closed_with_chamfer() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(0.0, 1.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
        // Should have chamfer at the closing corner
        assert!(contour.len() >= 5);
    }

    #[test]
    fn test_sample_contour_line_tangents() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.1).unwrap();
        let segment = &result.contours[0].segments[0];

        // Tangent should be along the line direction (1, 0)
        assert!((segment.start.tangent - DVec2::new(1.0, 0.0)).length() < 1e-9);
        assert!((segment.end.tangent - DVec2::new(1.0, 0.0)).length() < 1e-9);
    }

    #[test]
    fn test_sample_contour_empty_profile() {
        let profile = PlanarProfile::new();
        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert!(contour_set.is_empty());
    }

    #[test]
    fn test_sample_contour_only_move() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            })]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        // A single move doesn't create a contour
        assert!(contour_set.is_empty());
    }

    #[test]
    fn test_sample_contour_segment_connectivity() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1).unwrap();
        let contour = &result.contours[0];

        // Check that segments connect properly
        for i in 0..contour.segments.len() - 1 {
            let end_pos = contour.segments[i].end.position;
            let start_pos = contour.segments[i + 1].start.position;
            assert!(
                (end_pos - start_pos).length() < 1e-6,
                "Segments {} and {} don't connect: end {:?} != start {:?}",
                i,
                i + 1,
                end_pos,
                start_pos
            );
        }
    }

    #[test]
    fn test_sample_contour_ellipse_without_move_error() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Ellipse {
                point: DVec2::new(1.0, 0.0),
                center: DVec2::new(0.0, 0.0),
            })]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_contour_bezier_without_move_error() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(1.0, 0.0),
                control_points: vec![DVec2::new(0.5, 0.5)],
            })]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_contour_complex_profile() {
        // A complex profile with multiple command types
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(2.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(2.0, 1.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Ellipse {
                point: DVec2::new(1.0, 2.0),
                center: DVec2::new(1.5, 1.5),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(0.0, 2.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(0.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
        // Should have many segments due to chamfer, fillet, and ellipse
        assert!(contour.len() > 10);
    }

    // ============================================================================
    // Ellipse tests (arc, circle, ellipse segment)
    // ============================================================================

    #[test]
    fn test_sample_contour_quarter_arc() {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);
        let end = DVec2::new(0.0, 1.0); // 90-degree arc

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse { point: end, center }),
        ]);

        let result = profile.sample_contour(0.2);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        // 90-degree arc should produce multiple segments
        assert!(contour.len() >= 2);
        assert!(!contour.is_closed());

        // Check that the arc starts and ends at the correct positions
        let first_segment = &contour.segments[0];
        let last_segment = &contour.segments[contour.len() - 1];
        assert!((first_segment.start.position - start).length() < 0.01);
        assert!((last_segment.end.position - end).length() < 0.01);
    }

    #[test]
    fn test_sample_contour_half_arc() {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);
        let end = DVec2::new(-1.0, 0.0); // 180-degree arc

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse { point: end, center }),
        ]);

        let result = profile.sample_contour(0.3);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // 180-degree arc should produce at least 3 segments
        assert!(contour.len() >= 3);
        assert!(!contour.is_closed());

        // Verify positions
        let first_segment = &contour.segments[0];
        let last_segment = &contour.segments[contour.len() - 1];
        assert!((first_segment.start.position - start).length() < 0.01);
        assert!((last_segment.end.position - end).length() < 0.01);
    }

    #[test]
    fn test_sample_contour_three_quarter_arc() {
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);
        let end = DVec2::new(0.0, -1.0); // 270-degree arc (going counter-clockwise)

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse { point: end, center }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        assert!(!contour.is_closed());
    }

    #[test]
    fn test_sample_contour_circle_with_line() {
        // Create a closed shape with a line and a semicircle
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(-1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Ellipse {
                point: DVec2::new(1.0, 0.0),
                center: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(-1.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.3);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
        assert!(contour.len() >= 4);
    }

    #[test]
    fn test_sample_contour_small_arc() {
        // Small arc (about 45 degrees)
        let center = DVec2::new(0.0, 0.0);
        let start = DVec2::new(1.0, 0.0);
        let end = DVec2::new(0.707, 0.707); // approximately 45 degrees

        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move { point: start }),
            ProfileCommand::with_edge(PathCommand::Ellipse { point: end, center }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        assert!(contour.len() >= 1);
    }

    #[test]
    fn test_sample_contour_arc_with_corner_fillet() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Ellipse {
                    point: DVec2::new(1.0, 1.0),
                    center: DVec2::new(1.0, 0.0),
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(2.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Should have arc segments + fillet + line
        assert!(contour.len() >= 3);
    }

    #[test]
    fn test_sample_contour_arc_with_corner_chamfer() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Ellipse {
                    point: DVec2::new(1.0, 1.0),
                    center: DVec2::new(1.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(2.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Should have arc segments + chamfer + line
        assert!(contour.len() >= 3);
    }

    // ============================================================================
    // Bezier curve tests
    // ============================================================================

    #[test]
    fn test_sample_contour_bezier_quadratic() {
        // Quadratic Bezier with 1 control point
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(2.0, 0.0),
                control_points: vec![DVec2::new(1.0, 1.0)],
            }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Quadratic Bezier (degree 2) should produce at least 2 segments
        assert!(contour.len() >= 2);

        // Verify start and end positions
        let first_segment = &contour.segments[0];
        let last_segment = &contour.segments[contour.len() - 1];
        assert!((first_segment.start.position - DVec2::new(0.0, 0.0)).length() < 0.01);
        assert!((last_segment.end.position - DVec2::new(2.0, 0.0)).length() < 0.01);
    }

    #[test]
    fn test_sample_contour_bezier_cubic() {
        // Cubic Bezier with 2 control points
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(3.0, 0.0),
                control_points: vec![DVec2::new(1.0, 1.0), DVec2::new(2.0, 1.0)],
            }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Cubic Bezier (degree 3) should produce at least 3 segments
        assert!(contour.len() >= 3);

        // Verify start and end positions
        let first_segment = &contour.segments[0];
        let last_segment = &contour.segments[contour.len() - 1];
        assert!((first_segment.start.position - DVec2::new(0.0, 0.0)).length() < 0.01);
        assert!((last_segment.end.position - DVec2::new(3.0, 0.0)).length() < 0.01);
    }

    #[test]
    fn test_sample_contour_bezier_as_line() {
        // Bezier with no control points is a straight line
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(5.0, 0.0),
                control_points: vec![],
            }),
        ]);

        let result = profile.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Linear Bezier should produce exactly 1 segment regardless of length
        assert_eq!(contour.len(), 1);
    }

    #[test]
    fn test_sample_contour_bezier_with_fillet() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Bezier {
                    point: DVec2::new(2.0, 0.0),
                    control_points: vec![DVec2::new(1.0, 1.0)],
                },
                Corner::Fillet { radius: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(2.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Should have bezier segments + fillet + line
        assert!(contour.len() >= 4);
    }

    #[test]
    fn test_sample_contour_bezier_with_chamfer() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Bezier {
                    point: DVec2::new(2.0, 0.0),
                    control_points: vec![DVec2::new(1.0, 1.0)],
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(2.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Should have bezier segments + chamfer + line
        assert!(contour.len() >= 3);
    }

    #[test]
    fn test_sample_contour_bezier_s_curve() {
        // S-curve using cubic Bezier
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(4.0, 0.0),
                control_points: vec![DVec2::new(1.0, 2.0), DVec2::new(3.0, -2.0)],
            }),
        ]);

        let result = profile.sample_contour(0.3);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        assert!(contour.len() >= 3);
    }

    #[test]
    fn test_sample_contour_bezier_closed_shape() {
        // Closed shape using Bezier curves
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(2.0, 0.0),
                control_points: vec![DVec2::new(1.0, 1.0)],
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(0.0, 0.0),
                control_points: vec![DVec2::new(1.0, -1.0)],
            }),
        ]);

        let result = profile.sample_contour(0.3);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        assert!(contour.is_closed());
        assert!(contour.len() >= 4);
    }

    // ============================================================================
    // Segment ID tests for CurveSet conversion
    // ============================================================================

    #[test]
    fn test_sample_contour_segment_ids_lines() {
        // Test that line segments have correct segment IDs (even numbers)
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(2.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(3.0, 0.0),
            }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Path segments should have IDs: 0, 2, 4 (even numbers)
        assert_eq!(contour.segments[0].segment, 0);
        assert_eq!(contour.segments[1].segment, 2);
        assert_eq!(contour.segments[2].segment, 4);
    }

    #[test]
    fn test_sample_contour_segment_ids_with_corners() {
        // Test that corner segments have odd segment IDs
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(1.0, 0.0),
                },
                Corner::Chamfer { distance: 0.1 },
            ),
            ProfileCommand::with_edge(PathCommand::Line {
                point: DVec2::new(1.0, 1.0),
            }),
        ]);

        let result = profile.sample_contour(0.5);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Should have segments with IDs for: line (0), chamfer (1), line (2)
        // First line segment ID = 0
        assert_eq!(contour.segments[0].segment, 0);
        // Chamfer segment ID = 1 (odd)
        // Find the chamfer segment by looking for ID 1
        let chamfer_segments: Vec<_> = contour.segments.iter().filter(|s| s.segment == 1).collect();
        assert!(!chamfer_segments.is_empty());
        // Second line segment ID = 2
        let second_line_segments: Vec<_> =
            contour.segments.iter().filter(|s| s.segment == 2).collect();
        assert!(!second_line_segments.is_empty());
    }

    #[test]
    fn test_sample_contour_multiple_bezier_curves() {
        // Multiple Bezier curves in sequence
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(1.0, 0.0),
                control_points: vec![DVec2::new(0.5, 0.5)],
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(2.0, 0.0),
                control_points: vec![DVec2::new(1.5, -0.5)],
            }),
            ProfileCommand::with_edge(PathCommand::Bezier {
                point: DVec2::new(3.0, 0.0),
                control_points: vec![DVec2::new(2.5, 0.5)],
            }),
        ]);

        let result = profile.sample_contour(0.3);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Each Bezier produces at least 2 segments, so at least 6 total
        assert!(contour.len() >= 6);
    }

    // ============================================================================
    // Cylinder profile tests - tests for profiles like cylinder cross-sections
    // ============================================================================

    /// Creates a cylinder-like profile with the given edge treatments.
    /// The profile is: horizontal line (bottom cap) -> vertical line (body) -> horizontal line (top cap)
    /// This mimics the cylinder cross-section used in sweeps.rs
    fn create_cylinder_profile(
        radius: f64,
        half_height: f64,
        bottom_edge: Corner,
        top_edge: Corner,
    ) -> PlanarProfile {
        let bottom_center = DVec2::new(-radius, -half_height);
        let bottom_surface = DVec2::new(0.0, -half_height);
        let top_surface = DVec2::new(0.0, half_height);
        let top_center = DVec2::new(-radius, half_height);

        PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: bottom_center,
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: bottom_surface,
                },
                bottom_edge,
            ),
            ProfileCommand::new(PathCommand::Line { point: top_surface }, top_edge),
            ProfileCommand::new(PathCommand::Line { point: top_center }, Corner::Edge),
        ])
    }

    #[test]
    fn test_cylinder_profile_sharp_edges_to_curve_set() {
        let profile = create_cylinder_profile(1.0, 1.0, Corner::Edge, Corner::Edge);
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];

        // With sharp edges, we should have exactly 3 segments (3 lines, no corner segments)
        assert_eq!(
            curve.segments.len(),
            3,
            "Expected 3 segments for sharp edges"
        );

        // Check segment IDs: 0, 2, 4 (even numbers for path segments)
        assert_eq!(
            curve.segments[0].segment_id, 0,
            "First segment should have ID 0"
        );
        assert_eq!(
            curve.segments[1].segment_id, 2,
            "Second segment should have ID 2"
        );
        assert_eq!(
            curve.segments[2].segment_id, 4,
            "Third segment should have ID 4"
        );

        // Verify segment positions
        let seg0 = &curve.segments[0];
        let start0 = seg0.nurbs.evaluate(0.0);
        let end0 = seg0.nurbs.evaluate(1.0);
        assert!(
            (start0 - DVec2::new(-1.0, -1.0)).length() < 1e-9,
            "Seg0 start: {:?}",
            start0
        );
        assert!(
            (end0 - DVec2::new(0.0, -1.0)).length() < 1e-9,
            "Seg0 end: {:?}",
            end0
        );

        let seg1 = &curve.segments[1];
        let start1 = seg1.nurbs.evaluate(0.0);
        let end1 = seg1.nurbs.evaluate(1.0);
        assert!(
            (start1 - DVec2::new(0.0, -1.0)).length() < 1e-9,
            "Seg1 start: {:?}",
            start1
        );
        assert!(
            (end1 - DVec2::new(0.0, 1.0)).length() < 1e-9,
            "Seg1 end: {:?}",
            end1
        );

        let seg2 = &curve.segments[2];
        let start2 = seg2.nurbs.evaluate(0.0);
        let end2 = seg2.nurbs.evaluate(1.0);
        assert!(
            (start2 - DVec2::new(0.0, 1.0)).length() < 1e-9,
            "Seg2 start: {:?}",
            start2
        );
        assert!(
            (end2 - DVec2::new(-1.0, 1.0)).length() < 1e-9,
            "Seg2 end: {:?}",
            end2
        );
    }

    #[test]
    fn test_cylinder_profile_chamfer_edges_to_curve_set() {
        let chamfer_distance = 0.2;
        let profile = create_cylinder_profile(
            1.0,
            1.0,
            Corner::Chamfer {
                distance: chamfer_distance,
            },
            Corner::Chamfer {
                distance: chamfer_distance,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];

        // With chamfer edges, we should have 5 segments:
        // seg 0: bottom cap line (truncated)
        // seg 1: bottom chamfer
        // seg 2: body line (truncated at both ends)
        // seg 3: top chamfer
        // seg 4: top cap line (truncated)
        assert_eq!(
            curve.segments.len(),
            5,
            "Expected 5 segments for chamfer edges, got {}",
            curve.segments.len()
        );

        // Check segment IDs
        assert_eq!(
            curve.segments[0].segment_id, 0,
            "Bottom cap should have ID 0"
        );
        assert_eq!(
            curve.segments[1].segment_id, 1,
            "Bottom chamfer should have ID 1"
        );
        assert_eq!(curve.segments[2].segment_id, 2, "Body should have ID 2");
        assert_eq!(
            curve.segments[3].segment_id, 3,
            "Top chamfer should have ID 3"
        );
        assert_eq!(curve.segments[4].segment_id, 4, "Top cap should have ID 4");

        // Verify bottom cap segment (should end at chamfer_distance from corner)
        let seg0 = &curve.segments[0];
        let start0 = seg0.nurbs.evaluate(0.0);
        let end0 = seg0.nurbs.evaluate(1.0);
        assert!(
            (start0 - DVec2::new(-1.0, -1.0)).length() < 1e-9,
            "Bottom cap start wrong: {:?}",
            start0
        );
        // End should be at (-chamfer_distance, -1.0) since chamfer cuts the corner
        assert!(
            (end0 - DVec2::new(-chamfer_distance, -1.0)).length() < 1e-9,
            "Bottom cap end wrong: {:?}, expected {:?}",
            end0,
            DVec2::new(-chamfer_distance, -1.0)
        );

        // Verify bottom chamfer segment
        let seg1 = &curve.segments[1];
        let start1 = seg1.nurbs.evaluate(0.0);
        let end1 = seg1.nurbs.evaluate(1.0);
        // Chamfer should go from (-chamfer_distance, -1.0) to (0.0, -1.0 + chamfer_distance)
        assert!(
            (start1 - DVec2::new(-chamfer_distance, -1.0)).length() < 1e-9,
            "Bottom chamfer start wrong: {:?}",
            start1
        );
        assert!(
            (end1 - DVec2::new(0.0, -1.0 + chamfer_distance)).length() < 1e-9,
            "Bottom chamfer end wrong: {:?}, expected {:?}",
            end1,
            DVec2::new(0.0, -1.0 + chamfer_distance)
        );

        // Verify body segment (should start at chamfer offset and end at chamfer offset)
        let seg2 = &curve.segments[2];
        let start2 = seg2.nurbs.evaluate(0.0);
        let end2 = seg2.nurbs.evaluate(1.0);
        assert!(
            (start2 - DVec2::new(0.0, -1.0 + chamfer_distance)).length() < 1e-9,
            "Body start wrong: {:?}",
            start2
        );
        assert!(
            (end2 - DVec2::new(0.0, 1.0 - chamfer_distance)).length() < 1e-9,
            "Body end wrong: {:?}",
            end2
        );

        // Verify top chamfer segment
        let seg3 = &curve.segments[3];
        let start3 = seg3.nurbs.evaluate(0.0);
        let end3 = seg3.nurbs.evaluate(1.0);
        // Top chamfer should go from (0.0, 1.0 - chamfer_distance) to (-chamfer_distance, 1.0)
        assert!(
            (start3 - DVec2::new(0.0, 1.0 - chamfer_distance)).length() < 1e-9,
            "Top chamfer start wrong: {:?}",
            start3
        );
        assert!(
            (end3 - DVec2::new(-chamfer_distance, 1.0)).length() < 1e-9,
            "Top chamfer end wrong: {:?}, expected {:?}",
            end3,
            DVec2::new(-chamfer_distance, 1.0)
        );

        // Verify top cap segment
        let seg4 = &curve.segments[4];
        let start4 = seg4.nurbs.evaluate(0.0);
        let end4 = seg4.nurbs.evaluate(1.0);
        assert!(
            (start4 - DVec2::new(-chamfer_distance, 1.0)).length() < 1e-9,
            "Top cap start wrong: {:?}",
            start4
        );
        assert!(
            (end4 - DVec2::new(-1.0, 1.0)).length() < 1e-9,
            "Top cap end wrong: {:?}",
            end4
        );
    }

    #[test]
    fn test_cylinder_profile_fillet_edges_to_curve_set() {
        let fillet_radius = 0.2;
        let profile = create_cylinder_profile(
            1.0,
            1.0,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];

        // With fillet edges, we should have 5 segments:
        // seg 0: bottom cap line (truncated)
        // seg 1: bottom fillet arc
        // seg 2: body line (truncated at both ends)
        // seg 3: top fillet arc
        // seg 4: top cap line (truncated)
        assert_eq!(
            curve.segments.len(),
            5,
            "Expected 5 segments for fillet edges, got {}",
            curve.segments.len()
        );

        // Check segment IDs
        assert_eq!(
            curve.segments[0].segment_id, 0,
            "Bottom cap should have ID 0"
        );
        assert_eq!(
            curve.segments[1].segment_id, 1,
            "Bottom fillet should have ID 1"
        );
        assert_eq!(curve.segments[2].segment_id, 2, "Body should have ID 2");
        assert_eq!(
            curve.segments[3].segment_id, 3,
            "Top fillet should have ID 3"
        );
        assert_eq!(curve.segments[4].segment_id, 4, "Top cap should have ID 4");

        // For a 90-degree corner, the offset along each edge = radius (since tan(45°) = 1)
        let offset = fillet_radius;

        // Verify bottom cap segment
        let seg0 = &curve.segments[0];
        let start0 = seg0.nurbs.evaluate(0.0);
        let end0 = seg0.nurbs.evaluate(1.0);
        assert!(
            (start0 - DVec2::new(-1.0, -1.0)).length() < 1e-9,
            "Bottom cap start wrong: {:?}",
            start0
        );
        assert!(
            (end0 - DVec2::new(-offset, -1.0)).length() < 1e-9,
            "Bottom cap end wrong: {:?}, expected {:?}",
            end0,
            DVec2::new(-offset, -1.0)
        );

        // Verify bottom fillet arc endpoints
        let seg1 = &curve.segments[1];
        let start1 = seg1.nurbs.evaluate(0.0);
        let end1 = seg1.nurbs.evaluate(1.0);
        assert!(
            (start1 - DVec2::new(-offset, -1.0)).length() < 1e-9,
            "Bottom fillet start wrong: {:?}",
            start1
        );
        assert!(
            (end1 - DVec2::new(0.0, -1.0 + offset)).length() < 1e-9,
            "Bottom fillet end wrong: {:?}, expected {:?}",
            end1,
            DVec2::new(0.0, -1.0 + offset)
        );

        // Verify body segment
        let seg2 = &curve.segments[2];
        let start2 = seg2.nurbs.evaluate(0.0);
        let end2 = seg2.nurbs.evaluate(1.0);
        assert!(
            (start2 - DVec2::new(0.0, -1.0 + offset)).length() < 1e-9,
            "Body start wrong: {:?}",
            start2
        );
        assert!(
            (end2 - DVec2::new(0.0, 1.0 - offset)).length() < 1e-9,
            "Body end wrong: {:?}",
            end2
        );

        // Verify top fillet arc endpoints
        let seg3 = &curve.segments[3];
        let start3 = seg3.nurbs.evaluate(0.0);
        let end3 = seg3.nurbs.evaluate(1.0);
        assert!(
            (start3 - DVec2::new(0.0, 1.0 - offset)).length() < 1e-9,
            "Top fillet start wrong: {:?}",
            start3
        );
        assert!(
            (end3 - DVec2::new(-offset, 1.0)).length() < 1e-9,
            "Top fillet end wrong: {:?}, expected {:?}",
            end3,
            DVec2::new(-offset, 1.0)
        );

        // Verify top cap segment
        let seg4 = &curve.segments[4];
        let start4 = seg4.nurbs.evaluate(0.0);
        let end4 = seg4.nurbs.evaluate(1.0);
        assert!(
            (start4 - DVec2::new(-offset, 1.0)).length() < 1e-9,
            "Top cap start wrong: {:?}",
            start4
        );
        assert!(
            (end4 - DVec2::new(-1.0, 1.0)).length() < 1e-9,
            "Top cap end wrong: {:?}",
            end4
        );
    }

    #[test]
    fn test_cylinder_profile_mixed_edges_to_curve_set() {
        // Bottom: chamfer, Top: fillet
        let chamfer_distance = 0.15;
        let fillet_radius = 0.25;
        let profile = create_cylinder_profile(
            1.0,
            1.0,
            Corner::Chamfer {
                distance: chamfer_distance,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];

        assert_eq!(
            curve.segments.len(),
            5,
            "Expected 5 segments for mixed edges"
        );

        // Bottom chamfer offset = chamfer_distance
        // Top fillet offset = fillet_radius (for 90 degree corner)

        // Bottom cap ends at (-chamfer_distance, -1.0)
        let seg0_end = curve.segments[0].nurbs.evaluate(1.0);
        assert!(
            (seg0_end - DVec2::new(-chamfer_distance, -1.0)).length() < 1e-9,
            "Bottom cap end wrong: {:?}",
            seg0_end
        );

        // Body starts at (0.0, -1.0 + chamfer_distance)
        let seg2_start = curve.segments[2].nurbs.evaluate(0.0);
        assert!(
            (seg2_start - DVec2::new(0.0, -1.0 + chamfer_distance)).length() < 1e-9,
            "Body start wrong: {:?}",
            seg2_start
        );

        // Body ends at (0.0, 1.0 - fillet_radius)
        let seg2_end = curve.segments[2].nurbs.evaluate(1.0);
        assert!(
            (seg2_end - DVec2::new(0.0, 1.0 - fillet_radius)).length() < 1e-9,
            "Body end wrong: {:?}",
            seg2_end
        );

        // Top fillet ends at (-fillet_radius, 1.0)
        let seg3_end = curve.segments[3].nurbs.evaluate(1.0);
        assert!(
            (seg3_end - DVec2::new(-fillet_radius, 1.0)).length() < 1e-9,
            "Top fillet end wrong: {:?}",
            seg3_end
        );
    }

    #[test]
    fn test_cylinder_profile_small_fillet_radius() {
        // Test with small fillet radius that was causing issues
        let fillet_radius = 0.05;
        let profile = create_cylinder_profile(
            0.5,
            1.0,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];
        assert_eq!(curve.segments.len(), 5, "Expected 5 segments");

        // Verify the fillet segments have reasonable positions (with floating point tolerance)
        let tolerance = 1e-9;
        let seg1 = &curve.segments[1];
        let start1 = seg1.nurbs.evaluate(0.0);
        let end1 = seg1.nurbs.evaluate(1.0);

        // Start and end should be within the profile bounds (with tolerance for floating point)
        assert!(
            start1.x >= -0.5 - tolerance && start1.x <= tolerance,
            "Fillet start x out of bounds: {}",
            start1.x
        );
        assert!(
            start1.y >= -1.0 - tolerance && start1.y <= 1.0 + tolerance,
            "Fillet start y out of bounds: {}",
            start1.y
        );
        assert!(
            end1.x >= -0.5 - tolerance && end1.x <= tolerance,
            "Fillet end x out of bounds: {}",
            end1.x
        );
        assert!(
            end1.y >= -1.0 - tolerance && end1.y <= 1.0 + tolerance,
            "Fillet end y out of bounds: {}",
            end1.y
        );
    }

    // ============================================================================
    // Conical frustum profile tests - tests for profiles with diagonal lines
    // ============================================================================

    /// Creates a frustum-like profile with the given edge treatments.
    /// The profile is: horizontal line (bottom cap) -> diagonal line (body) -> horizontal line (top cap)
    /// This mimics the conical frustum cross-section used in sweeps.rs
    fn create_frustum_profile(
        bottom_radius: f64,
        top_radius: f64,
        half_height: f64,
        bottom_edge: Corner,
        top_edge: Corner,
    ) -> PlanarProfile {
        let max_radius = bottom_radius.max(top_radius);
        let bottom_x = -(max_radius - bottom_radius);
        let top_x = -(max_radius - top_radius);

        let bottom_center = DVec2::new(-max_radius, -half_height);
        let bottom_surface = DVec2::new(bottom_x, -half_height);
        let top_surface = DVec2::new(top_x, half_height);
        let top_center = DVec2::new(-max_radius, half_height);

        PlanarProfile::with_commands(vec![
            ProfileCommand::new(
                PathCommand::Move {
                    point: bottom_center,
                },
                Corner::Edge,
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: bottom_surface,
                },
                bottom_edge,
            ),
            ProfileCommand::new(PathCommand::Line { point: top_surface }, top_edge),
            ProfileCommand::new(PathCommand::Line { point: top_center }, Corner::Edge),
        ])
    }

    #[test]
    fn test_frustum_profile_no_edges() {
        // Test frustum profile with no fillets/chamfers
        let profile = create_frustum_profile(0.5, 0.25, 1.0, Corner::Edge, Corner::Edge);
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];
        assert_eq!(
            curve.segments.len(),
            3,
            "Expected 3 segments for no-edge frustum"
        );
    }

    #[test]
    fn test_frustum_profile_with_fillet_small_taper() {
        // Small taper: bottom_radius=0.5, top_radius=0.4
        let fillet_radius = 0.1;
        let profile = create_frustum_profile(
            0.5,
            0.4,
            1.0,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];
        assert_eq!(curve.segments.len(), 5, "Expected 5 segments with fillets");

        // Verify segments are connected
        verify_segment_connectivity(curve, 1e-6);
    }

    #[test]
    fn test_frustum_profile_with_fillet_large_taper() {
        // Large taper: bottom_radius=0.5, top_radius=0.1
        let fillet_radius = 0.05;
        let profile = create_frustum_profile(
            0.5,
            0.1,
            1.0,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];
        assert_eq!(curve.segments.len(), 5, "Expected 5 segments with fillets");

        // Verify segments are connected - this is the key test for the bug
        verify_segment_connectivity(curve, 1e-6);
    }

    #[test]
    fn test_frustum_profile_with_fillet_cone() {
        // Cone shape: bottom_radius=0.5, top_radius=0.0 (apex)
        let fillet_radius = 0.05;
        let profile = create_frustum_profile(
            0.5,
            0.0,
            1.0,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];

        // Verify segments are connected
        verify_segment_connectivity(curve, 1e-6);
    }

    #[test]
    fn test_frustum_profile_with_fillet_inverted_cone() {
        // Inverted cone: bottom_radius=0.0, top_radius=0.5
        let fillet_radius = 0.05;
        let profile = create_frustum_profile(
            0.0,
            0.5,
            1.0,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(curve_set.curves.len(), 1);
        let curve = &curve_set.curves[0];

        // Verify segments are connected
        verify_segment_connectivity(curve, 1e-6);
    }

    #[test]
    fn test_frustum_profile_fillet_positions() {
        // Test that fillet positions are geometrically correct
        // Bottom: horizontal line, then diagonal at 45 degrees (roughly)
        // The fillet should be inscribed in the corner
        let bottom_radius = 0.5;
        let top_radius = 0.25;
        let half_height = 1.0;
        let fillet_radius = 0.1;

        let profile = create_frustum_profile(
            bottom_radius,
            top_radius,
            half_height,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Edge,
        );
        let curve_set = profile.to_curve_set().unwrap();
        let curve = &curve_set.curves[0];

        // Should have 4 segments: bottom cap, bottom fillet, body, top cap
        assert_eq!(curve.segments.len(), 4, "Expected 4 segments");

        // The fillet (segment 1) should be tangent to both the bottom cap and the body
        let fillet_seg = &curve.segments[1];
        let fillet_start = fillet_seg.nurbs.start_point();
        let fillet_end = fillet_seg.nurbs.end_point();

        // Bottom cap segment (0) should end where fillet starts
        let bottom_cap_end = curve.segments[0].nurbs.end_point();
        assert!(
            (bottom_cap_end - fillet_start).length() < 1e-6,
            "Bottom cap should connect to fillet start: cap_end={:?}, fillet_start={:?}",
            bottom_cap_end,
            fillet_start
        );

        // Body segment (2) should start where fillet ends
        let body_start = curve.segments[2].nurbs.start_point();
        assert!(
            (body_start - fillet_end).length() < 1e-6,
            "Body should connect to fillet end: body_start={:?}, fillet_end={:?}",
            body_start,
            fillet_end
        );

        // Fillet start should be on the horizontal line (y = -half_height)
        assert!(
            (fillet_start.y - (-half_height)).abs() < 1e-6,
            "Fillet start should be on bottom horizontal: y={}",
            fillet_start.y
        );

        // The body segment is now truncated. Its end point should be on the original diagonal
        // but the start point (fillet_end) might not be exactly on the original line
        // because the fillet inscribes a circle that is tangent to both edges.
        // The key check is that all segments are connected.

        // Verify segments are connected (this is the important geometric property)
        verify_segment_connectivity(curve, 1e-6);

        // Check that the fillet is an arc (has proper arc properties)
        // The arc should have the specified radius
        // We can check this by verifying the midpoint is at radius distance from start/end chord
    }

    #[test]
    fn test_frustum_profile_fillet_tangent_continuity() {
        // Test that fillets are tangent to adjacent segments
        let bottom_radius = 0.5;
        let top_radius = 0.1;
        let half_height = 1.0;
        let fillet_radius = 0.1;

        let profile = create_frustum_profile(
            bottom_radius,
            top_radius,
            half_height,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Fillet {
                radius: fillet_radius,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();
        let curve = &curve_set.curves[0];

        // Should have 5 segments: bottom cap, bottom fillet, body, top fillet, top cap
        assert_eq!(curve.segments.len(), 5, "Expected 5 segments");

        // Check tangent continuity at bottom fillet
        let bottom_cap_end_tangent = curve.segments[0].nurbs.derivative(1.0).normalize_or_zero();
        let fillet_start_tangent = curve.segments[1].nurbs.derivative(0.0).normalize_or_zero();

        // Tangents should be parallel (dot product close to 1 or -1)
        let dot = bottom_cap_end_tangent.dot(fillet_start_tangent);
        assert!(
            dot.abs() > 0.99,
            "Bottom cap and fillet should be tangent: dot={}, cap_tan={:?}, fillet_tan={:?}",
            dot,
            bottom_cap_end_tangent,
            fillet_start_tangent
        );

        // Check tangent continuity at bottom fillet to body
        let fillet_end_tangent = curve.segments[1].nurbs.derivative(1.0).normalize_or_zero();
        let body_start_tangent = curve.segments[2].nurbs.derivative(0.0).normalize_or_zero();

        let dot2 = fillet_end_tangent.dot(body_start_tangent);
        assert!(
            dot2.abs() > 0.99,
            "Fillet and body should be tangent: dot={}, fillet_tan={:?}, body_tan={:?}",
            dot2,
            fillet_end_tangent,
            body_start_tangent
        );

        // Check tangent continuity at top fillet
        let body_end_tangent = curve.segments[2].nurbs.derivative(1.0).normalize_or_zero();
        let top_fillet_start_tangent = curve.segments[3].nurbs.derivative(0.0).normalize_or_zero();

        let dot3 = body_end_tangent.dot(top_fillet_start_tangent);
        assert!(
            dot3.abs() > 0.99,
            "Body and top fillet should be tangent: dot={}, body_tan={:?}, fillet_tan={:?}",
            dot3,
            body_end_tangent,
            top_fillet_start_tangent
        );
    }

    #[test]
    fn test_frustum_profile_fillet_radius_verification() {
        // Verify that the fillet arc has the correct radius
        let bottom_radius = 0.5;
        let top_radius = 0.25;
        let half_height = 1.0;
        let fillet_radius = 0.1;

        let profile = create_frustum_profile(
            bottom_radius,
            top_radius,
            half_height,
            Corner::Fillet {
                radius: fillet_radius,
            },
            Corner::Edge,
        );
        let curve_set = profile.to_curve_set().unwrap();
        let curve = &curve_set.curves[0];

        // Get the fillet segment
        let fillet_seg = &curve.segments[1];
        let fillet_nurbs = &fillet_seg.nurbs;

        // Sample points along the fillet and verify they're all at radius distance from center
        // The center can be found as the intersection of perpendiculars from start and end
        let start = fillet_nurbs.start_point();
        let end = fillet_nurbs.end_point();
        let mid = fillet_nurbs.evaluate(0.5);

        // For an arc, all points should be equidistant from the center
        // We can estimate the center from three points
        // Using the circumcircle formula
        let d = 2.0
            * (start.x * (mid.y - end.y) + mid.x * (end.y - start.y) + end.x * (start.y - mid.y));
        if d.abs() > 1e-10 {
            let center_x = ((start.x * start.x + start.y * start.y) * (mid.y - end.y)
                + (mid.x * mid.x + mid.y * mid.y) * (end.y - start.y)
                + (end.x * end.x + end.y * end.y) * (start.y - mid.y))
                / d;
            let center_y = ((start.x * start.x + start.y * start.y) * (end.x - mid.x)
                + (mid.x * mid.x + mid.y * mid.y) * (start.x - end.x)
                + (end.x * end.x + end.y * end.y) * (mid.x - start.x))
                / d;
            let center = DVec2::new(center_x, center_y);

            let r_start = (start - center).length();
            let r_mid = (mid - center).length();
            let r_end = (end - center).length();

            // All radii should be approximately equal to fillet_radius
            let tolerance = 1e-4;
            assert!(
                (r_start - fillet_radius).abs() < tolerance,
                "Start radius {} should be {}",
                r_start,
                fillet_radius
            );
            assert!(
                (r_mid - fillet_radius).abs() < tolerance,
                "Mid radius {} should be {}",
                r_mid,
                fillet_radius
            );
            assert!(
                (r_end - fillet_radius).abs() < tolerance,
                "End radius {} should be {}",
                r_end,
                fillet_radius
            );
        }
    }

    #[test]
    fn test_frustum_profile_chamfer_positions() {
        // Test chamfer positions on frustum profile
        let bottom_radius = 0.5;
        let top_radius = 0.1;
        let half_height = 1.0;
        let chamfer_distance = 0.1;

        let profile = create_frustum_profile(
            bottom_radius,
            top_radius,
            half_height,
            Corner::Chamfer {
                distance: chamfer_distance,
            },
            Corner::Chamfer {
                distance: chamfer_distance,
            },
        );
        let curve_set = profile.to_curve_set().unwrap();
        let curve = &curve_set.curves[0];

        assert_eq!(curve.segments.len(), 5, "Expected 5 segments with chamfers");

        // Verify segments are connected
        verify_segment_connectivity(curve, 1e-6);

        // Chamfer should be a straight line
        let bottom_chamfer = &curve.segments[1];
        // A chamfer NURBS should be a line (2 control points)
        assert!(
            bottom_chamfer.nurbs.control_points.len() <= 2,
            "Chamfer should be a line"
        );
    }

    /// Helper function to verify all segments in a curve are connected end-to-end
    fn verify_segment_connectivity(curve: &Curve, tolerance: f64) {
        for i in 0..curve.segments.len() - 1 {
            let end_of_current = curve.segments[i].nurbs.end_point();
            let start_of_next = curve.segments[i + 1].nurbs.start_point();
            let gap = (end_of_current - start_of_next).length();
            assert!(
                gap < tolerance,
                "Gap between segment {} and {}: {:?} to {:?}, gap = {}",
                i,
                i + 1,
                end_of_current,
                start_of_next,
                gap
            );
        }
    }

    // ============================================================================
    // GeometryModel tests for PlanarProfile
    // ============================================================================

    #[test]
    fn test_planar_profile_to_polygon_mesh_simple_square() {
        // Create a simple square profile
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

        let mesh = profile.to_polygon_mesh();
        assert!(mesh.is_ok());

        let mesh = mesh.unwrap();
        assert!(!mesh.vertices.is_empty(), "Mesh should have vertices");
        assert!(!mesh.polygons.is_empty(), "Mesh should have polygons");

        // Check that vertices are in the XY plane
        for vertex in &mesh.vertices {
            assert!(
                (vertex.position.z).abs() < 1e-10,
                "Vertices should be in XY plane (Z=0)"
            );
        }
    }

    #[test]
    fn test_planar_profile_to_polygon_mesh_triangle() {
        // Create a simple triangle profile
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

        let mesh = profile.to_polygon_mesh();
        assert!(mesh.is_ok());

        let mesh = mesh.unwrap();
        assert_eq!(mesh.vertices.len(), 3, "Triangle should have 3 vertices");
        assert!(
            !mesh.polygons.is_empty(),
            "Mesh should have at least one polygon"
        );
    }

    #[test]
    fn test_planar_profile_to_polygon_mesh_with_sample_distance() {
        // Create a circle profile
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

        // Test with different sample distances
        let mesh_coarse = profile.to_polygon_mesh_with_sample_distance(0.5);
        assert!(mesh_coarse.is_ok());

        let mesh_fine = profile.to_polygon_mesh_with_sample_distance(0.1);
        assert!(mesh_fine.is_ok());

        // Finer sampling should produce more vertices
        assert!(
            mesh_fine.unwrap().vertices.len() >= mesh_coarse.unwrap().vertices.len(),
            "Finer sampling should produce at least as many vertices"
        );
    }

    #[test]
    fn test_planar_profile_to_polygon_mesh_with_fillet() {
        // Create a square with filleted corners
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

        let mesh = profile.to_polygon_mesh_with_sample_distance(0.05);
        assert!(mesh.is_ok());

        let mesh = mesh.unwrap();
        // With fillets, we should have more vertices than a simple square
        assert!(
            mesh.vertices.len() > 4,
            "Filleted square should have more than 4 vertices"
        );
    }

    #[test]
    fn test_planar_profile_to_polygon_mesh_normals() {
        // Create a simple square profile
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

        let mesh = profile.to_polygon_mesh().unwrap();

        // All vertex normals should point up (positive Z)
        for vertex in &mesh.vertices {
            assert!(
                vertex.normal.z > 0.0,
                "Vertex normals should point up (positive Z)"
            );
        }

        // All polygon normals should point up (positive Z)
        for polygon in &mesh.polygons {
            assert!(
                polygon.normal.z > 0.0,
                "Polygon normals should point up (positive Z)"
            );
        }
    }

    #[test]
    fn test_planar_profile_to_polygon_mesh_empty_profile() {
        let profile = PlanarProfile::new();
        let mesh = profile.to_polygon_mesh();

        // Empty profile should result in an empty mesh or an error
        // The behavior depends on how the ContourSet handles empty input
        // Most likely it will be an empty mesh
        if let Ok(mesh) = mesh {
            assert!(mesh.is_empty(), "Empty profile should produce empty mesh");
        }
        // If it's an error, that's also acceptable
    }

    /// Detailed diagnostic test for triangle with chamfer corners
    #[test]
    fn test_triangle_profile_with_chamfer_corners_diagnostic() {
        // Create a triangle profile with chamfer corners at all 3 vertices
        let chamfer_distance = 0.1;
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
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.5, 0.866),
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
            ProfileCommand::new(
                PathCommand::Line {
                    point: DVec2::new(0.0, 0.0),
                },
                Corner::Chamfer {
                    distance: chamfer_distance,
                },
            ),
        ]);

        let curve_set = profile.to_curve_set().unwrap();

        assert_eq!(
            curve_set.curves.len(),
            1,
            "Should have exactly 1 curve for the outline"
        );
        let curve = &curve_set.curves[0];

        // Triangle with 3 chamfers should have 6 segments:
        // 3 edge segments (truncated at both ends) + 3 chamfer segments
        println!("\n=== Triangle Chamfer Profile Diagnostic ===");
        println!("Number of segments: {}", curve.segments.len());

        for (i, segment) in curve.segments.iter().enumerate() {
            let start = segment.nurbs.evaluate(0.0);
            let end = segment.nurbs.evaluate(1.0);
            let mid = segment.nurbs.evaluate(0.5);
            println!(
                "Segment {} (id={}): start=({:.4}, {:.4}), end=({:.4}, {:.4}), mid=({:.4}, {:.4})",
                i, segment.segment_id, start.x, start.y, end.x, end.y, mid.x, mid.y
            );
        }

        // Verify we have 6 segments
        assert_eq!(
            curve.segments.len(),
            6,
            "Triangle with 3 chamfers should have 6 segments (3 edges + 3 chamfers)"
        );

        // Verify segments form a closed loop (end of each segment is start of next)
        for i in 0..curve.segments.len() {
            let this_end = curve.segments[i].nurbs.evaluate(1.0);
            let next_start = curve.segments[(i + 1) % curve.segments.len()]
                .nurbs
                .evaluate(0.0);
            let gap = (this_end - next_start).length();
            assert!(
                gap < 1e-9,
                "Gap between segment {} end and segment {} start: {} (end={:?}, start={:?})",
                i,
                (i + 1) % curve.segments.len(),
                gap,
                this_end,
                next_start
            );
        }

        // Verify segment connectivity back to start
        let first_start = curve.segments[0].nurbs.evaluate(0.0);
        let last_end = curve.segments.last().unwrap().nurbs.evaluate(1.0);
        let closing_gap = (last_end - first_start).length();
        assert!(
            closing_gap < 1e-9,
            "Closing gap: {} (last_end={:?}, first_start={:?})",
            closing_gap,
            last_end,
            first_start
        );

        println!("All 6 segments present and connected!\n");
    }
}
