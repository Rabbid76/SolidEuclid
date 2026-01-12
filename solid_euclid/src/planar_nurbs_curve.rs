use crate::geometry_error::GeometryError;
use crate::planar_contour::{Contour, ContourSet, Point, SampleContour, Segment};
use glam::DVec2;

/// Represents a 2D Non-Uniform Rational B-Spline (NURBS) curve.
///
/// A NURBS curve is defined by a set of control points. The curve interpolates
/// through the first and last control points, while the intermediate control
/// points influence the shape of the curve.
///
/// For a simple line, only 2 points are needed (start and end).
/// For a quadratic Bézier curve, 3 points are needed (start, control, end).
/// For a cubic Bézier curve, 4 points are needed (start, control1, control2, end).
///
/// See: <https://en.wikipedia.org/wiki/Non-uniform_rational_B-spline>
#[derive(Debug, Clone, PartialEq)]
pub struct Nurbs {
    /// The control points of the NURBS curve.
    /// The first point is the start point, the last point is the end point,
    /// and intermediate points are control points that influence the curve shape.
    pub control_points: Vec<DVec2>,

    /// The weights associated with each control point.
    /// Weights influence how strongly each control point affects the curve.
    /// If empty, all weights are assumed to be 1.0 (uniform weights).
    pub weights: Vec<f64>,

    /// The knot vector for the NURBS curve.
    /// If empty, a uniform knot vector is assumed.
    pub knots: Vec<f64>,

    /// The degree of the NURBS curve.
    /// Degree 1 = linear (line segments)
    /// Degree 2 = quadratic
    /// Degree 3 = cubic (most common)
    pub degree: usize,
}

impl Nurbs {
    /// Creates a new NURBS curve with the given control points.
    ///
    /// Uses default degree 3 (cubic) unless fewer control points are provided.
    /// The degree will be adjusted to `control_points.len() - 1` if necessary.
    ///
    /// # Arguments
    /// * `control_points` - The control points defining the curve
    ///
    /// # Panics
    /// Panics if fewer than 2 control points are provided.
    pub fn new(control_points: Vec<DVec2>) -> Self {
        assert!(
            control_points.len() >= 2,
            "NURBS curve requires at least 2 control points"
        );

        let degree = (control_points.len() - 1).min(3);

        Self {
            control_points,
            weights: Vec::new(),
            knots: Vec::new(),
            degree,
        }
    }

    /// Creates a new linear NURBS curve (a line) from start to end point.
    ///
    /// # Arguments
    /// * `start` - The start point of the line
    /// * `end` - The end point of the line
    pub fn line(start: DVec2, end: DVec2) -> Self {
        Self {
            control_points: vec![start, end],
            weights: Vec::new(),
            knots: Vec::new(),
            degree: 1,
        }
    }

    /// Creates a quadratic Bézier curve.
    ///
    /// # Arguments
    /// * `start` - The start point
    /// * `control` - The control point
    /// * `end` - The end point
    pub fn quadratic(start: DVec2, control: DVec2, end: DVec2) -> Self {
        Self {
            control_points: vec![start, control, end],
            weights: Vec::new(),
            knots: Vec::new(),
            degree: 2,
        }
    }

    /// Creates a cubic Bézier curve.
    ///
    /// # Arguments
    /// * `start` - The start point
    /// * `control1` - The first control point
    /// * `control2` - The second control point
    /// * `end` - The end point
    pub fn cubic(start: DVec2, control1: DVec2, control2: DVec2, end: DVec2) -> Self {
        Self {
            control_points: vec![start, control1, control2, end],
            weights: Vec::new(),
            knots: Vec::new(),
            degree: 3,
        }
    }

    /// Creates a NURBS curve with custom degree.
    ///
    /// # Arguments
    /// * `control_points` - The control points defining the curve
    /// * `degree` - The degree of the curve
    ///
    /// # Panics
    /// Panics if degree >= control_points.len() or if fewer than 2 control points.
    pub fn with_degree(control_points: Vec<DVec2>, degree: usize) -> Self {
        assert!(
            control_points.len() >= 2,
            "NURBS curve requires at least 2 control points"
        );
        assert!(
            degree < control_points.len(),
            "Degree must be less than number of control points"
        );

        Self {
            control_points,
            weights: Vec::new(),
            knots: Vec::new(),
            degree,
        }
    }

    /// Creates a NURBS curve with custom weights.
    ///
    /// # Arguments
    /// * `control_points` - The control points defining the curve
    /// * `weights` - The weights for each control point
    /// * `degree` - The degree of the curve
    ///
    /// # Panics
    /// Panics if weights length doesn't match control points length.
    pub fn with_weights(control_points: Vec<DVec2>, weights: Vec<f64>, degree: usize) -> Self {
        assert!(
            control_points.len() >= 2,
            "NURBS curve requires at least 2 control points"
        );
        assert!(
            degree < control_points.len(),
            "Degree must be less than number of control points"
        );
        assert_eq!(
            control_points.len(),
            weights.len(),
            "Weights must have same length as control points"
        );

        Self {
            control_points,
            weights,
            knots: Vec::new(),
            degree,
        }
    }

    /// Returns the start point of the curve.
    pub fn start_point(&self) -> DVec2 {
        self.control_points[0]
    }

    /// Returns the end point of the curve.
    pub fn end_point(&self) -> DVec2 {
        self.control_points[self.control_points.len() - 1]
    }

    /// Evaluates the curve at parameter t (0.0 to 1.0).
    ///
    /// Uses de Casteljau's algorithm for Bézier curves when no custom
    /// knots are specified.
    ///
    /// # Arguments
    /// * `t` - The parameter value, typically in range [0, 1]
    ///
    /// # Returns
    /// The point on the curve at parameter t
    pub fn evaluate(&self, t: f64) -> DVec2 {
        if self.control_points.len() == 2 {
            // Linear interpolation for lines
            return self.control_points[0].lerp(self.control_points[1], t);
        }

        if self.weights.is_empty() && self.knots.is_empty() {
            // Use de Casteljau's algorithm for unweighted Bézier curves
            self.evaluate_bezier(t)
        } else if self.knots.is_empty() {
            // Rational Bézier curve
            self.evaluate_rational_bezier(t)
        } else {
            // Full NURBS evaluation
            self.evaluate_nurbs(t)
        }
    }

    /// Evaluates a Bézier curve using de Casteljau's algorithm.
    fn evaluate_bezier(&self, t: f64) -> DVec2 {
        let mut points = self.control_points.clone();

        while points.len() > 1 {
            let mut new_points = Vec::with_capacity(points.len() - 1);
            for i in 0..points.len() - 1 {
                new_points.push(points[i].lerp(points[i + 1], t));
            }
            points = new_points;
        }

        points[0]
    }

    /// Evaluates a rational Bézier curve.
    fn evaluate_rational_bezier(&self, t: f64) -> DVec2 {
        let n = self.control_points.len() - 1;
        let mut numerator = DVec2::ZERO;
        let mut denominator = 0.0;

        for i in 0..=n {
            let bernstein = Self::bernstein_basis(n, i, t);
            let weight = self.weights[i];
            numerator += self.control_points[i] * bernstein * weight;
            denominator += bernstein * weight;
        }

        numerator / denominator
    }

    /// Evaluates a NURBS curve at parameter t.
    fn evaluate_nurbs(&self, t: f64) -> DVec2 {
        let n = self.control_points.len();
        let p = self.degree;

        // Find the knot span
        let span = self.find_knot_span(t);

        // Calculate basis functions
        let basis = self.basis_functions(span, t);

        let mut numerator = DVec2::ZERO;
        let mut denominator = 0.0;

        for i in 0..=p {
            let idx = span - p + i;
            if idx < n {
                let weight = if self.weights.is_empty() {
                    1.0
                } else {
                    self.weights[idx]
                };
                numerator += self.control_points[idx] * basis[i] * weight;
                denominator += basis[i] * weight;
            }
        }

        if denominator.abs() > f64::EPSILON {
            numerator / denominator
        } else {
            self.control_points[0]
        }
    }

