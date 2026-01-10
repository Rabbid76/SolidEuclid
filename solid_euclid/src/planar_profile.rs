use crate::geometry_error::GeometryError;
use crate::planar_contour::{Contour, ContourSet, Point, SampleContour, Segment};
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
    /// Draw a NURBS curve to a point.
    Nurbs {
        /// The end point of the NURBS curve.
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
}

/// Helper struct to track the state during contour sampling.
struct ContourBuilder {
    /// The current position.
    current_pos: DVec2,
    /// The current tangent.
    current_tangent: DVec2,
    /// The current distance along the contour.
    current_distance: f64,
    /// The first point of the current contour.
    first_pos: Option<DVec2>,
    /// The first tangent of the current contour.
    first_tangent: Option<DVec2>,
    /// The current segment index.
    segment_index: u32,
    /// Accumulated segments for the current contour.
    segments: Vec<Segment>,
    /// Pending corner from the previous command.
    pending_corner: Option<Corner>,
    /// Pending end position for corner processing.
    pending_end_pos: Option<DVec2>,
    /// Pending end tangent for corner processing.
    pending_end_tangent: Option<DVec2>,
}

impl ContourBuilder {
    fn new() -> Self {
        Self {
            current_pos: DVec2::ZERO,
            current_tangent: DVec2::X,
            current_distance: 0.0,
            first_pos: None,
            first_tangent: None,
            segment_index: 0,
            segments: Vec::new(),
            pending_corner: None,
            pending_end_pos: None,
            pending_end_tangent: None,
        }
    }

    fn start_at(&mut self, pos: DVec2) {
        self.current_pos = pos;
        self.current_distance = 0.0;
        self.first_pos = Some(pos);
        self.first_tangent = None;
        self.segment_index = 0;
        self.segments.clear();
        self.pending_corner = None;
        self.pending_end_pos = None;
        self.pending_end_tangent = None;
    }

    fn add_segment(
        &mut self,
        start_pos: DVec2,
        start_tangent: DVec2,
        end_pos: DVec2,
        end_tangent: DVec2,
        smooth_end: bool,
    ) {
        let segment_length = (end_pos - start_pos).length();
        let start_distance = self.current_distance;
        let end_distance = start_distance + segment_length;

        let segment = Segment::new(
            Point::new(start_pos, start_tangent, start_distance),
            Point::new(end_pos, end_tangent, end_distance),
            self.segment_index,
            smooth_end,
        );
        self.segments.push(segment);

        // Set first tangent if this is the first segment
        if self.first_tangent.is_none() {
            self.first_tangent = Some(start_tangent);
        }

        self.current_pos = end_pos;
        self.current_tangent = end_tangent;
        self.current_distance = end_distance;
    }

    fn next_segment(&mut self) {
        self.segment_index += 1;
    }

    fn finalize_contour(&mut self, force_closed: Option<bool>) -> Option<Contour> {
        if self.segments.is_empty() {
            return None;
        }

        // Check if closed
        let is_closed = force_closed.unwrap_or_else(|| {
            if let Some(first_pos) = self.first_pos {
                (self.current_pos - first_pos).length() < 1e-9
            } else {
                false
            }
        });

        let contour = Contour::with_segments_and_closed(self.segments.clone(), is_closed);
        self.segments.clear();
        self.first_pos = None;
        self.first_tangent = None;
        Some(contour)
    }
}

/// Samples a line segment.
fn sample_line(start: DVec2, end: DVec2) -> (DVec2, DVec2) {
    let direction = (end - start).normalize_or_zero();
    let tangent = if direction.length_squared() > 0.0 {
        direction
    } else {
        DVec2::X
    };
    (tangent, tangent)
}

/// Calculates the angle for an ellipse point.
fn ellipse_angle(center: DVec2, point: DVec2) -> f64 {
    let delta = point - center;
    delta.y.atan2(delta.x)
}

