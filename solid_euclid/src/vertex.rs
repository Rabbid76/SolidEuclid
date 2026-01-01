use glam::{DVec2, DVec3};

/// Type alias for vertex indices.
pub type Index = i32;

/// Represents a vertex with position, normal, and UV coordinates.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