    /// Finds the knot span index for parameter t.
    fn find_knot_span(&self, t: f64) -> usize {
        let n = self.control_points.len() - 1;
        let p = self.degree;

        if t >= self.knots[n + 1] {
            return n;
        }
        if t <= self.knots[p] {
            return p;
        }

        let mut low = p;
        let mut high = n + 1;
        let mut mid = (low + high) / 2;

        while t < self.knots[mid] || t >= self.knots[mid + 1] {
            if t < self.knots[mid] {
                high = mid;
            } else {
                low = mid;
            }
            mid = (low + high) / 2;
        }

        mid
    }

    /// Calculates the B-spline basis functions.
    fn basis_functions(&self, span: usize, t: f64) -> Vec<f64> {
        let p = self.degree;
        let mut n_basis = vec![0.0; p + 1];
        let mut left = vec![0.0; p + 1];
        let mut right = vec![0.0; p + 1];

        n_basis[0] = 1.0;

        for j in 1..=p {
            left[j] = t - self.knots[span + 1 - j];
            right[j] = self.knots[span + j] - t;
            let mut saved = 0.0;

            for r in 0..j {
                let temp = n_basis[r] / (right[r + 1] + left[j - r]);
                n_basis[r] = saved + right[r + 1] * temp;
                saved = left[j - r] * temp;
            }

            n_basis[j] = saved;
        }

        n_basis
    }

    /// Calculates the Bernstein basis polynomial.
    fn bernstein_basis(n: usize, i: usize, t: f64) -> f64 {
        Self::binomial(n, i) as f64 * t.powi(i as i32) * (1.0 - t).powi((n - i) as i32)
    }

    /// Calculates the binomial coefficient "n choose k".
    fn binomial(n: usize, k: usize) -> usize {
        if k > n {
            return 0;
        }
        if k == 0 || k == n {
            return 1;
        }

        let k = k.min(n - k);
        let mut result = 1;
        for i in 0..k {
            result = result * (n - i) / (i + 1);
        }
        result
    }

    /// Evaluates the derivative of the curve at parameter t.
    ///
    /// # Arguments
    /// * `t` - The parameter value, typically in range [0, 1]
    ///
    /// # Returns
    /// The tangent vector at parameter t
    pub fn derivative(&self, t: f64) -> DVec2 {
        if self.control_points.len() == 2 {
            // Constant derivative for lines
            return self.control_points[1] - self.control_points[0];
        }

        // Compute derivative using forward difference
        let h = 1e-6;
        let t0 = (t - h).max(0.0);
        let t1 = (t + h).min(1.0);

        (self.evaluate(t1) - self.evaluate(t0)) / (t1 - t0)
    }

    /// Calculates the mathematical length of the curve.
    ///
    /// For a line (2 control points), this is the exact Euclidean distance.
    /// For curves with more control points, numerical integration is used
    /// with adaptive Gaussian quadrature for accuracy.
    ///
    /// # Returns
    /// The arc length of the curve
    pub fn length(&self) -> f64 {
        if self.control_points.len() == 2 {
            // Exact length for lines
            return self.control_points[0].distance(self.control_points[1]);
        }

        // Use adaptive numerical integration for curves
        // Max depth of 20 prevents infinite recursion with small curves
        // At depth 20, worst case is ~2M iterations which is acceptable
        self.compute_arc_length_adaptive(0.0, 1.0, 1e-10, 20)
    }

    /// Computes arc length using adaptive Simpson's rule with recursion limit.
    fn compute_arc_length_adaptive(&self, a: f64, b: f64, tolerance: f64, depth: u32) -> f64 {
        let mid = (a + b) / 2.0;

        let whole = self.simpsons_rule(a, b);
        let left = self.simpsons_rule(a, mid);
        let right = self.simpsons_rule(mid, b);

        let sum = left + right;

        // Stop recursion if tolerance met OR max depth reached
        if (whole - sum).abs() < 15.0 * tolerance || depth == 0 {
            sum + (sum - whole) / 15.0
        } else {
            self.compute_arc_length_adaptive(a, mid, tolerance / 2.0, depth - 1)
                + self.compute_arc_length_adaptive(mid, b, tolerance / 2.0, depth - 1)
        }
    }

    /// Simpson's rule for arc length integration.
    fn simpsons_rule(&self, a: f64, b: f64) -> f64 {
        let mid = (a + b) / 2.0;
        let h = (b - a) / 6.0;

        let fa = self.derivative(a).length();
        let fm = self.derivative(mid).length();
        let fb = self.derivative(b).length();

        h * (fa + 4.0 * fm + fb)
    }

    /// Calculates the length using a specified number of samples.
    ///
    /// This is a simpler but less accurate method that approximates
    /// the curve with line segments.
    ///
    /// # Arguments
    /// * `samples` - The number of sample points to use
    ///
    /// # Returns
    /// The approximate arc length
    pub fn length_sampled(&self, samples: usize) -> f64 {
        if self.control_points.len() == 2 || samples < 2 {
            return self.control_points[0].distance(self.control_points[1]);
        }

        let mut length = 0.0;
        let mut prev_point = self.evaluate(0.0);

        for i in 1..=samples {
            let t = i as f64 / samples as f64;
            let point = self.evaluate(t);
            length += prev_point.distance(point);
            prev_point = point;
        }

        length
    }
}

impl Default for Nurbs {
    fn default() -> Self {
        Self {
            control_points: vec![DVec2::ZERO, DVec2::X],
            weights: Vec::new(),
            knots: Vec::new(),
            degree: 1,
        }
    }
}

/// Represents a segment of a NURBS curve within a contour.
///
/// A `NurbsSegment` contains a NURBS curve along with distance information
/// relative to the start of a parent contour.
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsSegment {
    /// The NURBS curve for this segment.
    pub nurbs: Nurbs,

    /// The distance from the start of the contour to the start point of this segment.
    pub start_distance: f64,

    /// The distance from the start of the contour to the end point of this segment.
    /// The difference `end_distance - start_distance` equals the length of the curve.
    pub end_distance: f64,

    /// Indicates whether the transition from this segment to the successor segment is smooth.
    /// A smooth transition means the tangent is continuous at the junction point.
    pub smooth_end: bool,

    /// The segment identifier used when sampling this segment into contour segments.
    pub segment_id: u32,
}

impl NurbsSegment {
    /// Creates a new `NurbsSegment` with the given NURBS curve and distance information.
    ///
    /// # Arguments
    /// * `nurbs` - The NURBS curve for this segment
    /// * `start_distance` - Distance from contour start to segment start
    /// * `smooth_end` - Whether the transition to the next segment is smooth
    /// * `segment_id` - The segment identifier used when sampling
    ///
    /// # Returns
    /// A new `NurbsSegment` with `end_distance` automatically calculated from the curve length
    pub fn new(nurbs: Nurbs, start_distance: f64, smooth_end: bool, segment_id: u32) -> Self {
        let length = nurbs.length();
        Self {
            nurbs,
            start_distance,
            end_distance: start_distance + length,
            smooth_end,
            segment_id,
        }
    }

    /// Creates a new `NurbsSegment` with explicit start and end distances.
    ///
    /// # Arguments
    /// * `nurbs` - The NURBS curve for this segment
    /// * `start_distance` - Distance from contour start to segment start
    /// * `end_distance` - Distance from contour start to segment end
    /// * `smooth_end` - Whether the transition to the next segment is smooth
    /// * `segment_id` - The segment identifier used when sampling
    pub fn with_distances(
        nurbs: Nurbs,
        start_distance: f64,
        end_distance: f64,
        smooth_end: bool,
        segment_id: u32,
    ) -> Self {
        Self {
            nurbs,
            start_distance,
            end_distance,
            smooth_end,
            segment_id,
        }
    }

