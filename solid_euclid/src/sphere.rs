use std::f64::consts::PI;

use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::material::Material;
use crate::polygon_mesh::{Polygon, PolygonMesh};
use crate::vertex::Vertex;

/// Represents different sphere generation strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sphere {
    /// UV sphere generated with horizontal slices and vertical segments.
    /// - `slices`: Number of horizontal divisions (from bottom to top)
    /// - `segments`: Number of vertical divisions (around the sphere)
    Slices { slices: u32, segments: u32 },
    /// Cube-mapped sphere where a cube is projected onto a sphere.
    /// - `segments`: Number of divisions per face edge
    Cube { segments: u32 },
}

/// A model combining a sphere geometry with material assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub geometry: Sphere,
    pub material: Material,
}

impl Model {
    /// Creates a new model with the specified geometry and material.
    pub fn new(geometry: Sphere, material: Material) -> Self {
        Self { geometry, material }
    }

    /// Creates a new model with default material.
    pub fn with_default_material(geometry: Sphere) -> Self {
        Self {
            geometry,
            material: Material::Default,
        }
    }

    /// Creates a new model with uniform material.
    pub fn with_uniform_material(geometry: Sphere, material: String) -> Self {
        Self {
            geometry,
            material: Material::Uniform(material),
        }
    }

    /// Sets up materials in the polygon mesh based on the material assignment.
    fn setup_materials(&self, mesh: &mut PolygonMesh, total_faces: i32) {
        for face_index in 0..total_faces {
            let material_name = self.material.get_material_for_face(face_index, total_faces);
            mesh.set_material(face_index, material_name);
        }
    }
}

impl GeometryModel for Model {
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        match self.geometry {
            Sphere::Slices { slices, segments } => self.create_slices_sphere(slices, segments),
            Sphere::Cube { segments } => self.create_cube_sphere(segments),
        }
    }
}

impl Model {
    /// Creates a UV sphere polygon mesh from slices and segments.
    ///
    /// The sphere is generated from bottom (v=0) to top (v=1).
    /// Each slice has `segments` tiles around the sphere.
    /// The v coordinate is distributed by the half circle angle of the slice.
    /// The u coordinate is distributed in the range 0 to 1 around the sphere.
    fn create_slices_sphere(
        &self,
        slices: u32,
        segments: u32,
    ) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Slices sphere has only 1 face (the entire sphere surface)
        let geometry_id: i32 = 0;

        // Generate vertices and faces for each slice
        for slice in 0..slices {
            let theta0 = PI * (slice as f64) / (slices as f64);
            let theta1 = PI * ((slice + 1) as f64) / (slices as f64);

            let y0 = -theta0.cos();
            let y1 = -theta1.cos();
            let r0 = theta0.sin();
            let r1 = theta1.sin();

            // v coordinates based on the angle (0 at bottom, 1 at top)
            let v0 = (slice as f64) / (slices as f64);
            let v1 = ((slice + 1) as f64) / (slices as f64);

            for segment in 0..segments {
                let phi0 = 2.0 * PI * (segment as f64) / (segments as f64);
                let phi1 = 2.0 * PI * ((segment + 1) as f64) / (segments as f64);

                // u coordinates
                let u0 = (segment as f64) / (segments as f64);
                let u1 = ((segment + 1) as f64) / (segments as f64);

                // Calculate positions
                let p00 = DVec3::new(r0 * phi0.cos(), y0, r0 * phi0.sin());
                let p01 = DVec3::new(r0 * phi1.cos(), y0, r0 * phi1.sin());
                let p10 = DVec3::new(r1 * phi0.cos(), y1, r1 * phi0.sin());
                let p11 = DVec3::new(r1 * phi1.cos(), y1, r1 * phi1.sin());

                // For a sphere centered at origin with radius 1, the normal is the position
                let n00 = p00.normalize();
                let n01 = p01.normalize();
                let n10 = p10.normalize();
                let n11 = p11.normalize();

                // UV coordinates
                let uv00 = DVec2::new(u0, v0);
                let uv01 = DVec2::new(u1, v0);
                let uv10 = DVec2::new(u0, v1);
                let uv11 = DVec2::new(u1, v1);

                // Handle degenerate triangles at the poles
                // Winding order: counter-clockwise when viewed from outside
                if slice == 0 {
                    // Bottom cap - triangle
                    // At the bottom pole, p00 and p01 are the same point (the pole)
                    let v0_idx =
                        mesh.add_vertex(Vertex::new(p00, n00, DVec2::new((u0 + u1) / 2.0, v0)));
                    let v1_idx = mesh.add_vertex(Vertex::new(p10, n10, uv10));
                    let v2_idx = mesh.add_vertex(Vertex::new(p11, n11, uv11));

                    let centroid = (p00 + p10 + p11).normalize();
                    mesh.add_polygon(Polygon::new(
                        vec![v0_idx, v1_idx, v2_idx],
                        centroid,
                        geometry_id,
                    ));
                } else if slice == slices - 1 {
                    // Top cap - triangle
                    // At the top pole, p10 and p11 are the same point (the pole)
                    let v0_idx = mesh.add_vertex(Vertex::new(p00, n00, uv00));
                    let v1_idx =
                        mesh.add_vertex(Vertex::new(p10, n10, DVec2::new((u0 + u1) / 2.0, v1)));
                    let v2_idx = mesh.add_vertex(Vertex::new(p01, n01, uv01));

                    let centroid = (p00 + p10 + p01).normalize();
                    mesh.add_polygon(Polygon::new(
                        vec![v0_idx, v1_idx, v2_idx],
                        centroid,
                        geometry_id,
                    ));
                } else {
                    // Regular quad - counter-clockwise when viewed from outside
                    // Order: bottom-left, top-left, top-right, bottom-right
                    let v0_idx = mesh.add_vertex(Vertex::new(p00, n00, uv00));
                    let v1_idx = mesh.add_vertex(Vertex::new(p10, n10, uv10));
                    let v2_idx = mesh.add_vertex(Vertex::new(p11, n11, uv11));
                    let v3_idx = mesh.add_vertex(Vertex::new(p01, n01, uv01));

                    let centroid = (p00 + p10 + p11 + p01).normalize();
                    mesh.add_polygon(Polygon::new(
                        vec![v0_idx, v1_idx, v2_idx, v3_idx],
                        centroid,
                        geometry_id,
                    ));
                }
            }
        }

