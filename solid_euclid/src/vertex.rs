use glam::{DVec2, DVec3, Vec2, Vec3};

/// Type alias for vertex indices.
pub type Index = i32;

/// Represents a vertex with double-precision position, normal, and UV coordinates.
///
/// This struct uses `f64` types (`DVec3`, `DVec2`) for high-precision geometric
/// calculations. It is primarily used in [`PolygonMesh`](crate::polygon_mesh::PolygonMesh)
/// for constructing and manipulating geometry with maximum precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// The 3D position of the vertex.
    pub position: DVec3,
    /// The normal vector at the vertex.
    pub normal: DVec3,
    /// The UV texture coordinates.
    pub uv: DVec2,
}

impl Vertex {
    /// Creates a new `Vertex` with the given position, normal, and UV coordinates.
    pub fn new(position: DVec3, normal: DVec3, uv: DVec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }

    /// Converts this `Vertex` to a `VertexF32` with single-precision floats.
    pub fn to_f32(&self) -> VertexF32 {
        VertexF32 {
            position: self.position.as_vec3(),
            normal: self.normal.as_vec3(),
            uv: self.uv.as_vec2(),
        }
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            normal: DVec3::Z,
            uv: DVec2::ZERO,
        }
    }
}

/// Represents a vertex with single-precision position, normal, and UV coordinates.
///
/// This struct uses `f32` types (`Vec3`, `Vec2`) which are suitable for GPU-based
/// rendering pipelines. It is primarily used in [`TriangularGeometry`](crate::triangular_mesh::TriangularGeometry)
/// for preparing geometry data for GPU renderers such as three.js, WebGL, or other
/// graphics APIs that typically operate with 32-bit floats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexF32 {
    /// The 3D position of the vertex.
    pub position: Vec3,
    /// The normal vector at the vertex.
    pub normal: Vec3,
    /// The UV texture coordinates.
    pub uv: Vec2,
}

impl VertexF32 {
    /// Creates a new `VertexF32` with the given position, normal, and UV coordinates.
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }

    /// Converts this `VertexF32` to a `Vertex` with double-precision floats.
    pub fn to_f64(&self) -> Vertex {
        Vertex {
            position: self.position.as_dvec3(),
            normal: self.normal.as_dvec3(),
            uv: self.uv.as_dvec2(),
        }
    }
}

impl Default for VertexF32 {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            normal: Vec3::Z,
            uv: Vec2::ZERO,
        }
    }
}

impl From<Vertex> for VertexF32 {
    fn from(vertex: Vertex) -> Self {
        vertex.to_f32()
    }
}

impl From<VertexF32> for Vertex {
    fn from(vertex: VertexF32) -> Self {
        vertex.to_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vertex tests

    #[test]
    fn test_vertex_new() {
        let position = DVec3::new(1.0, 2.0, 3.0);
        let normal = DVec3::new(0.0, 1.0, 0.0);
        let uv = DVec2::new(0.5, 0.5);

        let vertex = Vertex::new(position, normal, uv);

        assert_eq!(vertex.position, position);
        assert_eq!(vertex.normal, normal);
        assert_eq!(vertex.uv, uv);
    }

    #[test]
    fn test_vertex_default() {
        let vertex = Vertex::default();

        assert_eq!(vertex.position, DVec3::ZERO);
        assert_eq!(vertex.normal, DVec3::Z);
        assert_eq!(vertex.uv, DVec2::ZERO);
    }

    #[test]
    fn test_vertex_clone() {
        let vertex = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.25, 0.75),
        );

        let cloned = vertex.clone();