    /// Returns the length of this segment (the arc length of the NURBS curve).
    pub fn length(&self) -> f64 {
        self.end_distance - self.start_distance
    }

    /// Returns the start point of the segment.
    pub fn start_point(&self) -> DVec2 {
        self.nurbs.start_point()
    }

    /// Returns the end point of the segment.
    pub fn end_point(&self) -> DVec2 {
        self.nurbs.end_point()
    }

    /// Evaluates the segment at a given distance from the segment start.
    ///
    /// # Arguments
    /// * `distance` - Distance from the start of this segment
    ///
    /// # Returns
    /// The point on the curve at the given distance
    pub fn evaluate_at_distance(&self, distance: f64) -> DVec2 {
        let segment_length = self.length();
        if segment_length.abs() < f64::EPSILON {
            return self.nurbs.start_point();
        }

        let t = (distance / segment_length).clamp(0.0, 1.0);
        self.nurbs.evaluate(t)
    }

    /// Returns the tangent at a given distance from the segment start.
    ///
    /// # Arguments
    /// * `distance` - Distance from the start of this segment
    ///
    /// # Returns
    /// The tangent vector at the given distance, normalized
    pub fn tangent_at_distance(&self, distance: f64) -> DVec2 {
        let segment_length = self.length();
        if segment_length.abs() < f64::EPSILON {
            return DVec2::X;
        }

        let t = (distance / segment_length).clamp(0.0, 1.0);
        self.nurbs.derivative(t).normalize_or_zero()
    }
}

/// Represents a curve composed of multiple NURBS segments.
///
/// A `Curve` is a sequence of connected NURBS segments that form a continuous path.
/// The segments are ordered and their distance values should be consistent
/// (i.e., each segment's `start_distance` should equal the previous segment's `end_distance`).
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    /// The sequence of NURBS segments that make up this curve.
    pub segments: Vec<NurbsSegment>,
    /// Whether this curve is closed (forms a loop).
    pub closed: bool,
}

impl Curve {
    /// Creates a new empty curve.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            closed: false,
        }
    }

    /// Creates a new curve with the given segments.
    ///
    /// # Arguments
    /// * `segments` - The sequence of NURBS segments
    pub fn with_segments(segments: Vec<NurbsSegment>) -> Self {
        Self {
            segments,
            closed: false,
        }
    }

    /// Creates a new closed curve with the given segments.
    ///
    /// # Arguments
    /// * `segments` - The sequence of NURBS segments
    pub fn with_segments_closed(segments: Vec<NurbsSegment>) -> Self {
        Self {
            segments,
            closed: true,
        }
    }

    /// Sets whether the curve is closed.
    ///
    /// # Arguments
    /// * `closed` - Whether the curve forms a closed loop
    pub fn set_closed(&mut self, closed: bool) {
        self.closed = closed;
    }

    /// Returns whether the curve is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Adds a NURBS curve as a new segment to the end of this curve.
    ///
    /// The segment's `start_distance` is automatically set based on the
    /// current total length of the curve.
    ///
    /// # Arguments
    /// * `nurbs` - The NURBS curve to add
    /// * `smooth_end` - Whether the transition to the next segment should be smooth
    pub fn add_nurbs(&mut self, nurbs: Nurbs, smooth_end: bool) {
        let start_distance = self.total_length();
        let segment_id = self.segments.len() as u32;
        self.segments.push(NurbsSegment::new(
            nurbs,
            start_distance,
            smooth_end,
            segment_id,
        ));
    }

    /// Adds a pre-configured segment to the curve.
    ///
    /// # Arguments
    /// * `segment` - The segment to add
    pub fn add_segment(&mut self, segment: NurbsSegment) {
        self.segments.push(segment);
    }

    /// Returns the total length of the curve (sum of all segment lengths).
    pub fn total_length(&self) -> f64 {
        self.segments.last().map(|s| s.end_distance).unwrap_or(0.0)
    }

    /// Returns the number of segments in the curve.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Returns whether the curve is empty (has no segments).
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns the start point of the curve.
    ///
    /// # Returns
    /// `Some(point)` if the curve has at least one segment, `None` otherwise
    pub fn start_point(&self) -> Option<DVec2> {
        self.segments.first().map(|s| s.start_point())
    }

    /// Returns the end point of the curve.
    ///
    /// # Returns
    /// `Some(point)` if the curve has at least one segment, `None` otherwise
    pub fn end_point(&self) -> Option<DVec2> {
        self.segments.last().map(|s| s.end_point())
    }

    /// Finds the segment that contains a given distance from the curve start.
    ///
    /// # Arguments
    /// * `distance` - Distance from the start of the curve
    ///
    /// # Returns
    /// A tuple of (segment index, reference to segment) if found
    pub fn find_segment_at_distance(&self, distance: f64) -> Option<(usize, &NurbsSegment)> {
        for (i, segment) in self.segments.iter().enumerate() {
            if distance >= segment.start_distance && distance <= segment.end_distance {
                return Some((i, segment));
            }
        }

        // If distance is beyond the last segment, return the last segment
        if distance > 0.0 {
            self.segments.last().map(|s| (self.segments.len() - 1, s))
        } else {
            self.segments.first().map(|s| (0, s))
        }
    }

    /// Evaluates the curve at a given distance from the start.
    ///
    /// # Arguments
    /// * `distance` - Distance from the start of the curve
    ///
    /// # Returns
    /// `Some(point)` if the curve is not empty, `None` otherwise
    pub fn evaluate_at_distance(&self, distance: f64) -> Option<DVec2> {
        self.find_segment_at_distance(distance).map(|(_, segment)| {
            let local_distance = distance - segment.start_distance;
            segment.evaluate_at_distance(local_distance)
        })
    }

    /// Returns the tangent at a given distance from the curve start.
    ///
    /// # Arguments
    /// * `distance` - Distance from the start of the curve
    ///
    /// # Returns
    /// `Some(tangent)` if the curve is not empty, `None` otherwise
    pub fn tangent_at_distance(&self, distance: f64) -> Option<DVec2> {
        self.find_segment_at_distance(distance).map(|(_, segment)| {
            let local_distance = distance - segment.start_distance;
            segment.tangent_at_distance(local_distance)
        })
    }

    /// Samples the curve at regular intervals.
    ///
    /// # Arguments
    /// * `count` - The number of sample points
    ///
    /// # Returns
    /// A vector of sampled points along the curve
    pub fn sample(&self, count: usize) -> Vec<DVec2> {
        if count == 0 || self.is_empty() {
            return Vec::new();
        }

        let total_length = self.total_length();
        if total_length.abs() < f64::EPSILON || count == 1 {
            return self.start_point().map(|p| vec![p]).unwrap_or_default();
        }

        let mut points = Vec::with_capacity(count);
        for i in 0..count {
            let distance = (i as f64 / (count - 1) as f64) * total_length;
            if let Some(point) = self.evaluate_at_distance(distance) {
                points.push(point);
            }
        }

        points
    }
}