/// Samples an ellipse/arc.
fn sample_ellipse(
    start: DVec2,
    end: DVec2,
    center: DVec2,
    start_tangent: DVec2,
    sample_distance: f64,
) -> Vec<(DVec2, DVec2)> {
    let radius = (start - center).length();
    if radius < 1e-9 {
        return vec![(end, start_tangent)];
    }

    let start_angle = ellipse_angle(center, start);
    let end_angle = ellipse_angle(center, end);

    // Check if it's a full circle (start == end)
    let is_full_circle = (start - end).length() < 1e-9;

    // Check if it's a half circle (center is on line between start and end)
    let is_half_circle = if !is_full_circle {
        let mid = (start + end) / 2.0;
        (mid - center).length() < 1e-9
    } else {
        false
    };

    // Determine the arc direction based on the start tangent
    let start_to_center = center - start;
    let cross = start_tangent.x * start_to_center.y - start_tangent.y * start_to_center.x;
    let clockwise = cross > 0.0;

    let arc_angle = if is_full_circle {
        2.0 * PI
    } else if is_half_circle {
        PI
    } else {
        let mut angle_diff = end_angle - start_angle;
        if clockwise {
            if angle_diff > 0.0 {
                angle_diff -= 2.0 * PI;
            }
        } else if angle_diff < 0.0 {
            angle_diff += 2.0 * PI;
        }
        angle_diff.abs()
    };

    // Calculate number of samples: at least 1 per 30 degrees, and based on sample_distance
    let arc_length = radius * arc_angle;
    let samples_by_distance = (arc_length / sample_distance).ceil() as usize;
    let samples_by_angle = (arc_angle / (PI / 6.0)).ceil() as usize; // 30 degrees = PI/6
    let num_samples = samples_by_distance.max(samples_by_angle).max(1);

    let mut points = Vec::with_capacity(num_samples);
    let angle_step = if clockwise { -arc_angle } else { arc_angle } / num_samples as f64;

    for i in 1..=num_samples {
        let angle = start_angle + angle_step * i as f64;
        let pos = center + DVec2::new(radius * angle.cos(), radius * angle.sin());

        // Tangent is perpendicular to radius
        let tangent = if clockwise {
            DVec2::new((angle + PI / 2.0).cos(), (angle + PI / 2.0).sin())
        } else {
            DVec2::new((angle - PI / 2.0).cos(), (angle - PI / 2.0).sin())
        };

        points.push((pos, tangent));
    }

    // Ensure the last point is exactly the end point
    if let Some(last) = points.last_mut() {
        last.0 = end;
    }

    points
}

/// Evaluates a cubic Bezier curve at parameter t.
fn bezier_point(p0: DVec2, p1: DVec2, p2: DVec2, p3: DVec2, t: f64) -> DVec2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    p0 * mt3 + p1 * 3.0 * mt2 * t + p2 * 3.0 * mt * t2 + p3 * t3
}

/// Evaluates the derivative of a cubic Bezier curve at parameter t.
fn bezier_tangent(p0: DVec2, p1: DVec2, p2: DVec2, p3: DVec2, t: f64) -> DVec2 {
    let t2 = t * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;

    let tangent = (p1 - p0) * 3.0 * mt2 + (p2 - p1) * 6.0 * mt * t + (p3 - p2) * 3.0 * t2;
    tangent.normalize_or_zero()
}