        assert_eq!(vertex, cloned);
    }

    #[test]
    fn test_vertex_copy() {
        let vertex = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.25, 0.75),
        );

        let copied = vertex;

        assert_eq!(vertex.position, copied.position);
        assert_eq!(vertex.normal, copied.normal);
        assert_eq!(vertex.uv, copied.uv);
    }

    #[test]
    fn test_vertex_equality() {
        let vertex1 = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.5, 0.5),
        );

        let vertex2 = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.5, 0.5),
        );

        let vertex3 = Vertex::new(
            DVec3::new(4.0, 5.0, 6.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec2::new(0.0, 1.0),
        );

        assert_eq!(vertex1, vertex2);
        assert_ne!(vertex1, vertex3);
    }

    #[test]
    fn test_vertex_debug() {
        let vertex = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.5, 0.5),
        );

        let debug_str = format!("{:?}", vertex);

        assert!(debug_str.contains("Vertex"));
        assert!(debug_str.contains("position"));
        assert!(debug_str.contains("normal"));
        assert!(debug_str.contains("uv"));
    }

    #[test]
    fn test_vertex_to_f32() {
        let vertex = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.5, 0.5),
        );

        let vertex_f32 = vertex.to_f32();

        assert_eq!(vertex_f32.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(vertex_f32.normal, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(vertex_f32.uv, Vec2::new(0.5, 0.5));
    }

    // VertexF32 tests

    #[test]
    fn test_vertex_f32_new() {
        let position = Vec3::new(1.0, 2.0, 3.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let uv = Vec2::new(0.5, 0.5);

        let vertex = VertexF32::new(position, normal, uv);

        assert_eq!(vertex.position, position);
        assert_eq!(vertex.normal, normal);
        assert_eq!(vertex.uv, uv);
    }

    #[test]
    fn test_vertex_f32_default() {
        let vertex = VertexF32::default();

        assert_eq!(vertex.position, Vec3::ZERO);
        assert_eq!(vertex.normal, Vec3::Z);
        assert_eq!(vertex.uv, Vec2::ZERO);
    }

    #[test]
    fn test_vertex_f32_clone() {
        let vertex = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.25, 0.75),
        );

        let cloned = vertex.clone();

        assert_eq!(vertex, cloned);
    }

    #[test]
    fn test_vertex_f32_copy() {
        let vertex = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.25, 0.75),
        );

        let copied = vertex;

        assert_eq!(vertex.position, copied.position);
        assert_eq!(vertex.normal, copied.normal);
        assert_eq!(vertex.uv, copied.uv);
    }

    #[test]
    fn test_vertex_f32_equality() {
        let vertex1 = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.5, 0.5),
        );

        let vertex2 = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.5, 0.5),
        );

        let vertex3 = VertexF32::new(
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec2::new(0.0, 1.0),
        );

        assert_eq!(vertex1, vertex2);
        assert_ne!(vertex1, vertex3);
    }

    #[test]
    fn test_vertex_f32_debug() {
        let vertex = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.5, 0.5),
        );

        let debug_str = format!("{:?}", vertex);

        assert!(debug_str.contains("VertexF32"));
        assert!(debug_str.contains("position"));
        assert!(debug_str.contains("normal"));
        assert!(debug_str.contains("uv"));
    }

    #[test]
    fn test_vertex_f32_to_f64() {
        let vertex = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.5, 0.5),
        );

        let vertex_f64 = vertex.to_f64();

        assert_eq!(vertex_f64.position, DVec3::new(1.0, 2.0, 3.0));
        assert_eq!(vertex_f64.normal, DVec3::new(0.0, 1.0, 0.0));
        assert_eq!(vertex_f64.uv, DVec2::new(0.5, 0.5));
    }

    // Conversion tests

    #[test]
    fn test_vertex_from_vertex_f32() {
        let vertex_f32 = VertexF32::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec2::new(0.5, 0.5),
        );

        let vertex: Vertex = vertex_f32.into();

        assert_eq!(vertex.position, DVec3::new(1.0, 2.0, 3.0));
        assert_eq!(vertex.normal, DVec3::new(0.0, 1.0, 0.0));
        assert_eq!(vertex.uv, DVec2::new(0.5, 0.5));
    }

    #[test]
    fn test_vertex_f32_from_vertex() {
        let vertex = Vertex::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.5, 0.5),
        );

        let vertex_f32: VertexF32 = vertex.into();

        assert_eq!(vertex_f32.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(vertex_f32.normal, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(vertex_f32.uv, Vec2::new(0.5, 0.5));
    }

    #[test]
    fn test_vertex_roundtrip_conversion() {
        let original = Vertex::new(
            DVec3::new(1.5, 2.5, 3.5),
            DVec3::new(0.0, 1.0, 0.0),
            DVec2::new(0.25, 0.75),
        );

        let vertex_f32: VertexF32 = original.into();
        let back: Vertex = vertex_f32.into();

        // Note: Due to f64 -> f32 -> f64 conversion, there may be small precision loss
        assert!((original.position.x - back.position.x).abs() < 1e-6);
        assert!((original.position.y - back.position.y).abs() < 1e-6);
        assert!((original.position.z - back.position.z).abs() < 1e-6);
    }
}