impl Default for Curve {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to calculate the number of sample segments for a NURBS curve.
///
/// For linear NURBS (degree 1), returns 1 segment.
/// For higher-order curves, returns max(order - 1, ceil(length / sample_distance)).
fn calculate_nurbs_sample_count(nurbs: &Nurbs, sample_distance: f64) -> usize {
    if nurbs.degree == 1 {
        // Linear NURBS (line) always results in exactly 1 segment
        1
    } else {
        // For higher-order curves: at least (order - 1) = degree segments
        // Order = degree + 1, so minimum segments = degree
        let min_segments = nurbs.degree;
        let length = nurbs.length();
        let length_based_segments = (length / sample_distance).ceil() as usize;
        min_segments.max(length_based_segments).max(1)
    }
}

/// Helper function to sample a single NurbsSegment into contour segments.
///
/// Returns a vector of (Point, Point, smooth_end) tuples representing the sampled segments.
fn sample_nurbs_segment(
    nurbs_segment: &NurbsSegment,
    sample_distance: f64,
    segment_id: u32,
) -> Vec<Segment> {
    let num_samples = calculate_nurbs_sample_count(&nurbs_segment.nurbs, sample_distance);
    let segment_length = nurbs_segment.length();

    let mut segments = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t_start = i as f64 / num_samples as f64;
        let t_end = (i + 1) as f64 / num_samples as f64;

        let local_distance_start = t_start * segment_length;
        let local_distance_end = t_end * segment_length;

        let position_start = nurbs_segment.evaluate_at_distance(local_distance_start);
        let position_end = nurbs_segment.evaluate_at_distance(local_distance_end);

        let tangent_start = nurbs_segment.tangent_at_distance(local_distance_start);
        let tangent_end = nurbs_segment.tangent_at_distance(local_distance_end);

        let distance_start = nurbs_segment.start_distance + local_distance_start;
        let distance_end = nurbs_segment.start_distance + local_distance_end;

        let start_point = Point::new(position_start, tangent_start, distance_start);
        let end_point = Point::new(position_end, tangent_end, distance_end);

        // For the last sample of this NURBS segment, use the segment's smooth_end flag
        // For intermediate samples within the same NURBS, the transition is always smooth
        let is_last_sample = i == num_samples - 1;
        let smooth = if is_last_sample {
            nurbs_segment.smooth_end
        } else {
            true // Intermediate samples within the same NURBS are always smooth
        };

        segments.push(Segment::new(start_point, end_point, segment_id, smooth));
    }

    segments
}

/// Helper function to sample a Curve into a Contour.
fn sample_curve_to_contour(curve: &Curve, sample_distance: f64) -> Result<Contour, GeometryError> {
    if sample_distance <= 0.0 {
        return Err(GeometryError::InvalidParameter {
            parameter: "sample_distance".to_string(),
            message: "Sample distance must be positive".to_string(),
        });
    }

    if curve.segments.is_empty() {
        return Ok(Contour::new());
    }

    let mut contour = Contour::new();

    for nurbs_segment in curve.segments.iter() {
        let sampled_segments =
            sample_nurbs_segment(nurbs_segment, sample_distance, nurbs_segment.segment_id);
        for segment in sampled_segments {
            contour.add_segment(segment);
        }
    }

    // Propagate closed state from curve to contour
    contour.set_closed(curve.closed);

    Ok(contour)
}

impl SampleContour for Curve {
    /// Samples the curve into discrete contour segments.
    ///
    /// # Arguments
    /// * `sample_distance` - The maximum distance between consecutive sample points
    ///
    /// # Returns
    /// A `ContourSet` containing exactly one `Contour` with the sampled segments.
    ///
    /// # Behavior
    /// - Linear NURBS (degree 1) always produce exactly 1 segment
    /// - Higher-order curves produce at least `degree` segments (order - 1)
    /// - The actual number of segments is `max(degree, ceil(length / sample_distance))`
    fn sample_contour(&self, sample_distance: f64) -> Result<ContourSet, GeometryError> {
        let contour = sample_curve_to_contour(self, sample_distance)?;
        Ok(ContourSet::with_contours(vec![contour]))
    }
}

/// Represents a collection of curves.
///
/// A `CurveSet` holds multiple `Curve` objects that together define
/// a complex outline, which may include multiple separate paths.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveSet {
    /// The collection of curves.
    pub curves: Vec<Curve>,
}

impl CurveSet {
    /// Creates a new empty curve set.
    pub fn new() -> Self {
        Self { curves: Vec::new() }
    }

    /// Creates a new curve set with the given curves.
    ///
    /// # Arguments
    /// * `curves` - The collection of curves
    pub fn with_curves(curves: Vec<Curve>) -> Self {
        Self { curves }
    }

    /// Adds a curve to the set.
    ///
    /// # Arguments
    /// * `curve` - The curve to add
    pub fn add_curve(&mut self, curve: Curve) {
        self.curves.push(curve);
    }

    /// Returns the number of curves.
    pub fn len(&self) -> usize {
        self.curves.len()
    }

    /// Returns true if there are no curves.
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
    }

    /// Returns the total length of all curves combined.
    pub fn total_length(&self) -> f64 {
        self.curves.iter().map(|c| c.total_length()).sum()
    }
}

impl Default for CurveSet {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleContour for CurveSet {
    /// Samples all curves into discrete contour segments.
    ///
    /// # Arguments
    /// * `sample_distance` - The maximum distance between consecutive sample points
    ///
    /// # Returns
    /// A `ContourSet` containing one `Contour` per `Curve` in this set.
    fn sample_contour(&self, sample_distance: f64) -> Result<ContourSet, GeometryError> {
        if sample_distance <= 0.0 {
            return Err(GeometryError::InvalidParameter {
                parameter: "sample_distance".to_string(),
                message: "Sample distance must be positive".to_string(),
            });
        }

        let mut contours = Vec::with_capacity(self.curves.len());

        for curve in &self.curves {
            let contour = sample_curve_to_contour(curve, sample_distance)?;
            contours.push(contour);
        }

        Ok(ContourSet::with_contours(contours))
    }
}

// ============================================================================
// Factory functions for creating NURBS curves representing conic sections
// ============================================================================

impl Nurbs {
    /// Creates a rational NURBS curve representing a 90-degree circular arc.
    ///
    /// The arc is centered at the origin and spans from `start_angle` for 90 degrees.
    ///
    /// # Arguments
    /// * `radius` - The radius of the arc
    /// * `start_angle` - The starting angle in radians
    ///
    /// # Returns
    /// A quadratic rational NURBS curve representing the arc
    pub fn arc_90(radius: f64, start_angle: f64) -> Self {
        let cos_start = start_angle.cos();
        let sin_start = start_angle.sin();
        let cos_end = (start_angle + std::f64::consts::FRAC_PI_2).cos();
        let sin_end = (start_angle + std::f64::consts::FRAC_PI_2).sin();

        // Start point on the circle
        let p0 = DVec2::new(radius * cos_start, radius * sin_start);
        // End point on the circle
        let p2 = DVec2::new(radius * cos_end, radius * sin_end);

        // Control point at the intersection of tangent lines
        // For a 90-degree arc, the control point is at distance radius/cos(45°) = radius * sqrt(2)
        let mid_angle = start_angle + std::f64::consts::FRAC_PI_4;
        let control_distance = radius / (std::f64::consts::FRAC_PI_4).cos();
        let p1 = DVec2::new(
            control_distance * mid_angle.cos(),
            control_distance * mid_angle.sin(),
        );

        // Weight for middle control point is cos(45°) = sqrt(2)/2 ≈ 0.7071
        let w = std::f64::consts::FRAC_1_SQRT_2;

        Self::with_weights(vec![p0, p1, p2], vec![1.0, w, 1.0], 2)
    }

    /// Creates a rational NURBS curve representing a circular arc with arbitrary angle.
    ///
    /// # Arguments
    /// * `center` - The center of the arc
    /// * `radius` - The radius of the arc
    /// * `start_angle` - The starting angle in radians
    /// * `sweep_angle` - The sweep angle in radians (positive = counterclockwise)
    ///
    /// # Returns
    /// A quadratic rational NURBS curve representing the arc.
    /// For arcs > 90°, multiple segments should be used (see `arc_segments`).
    pub fn arc(center: DVec2, radius: f64, start_angle: f64, sweep_angle: f64) -> Self {
        let half_angle = sweep_angle / 2.0;
        let cos_half = half_angle.cos();

        let cos_start = start_angle.cos();
        let sin_start = start_angle.sin();
        let cos_end = (start_angle + sweep_angle).cos();
        let sin_end = (start_angle + sweep_angle).sin();

        // Start and end points
        let p0 = center + DVec2::new(radius * cos_start, radius * sin_start);
        let p2 = center + DVec2::new(radius * cos_end, radius * sin_end);

        // Control point - intersection of tangent lines
        let mid_angle = start_angle + half_angle;
        let control_radius = radius / cos_half;
        let p1 = center
            + DVec2::new(
                control_radius * mid_angle.cos(),
                control_radius * mid_angle.sin(),
            );

        // Weight for the middle control point
        let w = cos_half;

        Self::with_weights(vec![p0, p1, p2], vec![1.0, w, 1.0], 2)
    }