/// Samples a NURBS curve (simplified as cubic Bezier segments).
fn sample_nurbs(
    start: DVec2,
    end: DVec2,
    control_points: &[DVec2],
    sample_distance: f64,
) -> Vec<(DVec2, DVec2)> {
    if control_points.is_empty() {
        // Just a line
        let tangent = (end - start).normalize_or_zero();
        return vec![(end, tangent)];
    }

    // Build a simple cubic Bezier from start through control points to end
    let mut all_points = vec![start];
    all_points.extend(control_points.iter().copied());
    all_points.push(end);

    // Minimum samples = number of control points
    let min_samples = control_points.len();

    // Estimate curve length for sample count
    let mut estimated_length = 0.0;
    for i in 0..all_points.len() - 1 {
        estimated_length += (all_points[i + 1] - all_points[i]).length();
    }
    let samples_by_distance = (estimated_length / sample_distance).ceil() as usize;
    let num_samples = samples_by_distance.max(min_samples).max(1);

    let mut result = Vec::with_capacity(num_samples);

    // For simplicity, use a Catmull-Rom like interpolation or cubic Bezier
    // Here we'll use a simple approach with cubic Bezier if we have exactly 2 control points
    if control_points.len() == 2 {
        let p0 = start;
        let p1 = control_points[0];
        let p2 = control_points[1];
        let p3 = end;

        for i in 1..=num_samples {
            let t = i as f64 / num_samples as f64;
            let pos = bezier_point(p0, p1, p2, p3, t);
            let tangent = bezier_tangent(p0, p1, p2, p3, t);
            result.push((pos, tangent));
        }
    } else {
        // General case: use quadratic Bezier segments or linear interpolation
        // For now, use linear interpolation through all points
        let total_segments = all_points.len() - 1;
        let samples_per_segment = (num_samples / total_segments).max(1);

        for seg in 0..total_segments {
            let p0 = all_points[seg];
            let p1 = all_points[seg + 1];
            let tangent = (p1 - p0).normalize_or_zero();

            let seg_samples = if seg == total_segments - 1 {
                num_samples - result.len()
            } else {
                samples_per_segment
            };

            for i in 1..=seg_samples {
                let t = i as f64 / seg_samples as f64;
                let pos = p0.lerp(p1, t);
                result.push((pos, tangent));
            }
        }
    }

    // Ensure the last point is exactly the end
    if let Some(last) = result.last_mut() {
        last.0 = end;
    }

    result
}

/// Creates a chamfer between two segments.
fn create_chamfer(
    corner_pos: DVec2,
    incoming_tangent: DVec2,
    outgoing_tangent: DVec2,
    distance: f64,
) -> Option<(DVec2, DVec2, DVec2, DVec2)> {
    if distance <= 0.0 {
        return None;
    }

    let start = corner_pos - incoming_tangent * distance;
    let end = corner_pos + outgoing_tangent * distance;
    let chamfer_tangent = (end - start).normalize_or_zero();

    Some((start, incoming_tangent, end, chamfer_tangent))
}

/// Creates a fillet arc between two segments.
fn create_fillet(
    corner_pos: DVec2,
    incoming_tangent: DVec2,
    outgoing_tangent: DVec2,
    radius: f64,
    sample_distance: f64,
) -> Vec<(DVec2, DVec2, DVec2, DVec2)> {
    if radius <= 0.0 {
        return vec![];
    }

    // Calculate the fillet center
    let incoming_normal = DVec2::new(-incoming_tangent.y, incoming_tangent.x);
    let _outgoing_normal = DVec2::new(-outgoing_tangent.y, outgoing_tangent.x);

    // Determine which side the center should be on
    let cross = incoming_tangent.x * outgoing_tangent.y - incoming_tangent.y * outgoing_tangent.x;
    let sign = if cross > 0.0 { 1.0 } else { -1.0 };

    let n1 = incoming_normal * sign;

    // Find the fillet start and end points
    let half_angle = ((1.0 - incoming_tangent.dot(outgoing_tangent)) / 2.0)
        .sqrt()
        .asin();
    if half_angle.abs() < 1e-9 {
        return vec![];
    }

    let offset = radius / half_angle.tan();
    let start = corner_pos - incoming_tangent * offset;
    let end = corner_pos + outgoing_tangent * offset;
    let center = start + n1 * radius;

    // Sample the arc
    let arc_points = sample_ellipse(start, end, center, incoming_tangent, sample_distance);

    let mut result = Vec::new();
    let mut prev_pos = start;
    let mut prev_tangent = incoming_tangent;

    for (pos, tangent) in arc_points {
        result.push((prev_pos, prev_tangent, pos, tangent));
        prev_pos = pos;
        prev_tangent = tangent;
    }

    result
}

/// Interpolates tangents for smooth edges.
fn interpolate_tangent(t1: DVec2, t2: DVec2) -> DVec2 {
    (t1 + t2).normalize_or_zero()
}