        // Slices sphere has 1 face
        self.setup_materials(&mut mesh, 1);
        Ok(mesh)
    }

    /// Creates a cube-mapped sphere polygon mesh.
    ///
    /// Each face of a cube is divided into `segments` x `segments` tiles.
    /// Each point on the cube is normalized to project it onto the unit sphere.
    /// The normal vector at each point is the same as the position (since radius = 1).
    /// Each face has texture coordinates in the range 0 to 1.
    fn create_cube_sphere(&self, segments: u32) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Define the six faces of the cube
        // Each face is defined by: (origin, u_axis, v_axis)
        // For CCW winding when viewed from outside, u_axis cross v_axis should point outward
        // Using right-hand rule: if fingers curl from u_axis to v_axis, thumb points outward
        let faces = [
            // Front face (z = 1): u_axis=+X, v_axis=+Y, cross product = +Z (outward) ✓
            (DVec3::new(-1.0, -1.0, 1.0), DVec3::X, DVec3::Y),
            // Back face (z = -1): u_axis=+X, v_axis=-Y, cross product = -Z (outward) ✓
            (DVec3::new(-1.0, 1.0, -1.0), DVec3::X, DVec3::NEG_Y),
            // Right face (x = 1): u_axis=+Z, v_axis=+Y, cross product = -X (inward!)
            // Fix: u_axis=-Z, v_axis=+Y gives cross = +X (outward) ✓
            (DVec3::new(1.0, -1.0, 1.0), DVec3::NEG_Z, DVec3::Y),
            // Left face (x = -1): u_axis=+Z, v_axis=+Y, cross product = -X (outward from left side)
            // Fix: u_axis=+Z, v_axis=+Y gives cross = -X (outward) ✓
            (DVec3::new(-1.0, -1.0, -1.0), DVec3::Z, DVec3::Y),
            // Top face (y = 1): u_axis=+X, v_axis=-Z, cross product = +Y (outward) ✓
            (DVec3::new(-1.0, 1.0, 1.0), DVec3::X, DVec3::NEG_Z),
            // Bottom face (y = -1): u_axis=+X, v_axis=+Z, cross product = -Y (outward) ✓
            (DVec3::new(-1.0, -1.0, -1.0), DVec3::X, DVec3::Z),
        ];

        for (cube_face_index, (origin, u_axis, v_axis)) in faces.iter().enumerate() {
            let geometry_id = cube_face_index as i32;
            for row in 0..segments {
                for col in 0..segments {
                    // Calculate UV coordinates (0 to 1 per face)
                    let u0 = (col as f64) / (segments as f64);
                    let u1 = ((col + 1) as f64) / (segments as f64);
                    let v0 = (row as f64) / (segments as f64);
                    let v1 = ((row + 1) as f64) / (segments as f64);

                    // Calculate positions on the cube face (scaled by 2 since face goes from -1 to 1)
                    let p00_cube = *origin + *u_axis * (u0 * 2.0) + *v_axis * (v0 * 2.0);
                    let p01_cube = *origin + *u_axis * (u1 * 2.0) + *v_axis * (v0 * 2.0);
                    let p10_cube = *origin + *u_axis * (u0 * 2.0) + *v_axis * (v1 * 2.0);
                    let p11_cube = *origin + *u_axis * (u1 * 2.0) + *v_axis * (v1 * 2.0);

                    // Project to sphere by normalizing (this gives both position and normal)
                    let p00 = p00_cube.normalize();
                    let p01 = p01_cube.normalize();
                    let p10 = p10_cube.normalize();
                    let p11 = p11_cube.normalize();

                    // For a unit sphere, position = normal
                    let n00 = p00;
                    let n01 = p01;
                    let n10 = p10;
                    let n11 = p11;

                    // UV coordinates
                    let uv00 = DVec2::new(u0, v0);
                    let uv01 = DVec2::new(u1, v0);
                    let uv10 = DVec2::new(u0, v1);
                    let uv11 = DVec2::new(u1, v1);

                    // Create quad with counter-clockwise winding when viewed from outside
                    // Order: bottom-left (00), bottom-right (01), top-right (11), top-left (10)
                    let v0_idx = mesh.add_vertex(Vertex::new(p00, n00, uv00));
                    let v1_idx = mesh.add_vertex(Vertex::new(p01, n01, uv01));
                    let v2_idx = mesh.add_vertex(Vertex::new(p11, n11, uv11));
                    let v3_idx = mesh.add_vertex(Vertex::new(p10, n10, uv10));

                    let centroid = (p00 + p01 + p11 + p10).normalize();
                    mesh.add_polygon(Polygon::new(
                        vec![v0_idx, v1_idx, v2_idx, v3_idx],
                        centroid,
                        geometry_id,
                    ));
                }
            }
        }

        // Cube sphere has 6 faces (one per cube face)
        self.setup_materials(&mut mesh, 6);
        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Sphere enum tests

    #[test]
    fn test_sphere_slices_variant() {
        let sphere = Sphere::Slices {
            slices: 16,
            segments: 32,
        };
        if let Sphere::Slices { slices, segments } = sphere {
            assert_eq!(slices, 16);
            assert_eq!(segments, 32);
        } else {
            panic!("Expected Sphere::Slices");
        }
    }

    #[test]
    fn test_sphere_cube_variant() {
        let sphere = Sphere::Cube { segments: 8 };
        if let Sphere::Cube { segments } = sphere {
            assert_eq!(segments, 8);
        } else {
            panic!("Expected Sphere::Cube");
        }
    }

    #[test]
    fn test_sphere_clone_and_copy() {
        let original = Sphere::Slices {
            slices: 10,
            segments: 20,
        };
        let cloned = original.clone();
        let copied = original;

        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }

    #[test]
    fn test_sphere_debug() {
        let sphere = Sphere::Slices {
            slices: 8,
            segments: 16,
        };
        let debug_str = format!("{:?}", sphere);
        assert!(debug_str.contains("Slices"));
        assert!(debug_str.contains("8"));
        assert!(debug_str.contains("16"));
    }

    // Model tests

    #[test]
    fn test_model_new() {
        let model = Model::new(Sphere::Cube { segments: 4 }, Material::Default);
        assert_eq!(model.geometry, Sphere::Cube { segments: 4 });
        assert_eq!(model.material, Material::Default);
    }

    #[test]
    fn test_model_with_default_material() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        assert_eq!(
            model.geometry,
            Sphere::Slices {
                slices: 8,
                segments: 16
            }
        );
        assert_eq!(model.material, Material::Default);
    }

    #[test]
    fn test_model_with_uniform_material() {
        let model = Model::with_uniform_material(Sphere::Cube { segments: 6 }, "metal".to_string());
        assert_eq!(model.geometry, Sphere::Cube { segments: 6 });
        assert_eq!(model.material, Material::Uniform("metal".to_string()));
    }

    #[test]
    fn test_model_with_map_material() {
        let mut face_map = HashMap::new();
        face_map.insert(0, "special".to_string());

        let material = Material::Map {
            default: "base".to_string(),
            face_map,
        };

        let model = Model::new(Sphere::Cube { segments: 2 }, material);
        assert_eq!(model.geometry, Sphere::Cube { segments: 2 });

        if let Material::Map { default, face_map } = &model.material {
            assert_eq!(default, "base");
            assert_eq!(face_map.len(), 1);
        } else {
            panic!("Expected Material::Map");
        }
    }

    #[test]
    fn test_model_clone() {
        let model = Model::new(
            Sphere::Slices {
                slices: 4,
                segments: 8,
            },
            Material::Uniform("glass".to_string()),
        );
        let cloned = model.clone();
        assert_eq!(model, cloned);
    }

    #[test]
    fn test_model_debug() {
        let model = Model::with_default_material(Sphere::Cube { segments: 4 });
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("Cube"));
        assert!(debug_str.contains("Default"));
    }

    // GeometryModel trait implementation tests

    #[test]
    fn test_slices_sphere_to_polygon_mesh() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 4,
            segments: 8,
        });
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // With 4 slices and 8 segments:
        // - 1st slice (bottom cap): 8 triangles
        // - 2nd and 3rd slices: 8 quads each = 16 quads
        // - 4th slice (top cap): 8 triangles
        // Total: 8 + 8 + 8 + 8 = 32 polygons
        assert_eq!(mesh.polygons.len(), 32);
        // Slices sphere has only 1 face, so 1 material
        assert_eq!(mesh.material_map.len(), 1);
    }

    #[test]
    fn test_cube_sphere_to_polygon_mesh() {
        let model = Model::with_default_material(Sphere::Cube { segments: 4 });
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // 6 faces * 4*4 tiles = 96 quads
        assert_eq!(mesh.polygons.len(), 96);
        // Cube sphere has 6 faces, so 6 materials
        assert_eq!(mesh.material_map.len(), 6);

        // Each polygon should be a quad
        for polygon in &mesh.polygons {
            assert_eq!(polygon.indices.len(), 4);
        }
    }

    #[test]
    fn test_slices_sphere_vertices_on_unit_sphere() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let length = vertex.position.length();
            assert!(
                (length - 1.0).abs() < 1e-10,
                "Vertex should be on unit sphere, got length {}",
                length
            );
        }
    }

    #[test]
    fn test_cube_sphere_vertices_on_unit_sphere() {
        let model = Model::with_default_material(Sphere::Cube { segments: 4 });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let length = vertex.position.length();
            assert!(
                (length - 1.0).abs() < 1e-10,
                "Vertex should be on unit sphere, got length {}",
                length
            );
        }
    }

    #[test]
    fn test_slices_sphere_normals_are_normalized() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let normal_length = vertex.normal.length();
            assert!(
                (normal_length - 1.0).abs() < 1e-10,
                "Normal should be unit length, got {}",
                normal_length
            );
        }
    }

    #[test]
    fn test_cube_sphere_normals_are_normalized() {
        let model = Model::with_default_material(Sphere::Cube { segments: 4 });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let normal_length = vertex.normal.length();
            assert!(
                (normal_length - 1.0).abs() < 1e-10,
                "Normal should be unit length, got {}",
                normal_length
            );
        }
    }

    #[test]
    fn test_slices_sphere_normal_equals_position() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let diff = (vertex.position - vertex.normal).length();
            assert!(
                diff < 1e-10,
                "For unit sphere, normal should equal position"
            );
        }
    }

    #[test]
    fn test_cube_sphere_normal_equals_position() {
        let model = Model::with_default_material(Sphere::Cube { segments: 4 });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let diff = (vertex.position - vertex.normal).length();
            assert!(
                diff < 1e-10,
                "For unit sphere, normal should equal position"
            );
        }
    }

    #[test]
    fn test_slices_sphere_uv_coordinates_range() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            assert!(
                vertex.uv.x >= 0.0 && vertex.uv.x <= 1.0,
                "U coordinate should be in [0,1], got {}",
                vertex.uv.x
            );
            assert!(
                vertex.uv.y >= 0.0 && vertex.uv.y <= 1.0,
                "V coordinate should be in [0,1], got {}",
                vertex.uv.y
            );
        }
    }

    #[test]
    fn test_cube_sphere_uv_coordinates_range() {
        let model = Model::with_default_material(Sphere::Cube { segments: 4 });
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            assert!(
                vertex.uv.x >= 0.0 && vertex.uv.x <= 1.0,
                "U coordinate should be in [0,1], got {}",
                vertex.uv.x
            );
            assert!(
                vertex.uv.y >= 0.0 && vertex.uv.y <= 1.0,
                "V coordinate should be in [0,1], got {}",
                vertex.uv.y
            );
        }
    }

    #[test]
    fn test_slices_sphere_bottom_v_is_zero() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        let mesh = model.to_polygon_mesh().unwrap();

        // Check that the bottom vertices (first slice) have v close to 0
        // The first 16 faces are triangles at the bottom
        // Each has 3 vertices, and one of them should have v = 0
        let mut found_bottom = false;
        for vertex in mesh.vertices.iter().take(48) {
            // First 16 triangles * 3 vertices
            if vertex.uv.y < 0.01 {
                found_bottom = true;
                break;
            }
        }
        assert!(
            found_bottom,
            "Should have vertices at bottom with v close to 0"
        );
    }

    #[test]
    fn test_slices_sphere_top_v_is_one() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 8,
            segments: 16,
        });
        let mesh = model.to_polygon_mesh().unwrap();

        // Check that some vertices have v close to 1
        let mut found_top = false;
        for vertex in &mesh.vertices {
            if vertex.uv.y > 0.99 {
                found_top = true;
                break;
            }
        }
        assert!(found_top, "Should have vertices at top with v close to 1");
    }

    #[test]
    fn test_to_triangular_mesh_slices_sphere() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 4,
            segments: 8,
        });
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let triangular_mesh = result.unwrap();

        // Check that we have geometries
        assert!(!triangular_mesh.geometries.is_empty());

        // Check that each geometry has triangles (indices divisible by 3)
        for geometry in triangular_mesh.geometries.values() {
            assert_eq!(geometry.indices.len() % 3, 0);
        }
    }

    #[test]
    fn test_to_triangular_mesh_cube_sphere() {
        let model = Model::with_default_material(Sphere::Cube { segments: 2 });
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let triangular_mesh = result.unwrap();

        // 6 faces * 2*2 quads = 24 quads = 48 triangles
        // Count total triangles across all geometries
        let total_triangles: usize = triangular_mesh
            .geometries
            .values()
            .map(|g| g.indices.len() / 3)
            .sum();
        assert_eq!(total_triangles, 48);
    }

    #[test]
    fn test_uniform_material() {
        let model = Model::with_uniform_material(Sphere::Cube { segments: 2 }, "gold".to_string());
        let mesh = model.to_polygon_mesh().unwrap();

        for (_, material) in &mesh.material_map {
            assert_eq!(material, "gold");
        }
    }

    #[test]
    fn test_map_material() {
        let mut face_map = HashMap::new();
        face_map.insert(0, "special".to_string());

        let material = Material::Map {
            default: "default".to_string(),
            face_map,
        };

        let model = Model::new(Sphere::Cube { segments: 2 }, material);
        let mesh = model.to_polygon_mesh().unwrap();

        assert_eq!(mesh.material_map.get(&0), Some(&"special".to_string()));
        assert_eq!(mesh.material_map.get(&1), Some(&"default".to_string()));
    }

    #[test]
    fn test_array_material() {
        let materials = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let model = Model::new(Sphere::Cube { segments: 1 }, Material::Array(materials));
        let mesh = model.to_polygon_mesh().unwrap();

        // 6 faces with cyclic materials
        assert_eq!(mesh.material_map.get(&0), Some(&"a".to_string()));
        assert_eq!(mesh.material_map.get(&1), Some(&"b".to_string()));
        assert_eq!(mesh.material_map.get(&2), Some(&"c".to_string()));
        assert_eq!(mesh.material_map.get(&3), Some(&"a".to_string()));
        assert_eq!(mesh.material_map.get(&4), Some(&"b".to_string()));
        assert_eq!(mesh.material_map.get(&5), Some(&"c".to_string()));
    }

    #[test]
    fn test_minimum_slices_sphere() {
        let model = Model::with_default_material(Sphere::Slices {
            slices: 2,
            segments: 3,
        });
        let result = model.to_polygon_mesh();
        assert!(result.is_ok());
        let mesh = result.unwrap();
        // 2 slices with 3 segments: all triangles (top and bottom caps)
        assert_eq!(mesh.polygons.len(), 6);
    }

    #[test]
    fn test_minimum_cube_sphere() {
        let model = Model::with_default_material(Sphere::Cube { segments: 1 });
        let result = model.to_polygon_mesh();
        assert!(result.is_ok());
        let mesh = result.unwrap();
        // 6 faces * 1*1 = 6 quads
        assert_eq!(mesh.polygons.len(), 6);
    }

    // Winding order tests

    mod winding_order_tests {
        use super::*;

        fn check_counter_clockwise_winding(mesh: &PolygonMesh) -> bool {
            for polygon in &mesh.polygons {
                if polygon.indices.len() < 3 {
                    continue;
                }

                // Get first three vertices
                let v0 = &mesh.vertices[polygon.indices[0] as usize];
                let v1 = &mesh.vertices[polygon.indices[1] as usize];
                let v2 = &mesh.vertices[polygon.indices[2] as usize];

                // Calculate face normal using cross product
                let edge1 = v1.position - v0.position;
                let edge2 = v2.position - v0.position;
                let computed_normal = edge1.cross(edge2).normalize();

                // For a sphere, the polygon normal should point outward (same direction as position)
                let centroid = (v0.position + v1.position + v2.position) / 3.0;
                let expected_direction = centroid.normalize();

                // The computed normal should be in the same hemisphere as the expected direction
                let dot = computed_normal.dot(expected_direction);
                if dot < 0.0 {
                    return false;
                }
            }
            true
        }

        #[test]
        fn test_slices_sphere_winding_order() {
            let model = Model::with_default_material(Sphere::Slices {
                slices: 8,
                segments: 16,
            });
            let mesh = model.to_polygon_mesh().unwrap();
            assert!(
                check_counter_clockwise_winding(&mesh),
                "Slices sphere should have counter-clockwise winding"
            );
        }

        #[test]
        fn test_cube_sphere_winding_order() {
            let model = Model::with_default_material(Sphere::Cube { segments: 4 });
            let mesh = model.to_polygon_mesh().unwrap();
            assert!(
                check_counter_clockwise_winding(&mesh),
                "Cube sphere should have counter-clockwise winding"
            );
        }
    }

    // Serde tests

    mod serde_tests {
        use super::*;

        #[test]
        fn test_sphere_slices_serialize_deserialize() {
            let sphere = Sphere::Slices {
                slices: 16,
                segments: 32,
            };
            let json = serde_json::to_string(&sphere).unwrap();
            let deserialized: Sphere = serde_json::from_str(&json).unwrap();
            assert_eq!(sphere, deserialized);
        }

        #[test]
        fn test_sphere_cube_serialize_deserialize() {
            let sphere = Sphere::Cube { segments: 8 };
            let json = serde_json::to_string(&sphere).unwrap();
            let deserialized: Sphere = serde_json::from_str(&json).unwrap();
            assert_eq!(sphere, deserialized);
        }

        #[test]
        fn test_model_serialize_deserialize() {
            let model = Model::new(
                Sphere::Slices {
                    slices: 12,
                    segments: 24,
                },
                Material::Uniform("texture".to_string()),
            );
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, deserialized);
        }

        #[test]
        fn test_model_with_map_material_serialize_deserialize() {
            let mut face_map = HashMap::new();
            face_map.insert(0, "front".to_string());
            face_map.insert(1, "back".to_string());

            let model = Model::new(
                Sphere::Cube { segments: 4 },
                Material::Map {
                    default: "default".to_string(),
                    face_map,
                },
            );
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, deserialized);
        }
    }

    // Hash tests

    mod hash_tests {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn test_sphere_hash_equality() {
            let sphere1 = Sphere::Slices {
                slices: 8,
                segments: 16,
            };
            let sphere2 = Sphere::Slices {
                slices: 8,
                segments: 16,
            };

            let mut set = HashSet::new();
            set.insert(sphere1);

            assert!(set.contains(&sphere2));
        }

        #[test]
        fn test_sphere_hash_different_values() {
            let sphere1 = Sphere::Slices {
                slices: 8,
                segments: 16,
            };
            let sphere2 = Sphere::Slices {
                slices: 16,
                segments: 32,
            };
            let sphere3 = Sphere::Cube { segments: 8 };

            let mut set = HashSet::new();
            set.insert(sphere1);
            set.insert(sphere2);
            set.insert(sphere3);

            assert_eq!(set.len(), 3);
        }
    }
}