    /// Creates multiple NURBS segments representing a circular arc.
    ///
    /// For arcs larger than 90°, this creates multiple quadratic segments
    /// that together form the arc accurately.
    ///
    /// # Arguments
    /// * `center` - The center of the arc
    /// * `radius` - The radius of the arc
    /// * `start_angle` - The starting angle in radians
    /// * `sweep_angle` - The total sweep angle in radians
    ///
    /// # Returns
    /// A vector of NURBS curves, each representing a portion of the arc
    pub fn arc_segments(
        center: DVec2,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    ) -> Vec<Self> {
        let abs_sweep = sweep_angle.abs();

        // Determine number of segments (max 90° per segment for accuracy)
        let num_segments = ((abs_sweep / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
        let segment_sweep = sweep_angle / num_segments as f64;

        let mut segments = Vec::with_capacity(num_segments);
        let mut current_angle = start_angle;

        for _ in 0..num_segments {
            segments.push(Self::arc(center, radius, current_angle, segment_sweep));
            current_angle += segment_sweep;
        }

        segments
    }

    /// Creates NURBS curves representing a full circle.
    ///
    /// A circle requires multiple segments (typically 4 quadrants) because
    /// a single NURBS curve cannot represent a full 360° arc.
    ///
    /// # Arguments
    /// * `center` - The center of the circle
    /// * `radius` - The radius of the circle
    ///
    /// # Returns
    /// A vector of 4 NURBS curves, each representing a 90° arc
    pub fn circle(center: DVec2, radius: f64) -> Vec<Self> {
        Self::arc_segments(center, radius, 0.0, 2.0 * std::f64::consts::PI)
    }

    /// Creates a rational NURBS curve representing an elliptical arc.
    ///
    /// # Arguments
    /// * `center` - The center of the ellipse
    /// * `radius_x` - The radius along the X axis
    /// * `radius_y` - The radius along the Y axis
    /// * `start_angle` - The starting angle in radians (parametric angle)
    /// * `sweep_angle` - The sweep angle in radians
    ///
    /// # Returns
    /// A quadratic rational NURBS curve representing the elliptical arc
    pub fn ellipse_arc(
        center: DVec2,
        radius_x: f64,
        radius_y: f64,
        start_angle: f64,
        sweep_angle: f64,
    ) -> Self {
        let half_angle = sweep_angle / 2.0;
        let cos_half = half_angle.cos();

        let cos_start = start_angle.cos();
        let sin_start = start_angle.sin();
        let cos_end = (start_angle + sweep_angle).cos();
        let sin_end = (start_angle + sweep_angle).sin();

        // Start and end points on ellipse
        let p0 = center + DVec2::new(radius_x * cos_start, radius_y * sin_start);
        let p2 = center + DVec2::new(radius_x * cos_end, radius_y * sin_end);

        // Control point - this is more complex for ellipses
        // We use the tangent line intersection approach
        let mid_angle = start_angle + half_angle;
        let cos_mid = mid_angle.cos();
        let sin_mid = mid_angle.sin();

        // The control point lies along the direction of the mid-angle
        // but scaled to make the curve pass through the correct mid-point
        let p1 = center + DVec2::new(radius_x * cos_mid / cos_half, radius_y * sin_mid / cos_half);

        let w = cos_half;

        Self::with_weights(vec![p0, p1, p2], vec![1.0, w, 1.0], 2)
    }

    /// Creates NURBS curves representing a full ellipse.
    ///
    /// # Arguments
    /// * `center` - The center of the ellipse
    /// * `radius_x` - The radius along the X axis
    /// * `radius_y` - The radius along the Y axis
    ///
    /// # Returns
    /// A vector of 4 NURBS curves, each representing a 90° arc of the ellipse
    pub fn ellipse(center: DVec2, radius_x: f64, radius_y: f64) -> Vec<Self> {
        let mut segments = Vec::with_capacity(4);
        let quarter = std::f64::consts::FRAC_PI_2;

        for i in 0..4 {
            let start_angle = i as f64 * quarter;
            segments.push(Self::ellipse_arc(
                center,
                radius_x,
                radius_y,
                start_angle,
                quarter,
            ));
        }

        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // Nurbs tests

    #[test]
    fn test_nurbs_line_creation() {
        let start = DVec2::new(0.0, 0.0);
        let end = DVec2::new(1.0, 0.0);
        let nurbs = Nurbs::line(start, end);

        assert_eq!(nurbs.control_points.len(), 2);
        assert_eq!(nurbs.degree, 1);
        assert_eq!(nurbs.start_point(), start);
        assert_eq!(nurbs.end_point(), end);
    }

    #[test]
    fn test_nurbs_quadratic_creation() {
        let start = DVec2::new(0.0, 0.0);
        let control = DVec2::new(0.5, 1.0);
        let end = DVec2::new(1.0, 0.0);
        let nurbs = Nurbs::quadratic(start, control, end);

        assert_eq!(nurbs.control_points.len(), 3);
        assert_eq!(nurbs.degree, 2);
    }

    #[test]
    fn test_nurbs_cubic_creation() {
        let start = DVec2::new(0.0, 0.0);
        let ctrl1 = DVec2::new(0.25, 1.0);
        let ctrl2 = DVec2::new(0.75, 1.0);
        let end = DVec2::new(1.0, 0.0);
        let nurbs = Nurbs::cubic(start, ctrl1, ctrl2, end);

        assert_eq!(nurbs.control_points.len(), 4);
        assert_eq!(nurbs.degree, 3);
    }

    #[test]
    fn test_nurbs_line_length() {
        let start = DVec2::new(0.0, 0.0);
        let end = DVec2::new(3.0, 4.0);
        let nurbs = Nurbs::line(start, end);

        assert!((nurbs.length() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_nurbs_line_evaluate() {
        let start = DVec2::new(0.0, 0.0);
        let end = DVec2::new(10.0, 0.0);
        let nurbs = Nurbs::line(start, end);

        let mid = nurbs.evaluate(0.5);
        assert!((mid.x - 5.0).abs() < 1e-10);
        assert!((mid.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_nurbs_quadratic_evaluate() {
        let start = DVec2::new(0.0, 0.0);
        let control = DVec2::new(0.5, 1.0);
        let end = DVec2::new(1.0, 0.0);
        let nurbs = Nurbs::quadratic(start, control, end);

        // At t=0, should be at start
        let p0 = nurbs.evaluate(0.0);
        assert!((p0 - start).length() < 1e-10);

        // At t=1, should be at end
        let p1 = nurbs.evaluate(1.0);
        assert!((p1 - end).length() < 1e-10);

        // At t=0.5, should be at the peak
        let pmid = nurbs.evaluate(0.5);
        assert!((pmid.x - 0.5).abs() < 1e-10);
        assert!((pmid.y - 0.5).abs() < 1e-10); // Quadratic Bézier at t=0.5
    }

    #[test]
    fn test_nurbs_length_sampled() {
        let start = DVec2::new(0.0, 0.0);
        let end = DVec2::new(3.0, 4.0);
        let nurbs = Nurbs::line(start, end);

        let sampled_length = nurbs.length_sampled(100);
        assert!((sampled_length - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_nurbs_derivative() {
        let start = DVec2::new(0.0, 0.0);
        let end = DVec2::new(10.0, 0.0);
        let nurbs = Nurbs::line(start, end);

        let derivative = nurbs.derivative(0.5);
        assert!((derivative.x - 10.0).abs() < 1e-5);
        assert!((derivative.y - 0.0).abs() < 1e-5);
    }

    // NurbsSegment tests

    #[test]
    fn test_nurbs_segment_creation() {
        let nurbs = Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0));
        let segment = NurbsSegment::new(nurbs, 5.0, true, 0);

        assert_eq!(segment.start_distance, 5.0);
        assert_eq!(segment.end_distance, 15.0);
        assert_eq!(segment.length(), 10.0);
        assert!(segment.smooth_end);
        assert_eq!(segment.segment_id, 0);
    }

    #[test]
    fn test_nurbs_segment_evaluate() {
        let nurbs = Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0));
        let segment = NurbsSegment::new(nurbs, 0.0, false, 0);

        let point = segment.evaluate_at_distance(5.0);
        assert!((point.x - 5.0).abs() < 1e-10);
        assert!((point.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_nurbs_segment_tangent() {
        let nurbs = Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0));
        let segment = NurbsSegment::new(nurbs, 0.0, false, 0);

        let tangent = segment.tangent_at_distance(5.0);
        assert!((tangent.x - 1.0).abs() < 1e-10);
        assert!((tangent.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_nurbs_segment_with_segment_id() {
        let nurbs = Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0));
        let segment = NurbsSegment::new(nurbs, 0.0, false, 42);

        assert_eq!(segment.segment_id, 42);
    }

    #[test]
    fn test_nurbs_segment_with_distances() {
        let nurbs = Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0));
        let segment = NurbsSegment::with_distances(nurbs, 5.0, 15.0, true, 7);

        assert_eq!(segment.start_distance, 5.0);
        assert_eq!(segment.end_distance, 15.0);
        assert_eq!(segment.length(), 10.0);
        assert!(segment.smooth_end);
        assert_eq!(segment.segment_id, 7);
    }

    // Curve tests

    #[test]
    fn test_curve_creation() {
        let curve = Curve::new();
        assert!(curve.is_empty());
        assert_eq!(curve.segment_count(), 0);
    }

    #[test]
    fn test_curve_add_nurbs() {
        let mut curve = Curve::new();

        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);
        curve.add_nurbs(
            Nurbs::line(DVec2::new(10.0, 0.0), DVec2::new(20.0, 0.0)),
            true,
        );

        assert_eq!(curve.segment_count(), 2);
        assert_eq!(curve.total_length(), 20.0);

        assert_eq!(curve.segments[0].start_distance, 0.0);
        assert_eq!(curve.segments[0].end_distance, 10.0);
        assert_eq!(curve.segments[1].start_distance, 10.0);
        assert_eq!(curve.segments[1].end_distance, 20.0);
    }

    #[test]
    fn test_curve_evaluate() {
        let mut curve = Curve::new();

        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);
        curve.add_nurbs(
            Nurbs::line(DVec2::new(10.0, 0.0), DVec2::new(10.0, 10.0)),
            false,
        );

        let p1 = curve.evaluate_at_distance(5.0).unwrap();
        assert!((p1.x - 5.0).abs() < 1e-10);
        assert!((p1.y - 0.0).abs() < 1e-10);

        let p2 = curve.evaluate_at_distance(15.0).unwrap();
        assert!((p2.x - 10.0).abs() < 1e-10);
        assert!((p2.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_curve_sample() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);

        let samples = curve.sample(11);
        assert_eq!(samples.len(), 11);

        for (i, point) in samples.iter().enumerate() {
            assert!((point.x - i as f64).abs() < 1e-10);
        }
    }

    #[test]
    fn test_curve_find_segment() {
        let mut curve = Curve::new();

        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);
        curve.add_nurbs(
            Nurbs::line(DVec2::new(10.0, 0.0), DVec2::new(20.0, 0.0)),
            false,
        );

        let (idx, _) = curve.find_segment_at_distance(5.0).unwrap();
        assert_eq!(idx, 0);

        let (idx, _) = curve.find_segment_at_distance(15.0).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_curve_start_end_points() {
        let mut curve = Curve::new();

        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);
        curve.add_nurbs(
            Nurbs::line(DVec2::new(10.0, 0.0), DVec2::new(20.0, 5.0)),
            false,
        );

        assert_eq!(curve.start_point(), Some(DVec2::ZERO));
        assert_eq!(curve.end_point(), Some(DVec2::new(20.0, 5.0)));
    }

    // ========================================================================
    // SampleContour tests for Curve
    // ========================================================================

    #[test]
    fn test_sample_contour_linear_nurbs_single_segment() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);

        let result = curve.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);

        let contour = &contour_set.contours[0];
        // Linear NURBS should always produce exactly 1 segment regardless of sample_distance
        assert_eq!(contour.len(), 1);
    }

    #[test]
    fn test_sample_contour_linear_nurbs_long_line() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(100.0, 0.0)), false);

        let result = curve.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Linear NURBS should still produce exactly 1 segment
        assert_eq!(contour.len(), 1);
    }

