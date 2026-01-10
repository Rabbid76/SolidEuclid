use crate::geometry_error::GeometryError;
use crate::planar_contour::{ContourSet, SampleContour};
use crate::planar_nurbs_curve::{Curve, CurveSet, Nurbs, NurbsSegment};
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
    Bezier {
        /// The end point of the Bezier curve.
        point: DVec2,
        /// The control points defining the curve shape.
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

        // Handle closing corner if the curve is closed
        if let Some(curve) = current_curve.as_mut() {
            if !curve.is_empty() {
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
                        if let Some(corner_nurbs) = Self::create_corner_nurbs(
                            corner,
                            current_pos,
                            end_tangent,
                            first_tangent,
                        ) {
                            let corner_segment_id = path_segment_index * 2 - 1;
                            curve.add_segment(NurbsSegment::new(
                                corner_nurbs,
                                curve.total_length(),
                                false,
                                corner_segment_id,
                            ));
                        }
                    }
                }
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
                // Update current position to corner end before moving corner_nurbs
                let corner_end = corner_nurbs.end_point();

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
        let incoming_normal = DVec2::new(-incoming_tangent.y, incoming_tangent.x);

        // Determine which side the center should be on
        let cross =
            incoming_tangent.x * outgoing_tangent.y - incoming_tangent.y * outgoing_tangent.x;
        let sign = if cross > 0.0 { 1.0 } else { -1.0 };

        let n1 = incoming_normal * sign;

        // Find the fillet start and end points
        let half_angle = ((1.0 - incoming_tangent.dot(outgoing_tangent)) / 2.0)
            .sqrt()
            .asin();
        if half_angle.abs() < 1e-9 {
            return None;
        }

        let offset = radius / half_angle.tan();
        let start = corner_pos - incoming_tangent * offset;
        let end = corner_pos + outgoing_tangent * offset;
        let center = start + n1 * radius;

        // Calculate sweep angle
        let start_angle = (start - center).y.atan2((start - center).x);
        let end_angle = (end - center).y.atan2((end - center).x);
        let mut sweep_angle = end_angle - start_angle;

        // Adjust sweep angle based on direction
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
}
