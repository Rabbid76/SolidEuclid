# Math

## NURBS Curve Capabilities Explained

> Is a NURBS curve with order 0 (i.e. no control points) a line?
> Can a NURBS curve represent an arc?
> Can a NURBS curve represent an ellipse segment?
>Can a NURBS curve represent a circle?
> Can a NURBS curve represent an ellipse?
> Could you provide some examples and specify the required order of NURBS curves?

Here is an analysis of NURBS (Non-Uniform Rational B-Splines) capabilities regarding your specific geometric questions.

### 1. Is a NURBS curve with Order 0 a line?

**No, it is technically undefined.**

In NURBS terminology, there is a strict relationship between **Degree** () and **Order** ():

- **Order 1 (Degree 0):** This would represent a point (or a piecewise constant function), not a line.
- **Order 0:** This implies a degree of , which is mathematically undefined and impossible to construct. A curve requires at least one control point to exist as a point, and two control points to exist as a span.
- **A Line:** A straight line is the simplest valid span. It requires **Degree 1 (Order 2)** and at least two control points.

### 2. Can a NURBS curve represent an Arc, Circle, or Ellipse?

**Yes, to all three.**

This is the primary advantage of **NURBS** over standard B-Splines or Bezier curves.

- **Standard B-Splines (Non-Rational):** Can only *approximate* circles and ellipses. They are polynomials, and polynomials cannot exactly describe a perfect circle.
- **NURBS (Rational):** The "R" stands for **Rational**. By assigning specific **weights** to control points, NURBS can define the ratio of polynomials, which allows them to represent conic sections (circles, ellipses, hyperbolas, parabolas) **exactly**.

#### Capabilities Breakdown

- **Arc:** Yes. A NURBS curve can represent a circular arc exactly.
- **Ellipse Segment:** Yes.
- **Circle:** Yes. A full circle is typically represented by piecing together multiple arcs (often 3 or 4 quadrants/segments) into a single NURBS definition.
- **Ellipse:** Yes. Similar to the circle, it is a rational curve requiring specific weights.

---

### 3. Required Orders and Examples

Below is a breakdown of the specific geometric shapes and the minimum NURBS configuration required to create them.

| Shape | Min. Degree () | Min. Order () | Control Points Needed | Weights () |
| --- | --- | --- | --- | --- |
| **Point** | 0 | 1 | 1 | All 1 |
| **Line / Polyline** | 1 | 2 | 2 per segment | All 1 |
| **Parabola** | 2 | 3 | 3 | All 1 |
| **Circle / Arc** | 2 | 3 | 3 per 90° arc | Alternating (see below) |
| **Ellipse** | 2 | 3 | 3 per quadrant | Alternating |
| **Freeform Cubic** | 3 | 4 | 4 | Varied |

#### Detailed Examples

**1. The Straight Line**
To create a simple line segment:

- **Order:** 2 (Degree 1).
- **Control Points:** 2 (Start and End).
- **Knots:** No internal knots are needed.
- **Weights:** All weights are .

**2. The Circular Arc (90°)**
To create a perfect quarter-circle arc:

- **Order:** 3 (Degree 2).
- **Control Points:** 3 points are arranged in a specific triangle (Start, Corner, End).
- **Weights:**
  - (Start): w = 1
  - (Corner): sqir(2)/2 (approx 0.7071) *This weight "pulls" the curve closer to the center to form a circle rather than a parabola.*
   (End): w = 1

**3. The Full Circle**
A single quadratic Bezier/NURBS span cannot form a full circle (it can't wrap 360° without self-intersecting or losing the rational property). Therefore, a NURBS circle is usually constructed by joining **four 90° arcs**.

- **Order:** 3 (Degree 2).
- **Control Points:** 9 points (forming a square bounding box around the circle).
  - The "Corner" points will have weights .
  - The "Midpoint" points (where the circle touches the square) will have weights .
- **Knot Vector:** Requires specific repeating knots (multiplicity) to join the arcs smoothly ( continuity).

**4. The Ellipse**
This is mathematically identical to the circle but stretched.

- **Order:** 3 (Degree 2).
- **Geometry:** You arrange the control points in a rectangle rather than a square.
- **Weights:** You use the same weights as the circle logic. The geometric position of the control points handles the "squashing" or "stretching" into an ellipse.

### Summary

If you are trying to implement or define these shapes in software:

- **Use Order 2** for linear segments.
- **Use Order 3** (Rational/Weighted) for all arcs, circles, and ellipses.