    #[test]
    fn test_sample_contour_quadratic_nurbs_minimum_segments() {
        let mut curve = Curve::new();
        // Quadratic curve (degree 2) - should have at least 2 segments
        curve.add_nurbs(
            Nurbs::quadratic(DVec2::ZERO, DVec2::new(0.5, 1.0), DVec2::new(1.0, 0.0)),
            false,
        );

        // Use a large sample distance that would suggest fewer segments
        let result = curve.sample_contour(100.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Quadratic (degree 2) should have at least 2 segments
        assert!(contour.len() >= 2);
    }

    #[test]
    fn test_sample_contour_cubic_nurbs_minimum_segments() {
        let mut curve = Curve::new();
        // Cubic curve (degree 3) - should have at least 3 segments
        curve.add_nurbs(
            Nurbs::cubic(
                DVec2::ZERO,
                DVec2::new(0.25, 1.0),
                DVec2::new(0.75, 1.0),
                DVec2::new(1.0, 0.0),
            ),
            false,
        );

        // Use a large sample distance
        let result = curve.sample_contour(100.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];
        // Cubic (degree 3) should have at least 3 segments
        assert!(contour.len() >= 3);
    }

    #[test]
    fn test_sample_contour_respects_sample_distance() {
        let mut curve = Curve::new();
        // Quadratic curve with length approximately 1.5 (rough estimate)
        curve.add_nurbs(
            Nurbs::quadratic(DVec2::ZERO, DVec2::new(0.5, 1.0), DVec2::new(1.0, 0.0)),
            false,
        );

        // Small sample distance should produce more segments
        let result_fine = curve.sample_contour(0.1);
        let result_coarse = curve.sample_contour(1.0);

        assert!(result_fine.is_ok());
        assert!(result_coarse.is_ok());

        let fine_segments = result_fine.unwrap().contours[0].len();
        let coarse_segments = result_coarse.unwrap().contours[0].len();

        // Fine sampling should produce more or equal segments
        assert!(fine_segments >= coarse_segments);
    }

    #[test]
    fn test_sample_contour_invalid_sample_distance() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);

        let result_zero = curve.sample_contour(0.0);
        assert!(result_zero.is_err());

        let result_negative = curve.sample_contour(-1.0);
        assert!(result_negative.is_err());
    }