impl SampleContour for PlanarProfile {
    fn sample_contour(&self, sample_distance: f64) -> Result<ContourSet, GeometryError> {
        if sample_distance <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "sample_distance".to_string(),
                message: "Sample distance must be positive".to_string(),
            });
        }

        let mut contour_set = ContourSet::new();
        let mut builder = ContourBuilder::new();
        let mut in_contour = false;
        let mut prev_was_move = false;

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

                    // If we were in a contour, finalize it
                    if in_contour && !prev_was_move {
                        if let Some(contour) = builder.finalize_contour(None) {
                            contour_set.add_contour(contour);
                        }
                    }

                    // Start new contour
                    builder.start_at(*point);
                    in_contour = true;
                    prev_was_move = true;
                }

                PathCommand::Line { point } => {
                    if !in_contour {
                        return Err(GeometryError::InvalidParameter {
                            parameter: format!("commands[{}]", cmd_idx),
                            message: "Line command without preceding Move".to_string(),
                        });
                    }

                    prev_was_move = false;
                    let start = builder.current_pos;
                    let end = *point;
                    let (start_tangent, end_tangent) = sample_line(start, end);

                    // Handle pending corner from previous command
                    if let Some(pending_corner) = builder.pending_corner.take() {
                        let pending_end_pos = builder.pending_end_pos.take().unwrap_or(start);
                        let pending_end_tangent =
                            builder.pending_end_tangent.take().unwrap_or(start_tangent);

                        match pending_corner {
                            Corner::Edge => {
                                // Just connect
                            }
                            Corner::SmoothEdge => {
                                // Interpolate tangents - modify the last segment's end tangent
                                if let Some(last_seg) = builder.segments.last_mut() {
                                    let smooth_tangent =
                                        interpolate_tangent(last_seg.end.tangent, start_tangent);
                                    last_seg.end.tangent = smooth_tangent;
                                }
                            }
                            Corner::Chamfer { distance } => {
                                if let Some((chamfer_start, _, chamfer_end, chamfer_tangent)) =
                                    create_chamfer(
                                        pending_end_pos,
                                        pending_end_tangent,
                                        start_tangent,
                                        distance,
                                    )
                                {
                                    // Modify last segment to end at chamfer start
                                    if let Some(last_seg) = builder.segments.last_mut() {
                                        last_seg.end.position = chamfer_start;
                                    }
                                    // Add chamfer segment
                                    builder.next_segment();
                                    builder.add_segment(
                                        chamfer_start,
                                        pending_end_tangent,
                                        chamfer_end,
                                        chamfer_tangent,
                                        false, // chamfer ends at corner, not smooth
                                    );
                                    builder.current_pos = chamfer_end;
                                }
                            }
                            Corner::Fillet { radius } => {
                                let fillet_segments = create_fillet(
                                    pending_end_pos,
                                    pending_end_tangent,
                                    start_tangent,
                                    radius,
                                    sample_distance,
                                );
                                if !fillet_segments.is_empty() {
                                    // Modify last segment to end at fillet start
                                    if let Some(last_seg) = builder.segments.last_mut() {
                                        last_seg.end.position = fillet_segments[0].0;
                                    }
                                    builder.next_segment();
                                    for (i, (s_start, s_start_t, s_end, s_end_t)) in
                                        fillet_segments.iter().enumerate()
                                    {
                                        let is_smooth = i < fillet_segments.len() - 1;
                                        builder.add_segment(
                                            *s_start, *s_start_t, *s_end, *s_end_t, is_smooth,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    builder.next_segment();
                    builder.add_segment(
                        builder.current_pos,
                        start_tangent,
                        end,
                        end_tangent,
                        false,
                    ); // line segment, corner follows

                    // Store corner for next command
                    builder.pending_corner = Some(cmd.corner);
                    builder.pending_end_pos = Some(end);
                    builder.pending_end_tangent = Some(end_tangent);
                }

                PathCommand::Ellipse { point, center } => {
                    if !in_contour {
                        return Err(GeometryError::InvalidParameter {
                            parameter: format!("commands[{}]", cmd_idx),
                            message: "Ellipse command without preceding Move".to_string(),
                        });
                    }

                    prev_was_move = false;
                    let start = builder.current_pos;
                    let end = *point;

                    // Calculate start tangent
                    let start_tangent = if prev_was_move || builder.segments.is_empty() {
                        // Tangent is perpendicular to radius from center
                        let radius_vec = start - *center;
                        DVec2::new(-radius_vec.y, radius_vec.x).normalize_or_zero()
                    } else {
                        builder.current_tangent
                    };

                    // Handle pending corner
                    if let Some(pending_corner) = builder.pending_corner.take() {
                        let pending_end_pos = builder.pending_end_pos.take().unwrap_or(start);
                        let pending_end_tangent =
                            builder.pending_end_tangent.take().unwrap_or(start_tangent);

                        match pending_corner {
                            Corner::Edge => {}
                            Corner::SmoothEdge => {
                                if let Some(last_seg) = builder.segments.last_mut() {
                                    let smooth_tangent =
                                        interpolate_tangent(last_seg.end.tangent, start_tangent);
                                    last_seg.end.tangent = smooth_tangent;
                                }
                            }
                            Corner::Chamfer { distance } => {
                                if let Some((chamfer_start, _, chamfer_end, chamfer_tangent)) =
                                    create_chamfer(
                                        pending_end_pos,
                                        pending_end_tangent,
                                        start_tangent,
                                        distance,
                                    )
                                {
                                    if let Some(last_seg) = builder.segments.last_mut() {
                                        last_seg.end.position = chamfer_start;
                                    }
                                    builder.next_segment();
                                    builder.add_segment(
                                        chamfer_start,
                                        pending_end_tangent,
                                        chamfer_end,
                                        chamfer_tangent,
                                        false, // chamfer ends at corner
                                    );
                                    builder.current_pos = chamfer_end;
                                }
                            }
                            Corner::Fillet { radius } => {
                                let fillet_segments = create_fillet(
                                    pending_end_pos,
                                    pending_end_tangent,
                                    start_tangent,
                                    radius,
                                    sample_distance,
                                );
                                if !fillet_segments.is_empty() {
                                    if let Some(last_seg) = builder.segments.last_mut() {
                                        last_seg.end.position = fillet_segments[0].0;
                                    }
                                    builder.next_segment();
                                    for (i, (s_start, s_start_t, s_end, s_end_t)) in
                                        fillet_segments.iter().enumerate()
                                    {
                                        let is_smooth = i < fillet_segments.len() - 1;
                                        builder.add_segment(
                                            *s_start, *s_start_t, *s_end, *s_end_t, is_smooth,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // Sample the ellipse
                    let ellipse_samples =
                        sample_ellipse(start, end, *center, start_tangent, sample_distance);

                    builder.next_segment();
                    let mut prev_pos = builder.current_pos;
                    let mut prev_tangent = start_tangent;

                    for (i, (pos, tangent)) in ellipse_samples.iter().enumerate() {
                        let is_smooth = i < ellipse_samples.len() - 1; // smooth within ellipse, not at last point
                        builder.add_segment(prev_pos, prev_tangent, *pos, *tangent, is_smooth);
                        prev_pos = *pos;
                        prev_tangent = *tangent;
                    }

                    builder.pending_corner = Some(cmd.corner);
                    builder.pending_end_pos = Some(end);
                    builder.pending_end_tangent = Some(builder.current_tangent);
                }

                PathCommand::Nurbs {
                    point,
                    control_points,
                } => {
                    if !in_contour {
                        return Err(GeometryError::InvalidParameter {
                            parameter: format!("commands[{}]", cmd_idx),
                            message: "Nurbs command without preceding Move".to_string(),
                        });
                    }

                    prev_was_move = false;
                    let start = builder.current_pos;
                    let end = *point;

                    // Handle pending corner
                    let nurbs_samples = sample_nurbs(start, end, control_points, sample_distance);
                    let start_tangent = nurbs_samples.first().map(|(_, t)| *t).unwrap_or(DVec2::X);

                    if let Some(pending_corner) = builder.pending_corner.take() {
                        let pending_end_pos = builder.pending_end_pos.take().unwrap_or(start);
                        let pending_end_tangent =
                            builder.pending_end_tangent.take().unwrap_or(start_tangent);

                        match pending_corner {
                            Corner::Edge => {}
                            Corner::SmoothEdge => {
                                if let Some(last_seg) = builder.segments.last_mut() {
                                    let smooth_tangent =
                                        interpolate_tangent(last_seg.end.tangent, start_tangent);
                                    last_seg.end.tangent = smooth_tangent;
                                }
                            }
                            Corner::Chamfer { distance } => {
                                if let Some((chamfer_start, _, chamfer_end, chamfer_tangent)) =
                                    create_chamfer(
                                        pending_end_pos,
                                        pending_end_tangent,
                                        start_tangent,
                                        distance,
                                    )
                                {
                                    if let Some(last_seg) = builder.segments.last_mut() {
                                        last_seg.end.position = chamfer_start;
                                    }
                                    builder.next_segment();
                                    builder.add_segment(
                                        chamfer_start,
                                        pending_end_tangent,
                                        chamfer_end,
                                        chamfer_tangent,
                                        false, // chamfer ends at corner
                                    );
                                    builder.current_pos = chamfer_end;
                                }
                            }
                            Corner::Fillet { radius } => {
                                let fillet_segments = create_fillet(
                                    pending_end_pos,
                                    pending_end_tangent,
                                    start_tangent,
                                    radius,
                                    sample_distance,
                                );
                                if !fillet_segments.is_empty() {
                                    if let Some(last_seg) = builder.segments.last_mut() {
                                        last_seg.end.position = fillet_segments[0].0;
                                    }
                                    builder.next_segment();
                                    for (i, (s_start, s_start_t, s_end, s_end_t)) in
                                        fillet_segments.iter().enumerate()
                                    {
                                        let is_smooth = i < fillet_segments.len() - 1;
                                        builder.add_segment(
                                            *s_start, *s_start_t, *s_end, *s_end_t, is_smooth,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    builder.next_segment();
                    let mut prev_pos = builder.current_pos;
                    let mut prev_tangent = if !builder.segments.is_empty() {
                        builder.current_tangent
                    } else {
                        start_tangent
                    };

                    for (i, (pos, tangent)) in nurbs_samples.iter().enumerate() {
                        let is_smooth = i < nurbs_samples.len() - 1; // smooth within NURBS, not at last point
                        builder.add_segment(prev_pos, prev_tangent, *pos, *tangent, is_smooth);
                        prev_pos = *pos;
                        prev_tangent = *tangent;
                    }

                    builder.pending_corner = Some(cmd.corner);
                    builder.pending_end_pos = Some(end);
                    builder.pending_end_tangent = Some(builder.current_tangent);
                }
            }
        }

        // Handle closing corner if contour is closed
        if in_contour && !prev_was_move {
            // Check if we need to handle the closing corner
            let mut detected_closed = false;
            if let (Some(pending_corner), Some(first_pos), Some(first_tangent)) = (
                builder.pending_corner.take(),
                builder.first_pos,
                builder.first_tangent,
            ) {
                let is_closed = (builder.current_pos - first_pos).length() < 1e-9;
                detected_closed = is_closed;

                if is_closed && pending_corner != Corner::Edge {
                    let pending_end_pos = builder
                        .pending_end_pos
                        .take()
                        .unwrap_or(builder.current_pos);
                    let pending_end_tangent = builder
                        .pending_end_tangent
                        .take()
                        .unwrap_or(builder.current_tangent);

                    match pending_corner {
                        Corner::Edge => {}
                        Corner::SmoothEdge => {
                            // Interpolate between last segment's end and first segment's start
                            let first_seg_tangent =
                                builder.segments.first().map(|s| s.start.tangent);
                            if let (Some(last_seg), Some(first_tangent_val)) =
                                (builder.segments.last_mut(), first_seg_tangent)
                            {
                                let smooth_tangent =
                                    interpolate_tangent(last_seg.end.tangent, first_tangent_val);
                                last_seg.end.tangent = smooth_tangent;
                            }
                            if let Some(first_seg) = builder.segments.first_mut() {
                                let smooth_tangent = interpolate_tangent(
                                    pending_end_tangent,
                                    first_seg.start.tangent,
                                );
                                first_seg.start.tangent = smooth_tangent;
                            }
                        }
                        Corner::Chamfer { distance } => {
                            if let Some((chamfer_start, _, chamfer_end, chamfer_tangent)) =
                                create_chamfer(
                                    pending_end_pos,
                                    pending_end_tangent,
                                    first_tangent,
                                    distance,
                                )
                            {
                                if let Some(last_seg) = builder.segments.last_mut() {
                                    last_seg.end.position = chamfer_start;
                                }
                                builder.next_segment();
                                builder.add_segment(
                                    chamfer_start,
                                    pending_end_tangent,
                                    chamfer_end,
                                    chamfer_tangent,
                                    false, // closing chamfer ends at corner
                                );
                                // Update first segment start
                                if let Some(first_seg) = builder.segments.first_mut() {
                                    first_seg.start.position = chamfer_end;
                                }
                            }
                        }
                        Corner::Fillet { radius } => {
                            let fillet_segments = create_fillet(
                                pending_end_pos,
                                pending_end_tangent,
                                first_tangent,
                                radius,
                                sample_distance,
                            );
                            if !fillet_segments.is_empty() {
                                if let Some(last_seg) = builder.segments.last_mut() {
                                    last_seg.end.position = fillet_segments[0].0;
                                }
                                builder.next_segment();
                                for (i, (s_start, s_start_t, s_end, s_end_t)) in
                                    fillet_segments.iter().enumerate()
                                {
                                    let is_smooth = i < fillet_segments.len() - 1;
                                    builder.add_segment(
                                        *s_start, *s_start_t, *s_end, *s_end_t, is_smooth,
                                    );
                                }
                                // Update first segment start
                                if let Some(last_fillet) = fillet_segments.last() {
                                    if let Some(first_seg) = builder.segments.first_mut() {
                                        first_seg.start.position = last_fillet.2;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let force_closed = if detected_closed { Some(true) } else { None };
            if let Some(contour) = builder.finalize_contour(force_closed) {
                contour_set.add_contour(contour);
            }
        }

        Ok(contour_set)
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
    fn test_path_command_nurbs() {
        let point = DVec2::new(10.0, 10.0);
        let control_points = vec![
            DVec2::new(2.0, 4.0),
            DVec2::new(5.0, 8.0),
            DVec2::new(8.0, 6.0),
        ];
        let cmd = PathCommand::Nurbs {
            point,
            control_points: control_points.clone(),
        };
        if let PathCommand::Nurbs {
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
            panic!("Expected PathCommand::Nurbs");
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

        let nurbs_cmd = PathCommand::Nurbs {
            point: DVec2::new(10.0, 10.0),
            control_points: vec![DVec2::new(2.0, 4.0), DVec2::new(5.0, 8.0)],
        };
        let json = serde_json::to_string(&nurbs_cmd).unwrap();
        let deserialized: PathCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(nurbs_cmd, deserialized);
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
    fn test_sample_contour_nurbs_basic() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Nurbs {
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
    fn test_sample_contour_nurbs_min_samples() {
        let profile = PlanarProfile::with_commands(vec![
            ProfileCommand::with_edge(PathCommand::Move {
                point: DVec2::new(0.0, 0.0),
            }),
            ProfileCommand::with_edge(PathCommand::Nurbs {
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
        // Should have at least 4 segments (one per control point)
        assert!(contour.len() >= 4);
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
    fn test_sample_contour_nurbs_without_move_error() {
        let profile =
            PlanarProfile::with_commands(vec![ProfileCommand::with_edge(PathCommand::Nurbs {
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
}