    #[test]
    fn test_sample_contour_empty_curve() {
        let curve = Curve::new();
        let result = curve.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 1);
        assert!(contour_set.contours[0].is_empty());
    }

    #[test]
    fn test_sample_contour_multiple_segments() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), true);
        curve.add_nurbs(
            Nurbs::quadratic(
                DVec2::new(10.0, 0.0),
                DVec2::new(15.0, 5.0),
                DVec2::new(20.0, 0.0),
            ),
            false,
        );

        let result = curve.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Should have 1 segment from line + at least 2 from quadratic
        assert!(contour.len() >= 3);

        // Check segment IDs
        assert_eq!(contour.segments[0].segment, 0); // First NURBS segment
        // Quadratic segments should have segment ID 1
        assert!(contour.segments.iter().any(|s| s.segment == 1));
    }

    #[test]
    fn test_sample_contour_smooth_end_propagation() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), true); // smooth_end = true
        curve.add_nurbs(
            Nurbs::line(DVec2::new(10.0, 0.0), DVec2::new(20.0, 0.0)),
            false, // smooth_end = false
        );

        let result = curve.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // First segment should have smooth_end = true (from first NURBS)
        assert!(contour.segments[0].smooth_end);
        // Last segment should have smooth_end = false (from second NURBS)
        assert!(!contour.segments.last().unwrap().smooth_end);
    }

    #[test]
    fn test_sample_contour_distance_continuity() {
        let mut curve = Curve::new();
        curve.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);
        curve.add_nurbs(
            Nurbs::line(DVec2::new(10.0, 0.0), DVec2::new(20.0, 0.0)),
            false,
        );

        let result = curve.sample_contour(1.0);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Check that distances are continuous
        let mut prev_end_distance = 0.0;
        for segment in &contour.segments {
            assert!(
                (segment.start.distance - prev_end_distance).abs() < 1e-10,
                "Distance discontinuity: {} vs {}",
                segment.start.distance,
                prev_end_distance
            );
            prev_end_distance = segment.end.distance;
        }

        // Total distance should equal total curve length
        assert!((prev_end_distance - 20.0).abs() < 1e-10);
    }

    // ========================================================================
    // CurveSet tests
    // ========================================================================

    #[test]
    fn test_curve_set_creation() {
        let curve_set = CurveSet::new();
        assert!(curve_set.is_empty());
        assert_eq!(curve_set.len(), 0);
    }

    #[test]
    fn test_curve_set_add_curve() {
        let mut curve_set = CurveSet::new();

        let mut curve1 = Curve::new();
        curve1.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);

        let mut curve2 = Curve::new();
        curve2.add_nurbs(
            Nurbs::line(DVec2::new(0.0, 5.0), DVec2::new(10.0, 5.0)),
            false,
        );

        curve_set.add_curve(curve1);
        curve_set.add_curve(curve2);

        assert_eq!(curve_set.len(), 2);
        assert!(!curve_set.is_empty());
        assert_eq!(curve_set.total_length(), 20.0);
    }

    #[test]
    fn test_curve_set_with_curves() {
        let mut curve1 = Curve::new();
        curve1.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);

        let mut curve2 = Curve::new();
        curve2.add_nurbs(
            Nurbs::line(DVec2::new(0.0, 5.0), DVec2::new(10.0, 5.0)),
            false,
        );

        let curve_set = CurveSet::with_curves(vec![curve1, curve2]);
        assert_eq!(curve_set.len(), 2);
    }

    #[test]
    fn test_curve_set_sample_contour() {
        let mut curve1 = Curve::new();
        curve1.add_nurbs(Nurbs::line(DVec2::ZERO, DVec2::new(10.0, 0.0)), false);

        let mut curve2 = Curve::new();
        curve2.add_nurbs(
            Nurbs::line(DVec2::new(0.0, 5.0), DVec2::new(10.0, 5.0)),
            false,
        );

        let curve_set = CurveSet::with_curves(vec![curve1, curve2]);
        let result = curve_set.sample_contour(1.0);

        assert!(result.is_ok());
        let contour_set = result.unwrap();

        // Should have 2 contours (one per curve)
        assert_eq!(contour_set.len(), 2);
    }

    #[test]
    fn test_curve_set_sample_contour_invalid_distance() {
        let curve_set = CurveSet::new();

        let result = curve_set.sample_contour(0.0);
        assert!(result.is_err());

        let result = curve_set.sample_contour(-1.0);
        assert!(result.is_err());
    }

    // ========================================================================
    // Arc, Circle, and Ellipse NURBS tests
    // ========================================================================

    #[test]
    fn test_nurbs_arc_90_degree() {
        let radius = 1.0;
        let arc = Nurbs::arc_90(radius, 0.0);

        // Should be a quadratic rational curve
        assert_eq!(arc.degree, 2);
        assert_eq!(arc.control_points.len(), 3);
        assert_eq!(arc.weights.len(), 3);

        // Check start and end points
        let start = arc.evaluate(0.0);
        let end = arc.evaluate(1.0);

        assert!((start - DVec2::new(radius, 0.0)).length() < 1e-10);
        assert!((end - DVec2::new(0.0, radius)).length() < 1e-10);

        // Check that midpoint is on the circle
        let mid = arc.evaluate(0.5);
        let mid_radius = mid.length();
        assert!(
            (mid_radius - radius).abs() < 0.01,
            "Midpoint radius {} should be close to {}",
            mid_radius,
            radius
        );
    }

    #[test]
    fn test_nurbs_arc_arbitrary_angle() {
        let center = DVec2::new(5.0, 5.0);
        let radius = 2.0;
        let start_angle = PI / 4.0; // 45 degrees
        let sweep_angle = PI / 3.0; // 60 degrees

        let arc = Nurbs::arc(center, radius, start_angle, sweep_angle);

        // Check start point
        let start = arc.evaluate(0.0);
        let expected_start =
            center + DVec2::new(radius * start_angle.cos(), radius * start_angle.sin());
        assert!((start - expected_start).length() < 1e-10);

        // Check end point
        let end = arc.evaluate(1.0);
        let end_angle = start_angle + sweep_angle;
        let expected_end = center + DVec2::new(radius * end_angle.cos(), radius * end_angle.sin());
        assert!((end - expected_end).length() < 1e-10);

        // Check that points along the arc are at the correct radius
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let point = arc.evaluate(t);
            let dist_from_center = (point - center).length();
            assert!(
                (dist_from_center - radius).abs() < 0.01,
                "Point at t={} has distance {} from center, expected {}",
                t,
                dist_from_center,
                radius
            );
        }
    }

    #[test]
    fn test_nurbs_arc_length() {
        let radius = 1.0;
        let sweep_angle = PI / 2.0; // 90 degrees
        let arc = Nurbs::arc(DVec2::ZERO, radius, 0.0, sweep_angle);

        let expected_length = radius * sweep_angle; // Arc length = r * θ
        let actual_length = arc.length();

        assert!(
            (actual_length - expected_length).abs() < 0.01,
            "Arc length {} should be close to {}",
            actual_length,
            expected_length
        );
    }

    #[test]
    fn test_nurbs_arc_segments() {
        let center = DVec2::ZERO;
        let radius = 1.0;
        let sweep_angle = PI; // 180 degrees - should need 2 segments

        let segments = Nurbs::arc_segments(center, radius, 0.0, sweep_angle);

        // Should have 2 segments for 180 degrees
        assert_eq!(segments.len(), 2);

        // Check continuity between segments
        let end_of_first = segments[0].evaluate(1.0);
        let start_of_second = segments[1].evaluate(0.0);
        assert!(
            (end_of_first - start_of_second).length() < 1e-10,
            "Segments should be continuous"
        );

        // Check total arc is correct
        let start = segments[0].evaluate(0.0);
        let end = segments.last().unwrap().evaluate(1.0);
        assert!((start - DVec2::new(radius, 0.0)).length() < 1e-10);
        assert!((end - DVec2::new(-radius, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_nurbs_circle() {
        let center = DVec2::new(1.0, 1.0);
        let radius = 2.0;

        let circle_segments = Nurbs::circle(center, radius);

        // Should have 4 segments (quadrants)
        assert_eq!(circle_segments.len(), 4);

        // Check that all points are on the circle
        for segment in &circle_segments {
            for i in 0..=10 {
                let t = i as f64 / 10.0;
                let point = segment.evaluate(t);
                let dist_from_center = (point - center).length();
                assert!(
                    (dist_from_center - radius).abs() < 0.02,
                    "Circle point at distance {} from center, expected {}",
                    dist_from_center,
                    radius
                );
            }
        }

        // Check continuity between all segments
        for i in 0..circle_segments.len() {
            let next_i = (i + 1) % circle_segments.len();
            let end_current = circle_segments[i].evaluate(1.0);
            let start_next = circle_segments[next_i].evaluate(0.0);
            assert!(
                (end_current - start_next).length() < 0.02,
                "Circle segments {} and {} should be continuous",
                i,
                next_i
            );
        }

        // Check total circumference
        let mut total_length = 0.0;
        for segment in &circle_segments {
            total_length += segment.length();
        }
        let expected_circumference = 2.0 * PI * radius;
        assert!(
            (total_length - expected_circumference).abs() < 0.1,
            "Circle circumference {} should be close to {}",
            total_length,
            expected_circumference
        );
    }

    #[test]
    fn test_nurbs_ellipse_arc() {
        let center = DVec2::ZERO;
        let radius_x = 2.0;
        let radius_y = 1.0;
        let start_angle = 0.0;
        let sweep_angle = PI / 2.0;

        let arc = Nurbs::ellipse_arc(center, radius_x, radius_y, start_angle, sweep_angle);

        // Check start point (should be at (radius_x, 0))
        let start = arc.evaluate(0.0);
        assert!((start - DVec2::new(radius_x, 0.0)).length() < 1e-10);

        // Check end point (should be at (0, radius_y))
        let end = arc.evaluate(1.0);
        assert!((end - DVec2::new(0.0, radius_y)).length() < 1e-10);

        // Check that points are approximately on the ellipse
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let point = arc.evaluate(t);
            // Ellipse equation: (x/a)^2 + (y/b)^2 = 1
            let ellipse_value = (point.x / radius_x).powi(2) + (point.y / radius_y).powi(2);
            assert!(
                (ellipse_value - 1.0).abs() < 0.05,
                "Point at t={} has ellipse value {}, expected 1.0",
                t,
                ellipse_value
            );
        }
    }

    #[test]
    fn test_nurbs_ellipse() {
        let center = DVec2::new(1.0, 2.0);
        let radius_x = 3.0;
        let radius_y = 1.5;

        let ellipse_segments = Nurbs::ellipse(center, radius_x, radius_y);

        // Should have 4 segments
        assert_eq!(ellipse_segments.len(), 4);

        // Check that all points are approximately on the ellipse
        for segment in &ellipse_segments {
            for i in 0..=10 {
                let t = i as f64 / 10.0;
                let point = segment.evaluate(t);
                let local = point - center;
                let ellipse_value = (local.x / radius_x).powi(2) + (local.y / radius_y).powi(2);
                assert!(
                    (ellipse_value - 1.0).abs() < 0.05,
                    "Ellipse point has value {}, expected 1.0",
                    ellipse_value
                );
            }
        }

        // Check continuity
        for i in 0..ellipse_segments.len() {
            let next_i = (i + 1) % ellipse_segments.len();
            let end_current = ellipse_segments[i].evaluate(1.0);
            let start_next = ellipse_segments[next_i].evaluate(0.0);
            assert!(
                (end_current - start_next).length() < 0.02,
                "Ellipse segments {} and {} should be continuous",
                i,
                next_i
            );
        }
    }

    #[test]
    fn test_sample_contour_arc() {
        let mut curve = Curve::new();
        let arc = Nurbs::arc(DVec2::ZERO, 1.0, 0.0, PI / 2.0);
        curve.add_nurbs(arc, false);

        let result = curve.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Should have multiple segments
        assert!(contour.len() >= 2, "Arc should produce at least 2 segments");

        // Verify all sampled points are approximately on the arc
        for segment in &contour.segments {
            let start_dist = segment.start.position.length();
            let end_dist = segment.end.position.length();
            assert!(
                (start_dist - 1.0).abs() < 0.01,
                "Start point distance {} should be 1.0",
                start_dist
            );
            assert!(
                (end_dist - 1.0).abs() < 0.01,
                "End point distance {} should be 1.0",
                end_dist
            );
        }
    }

    #[test]
    fn test_sample_contour_circle() {
        let center = DVec2::ZERO;
        let radius = 1.0;

        let mut curve = Curve::new();
        for (i, arc) in Nurbs::circle(center, radius).into_iter().enumerate() {
            let is_last = i == 3;
            curve.add_nurbs(arc, !is_last); // All but last are smooth
        }

        let result = curve.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Should have segments from all 4 quadrants
        assert!(
            contour.len() >= 8,
            "Circle should produce at least 8 segments"
        );

        // Verify all points are on the circle
        for segment in &contour.segments {
            let start_dist = segment.start.position.length();
            let end_dist = segment.end.position.length();
            assert!(
                (start_dist - radius).abs() < 0.02,
                "Point should be on circle"
            );
            assert!(
                (end_dist - radius).abs() < 0.02,
                "Point should be on circle"
            );
        }
    }

    #[test]
    fn test_sample_contour_ellipse() {
        let center = DVec2::ZERO;
        let radius_x = 2.0;
        let radius_y = 1.0;

        let mut curve = Curve::new();
        for (i, arc) in Nurbs::ellipse(center, radius_x, radius_y)
            .into_iter()
            .enumerate()
        {
            let is_last = i == 3;
            curve.add_nurbs(arc, !is_last);
        }

        let result = curve.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        let contour = &contour_set.contours[0];

        // Verify all points are on the ellipse
        for segment in &contour.segments {
            let p = segment.start.position;
            let ellipse_value = (p.x / radius_x).powi(2) + (p.y / radius_y).powi(2);
            assert!(
                (ellipse_value - 1.0).abs() < 0.05,
                "Point should be on ellipse"
            );
        }
    }

    #[test]
    fn test_curve_set_with_circle_and_ellipse() {
        let mut curve_set = CurveSet::new();

        // Add a circle
        let mut circle_curve = Curve::new();
        for (i, arc) in Nurbs::circle(DVec2::ZERO, 1.0).into_iter().enumerate() {
            circle_curve.add_nurbs(arc, i != 3);
        }
        curve_set.add_curve(circle_curve);

        // Add an ellipse
        let mut ellipse_curve = Curve::new();
        for (i, arc) in Nurbs::ellipse(DVec2::new(5.0, 0.0), 2.0, 1.0)
            .into_iter()
            .enumerate()
        {
            ellipse_curve.add_nurbs(arc, i != 3);
        }
        curve_set.add_curve(ellipse_curve);

        let result = curve_set.sample_contour(0.1);
        assert!(result.is_ok());

        let contour_set = result.unwrap();
        assert_eq!(contour_set.len(), 2);

        // Both contours should have segments
        assert!(!contour_set.contours[0].is_empty());
        assert!(!contour_set.contours[1].is_empty());
    }

    #[test]
    fn test_calculate_nurbs_sample_count() {
        // Linear NURBS always returns 1
        let line = Nurbs::line(DVec2::ZERO, DVec2::new(100.0, 0.0));
        assert_eq!(calculate_nurbs_sample_count(&line, 1.0), 1);
        assert_eq!(calculate_nurbs_sample_count(&line, 0.1), 1);

        // Quadratic NURBS has minimum of 2 segments
        let quadratic = Nurbs::quadratic(DVec2::ZERO, DVec2::new(0.5, 1.0), DVec2::new(1.0, 0.0));
        assert!(calculate_nurbs_sample_count(&quadratic, 100.0) >= 2);

        // Cubic NURBS has minimum of 3 segments
        let cubic = Nurbs::cubic(
            DVec2::ZERO,
            DVec2::new(0.25, 1.0),
            DVec2::new(0.75, 1.0),
            DVec2::new(1.0, 0.0),
        );
        assert!(calculate_nurbs_sample_count(&cubic, 100.0) >= 3);
    }
}
