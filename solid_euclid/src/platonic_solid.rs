use std::collections::HashMap;

use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

use crate::geometry_error::GeometryError;
use crate::geometry_model::GeometryModel;
use crate::polygon_mesh::{Polygon, PolygonMesh};
use crate::vertex::Vertex;

/// Represents the five Platonic solids - convex regular polyhedra with equivalent faces
/// composed of congruent convex regular polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatonicSolid {
    /// 4 triangular faces
    Tetrahedron,
    /// 6 square faces (cube)
    Hexahedron,
    /// 8 triangular faces
    Octahedron,
    /// 12 pentagonal faces
    Dodecahedron,
    /// 20 triangular faces
    Icosahedron,
}

/// Represents material assignment strategies for geometry faces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Material {
    /// Default material with no specific assignment
    Default,
    /// Uniform material applied to all faces
    Uniform(String),
    /// Material mapping with a default material and a map associating face indices to materials
    Map {
        default: String,
        face_map: HashMap<i32, String>,
    },
    /// Array of materials, one per face
    Array(Vec<String>),
}

/// A model combining a Platonic solid geometry with material assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub geometry: PlatonicSolid,
    pub material: Material,
}

impl Model {
    /// Creates a new model with the specified geometry and material.
    pub fn new(geometry: PlatonicSolid, material: Material) -> Self {
        Self { geometry, material }
    }

    /// Creates a new model with default material.
    pub fn with_default_material(geometry: PlatonicSolid) -> Self {
        Self {
            geometry,
            material: Material::Default,
        }
    }

    /// Creates a new model with uniform material.
    pub fn with_uniform_material(geometry: PlatonicSolid, material: String) -> Self {
        Self {
            geometry,
            material: Material::Uniform(material),
        }
    }

    /// Gets the material name for a face index based on the material assignment.
    fn get_material_for_face(&self, face_index: i32, _total_faces: i32) -> String {
        match &self.material {
            Material::Default => format!("material_{}", face_index),
            Material::Uniform(name) => name.clone(),
            Material::Map { default, face_map } => face_map
                .get(&face_index)
                .cloned()
                .unwrap_or_else(|| default.clone()),
            Material::Array(arr) => {
                if arr.is_empty() {
                    format!("material_{}", face_index)
                } else {
                    // Use cyclic indexing to reuse materials when there are more faces than materials
                    arr[(face_index as usize) % arr.len()].clone()
                }
            }
        }
    }

    /// Sets up materials in the polygon mesh based on the material assignment.
    fn setup_materials(&self, mesh: &mut PolygonMesh, total_faces: i32) {
        for face_index in 0..total_faces {
            let material_name = self.get_material_for_face(face_index, total_faces);
            mesh.set_material(face_index, material_name);
        }
    }
}

impl GeometryModel for Model {
    fn to_polygon_mesh(&self) -> Result<PolygonMesh, GeometryError> {
        match self.geometry {
            PlatonicSolid::Tetrahedron => self.create_tetrahedron(),
            PlatonicSolid::Hexahedron => self.create_hexahedron(),
            PlatonicSolid::Octahedron => self.create_octahedron(),
            PlatonicSolid::Dodecahedron => self.create_dodecahedron(),
            PlatonicSolid::Icosahedron => self.create_icosahedron(),
        }
    }
}

impl Model {
    /// Creates a tetrahedron polygon mesh.
    /// 4 vertices, 4 triangular faces.
    fn create_tetrahedron(&self) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Tetrahedron vertices
        let s_8_9: f64 = (8.0_f64 / 9.0).sqrt();
        let s_2_9: f64 = (2.0_f64 / 9.0).sqrt();
        let s_2_3: f64 = (2.0_f64 / 3.0).sqrt();
        let s_1_3: f64 = 1.0 / 3.0;

        let vertices = [
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::new(s_8_9, 0.0, -s_1_3),
            DVec3::new(-s_2_9, s_2_3, -s_1_3),
            DVec3::new(-s_2_9, -s_2_3, -s_1_3),
        ];

        // Face indices and opposite vertex for normal calculation
        let faces = [
            ([0, 1, 2], 3), // Face 0: opposite to vertex 3
            ([0, 2, 3], 1), // Face 1: opposite to vertex 1
            ([0, 3, 1], 2), // Face 2: opposite to vertex 2
            ([1, 3, 2], 0), // Face 3: opposite to vertex 0
        ];

        // UV coordinates for triangular faces
        let uvs = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 1.0),
        ];

        for (face_index, (face_indices, opposite_vertex)) in faces.iter().enumerate() {
            // Calculate face normal (pointing away from opposite vertex)
            let normal = -vertices[*opposite_vertex].normalize();

            // Add vertices for this face
            let mut polygon_indices = Vec::new();
            for (i, &vi) in face_indices.iter().enumerate() {
                let vertex = Vertex::new(vertices[vi], normal, uvs[i]);
                let idx = mesh.add_vertex(vertex);
                polygon_indices.push(idx);
            }

            mesh.add_polygon(Polygon::new(polygon_indices, normal, face_index as i32));
        }

        self.setup_materials(&mut mesh, 4);
        Ok(mesh)
    }

    /// Creates a hexahedron (cube) polygon mesh.
    /// 8 vertices, 6 square faces.
    fn create_hexahedron(&self) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Cube vertices (normalized to unit sphere)
        let l = 1.0 / 3.0_f64.sqrt();
        let vertices = [
            DVec3::new(-l, -l, l),  // 0
            DVec3::new(l, -l, l),   // 1
            DVec3::new(l, l, l),    // 2
            DVec3::new(-l, l, l),   // 3
            DVec3::new(-l, -l, -l), // 4
            DVec3::new(l, -l, -l),  // 5
            DVec3::new(l, l, -l),   // 6
            DVec3::new(-l, l, -l),  // 7
        ];

        // Face indices and normals
        let faces = [
            ([0, 1, 2, 3], DVec3::new(0.0, 0.0, 1.0)),  // Front
            ([1, 5, 6, 2], DVec3::new(1.0, 0.0, 0.0)),  // Right
            ([5, 4, 7, 6], DVec3::new(0.0, 0.0, -1.0)), // Back
            ([4, 0, 3, 7], DVec3::new(-1.0, 0.0, 0.0)), // Left
            ([3, 2, 6, 7], DVec3::new(0.0, 1.0, 0.0)),  // Top
            ([1, 0, 4, 5], DVec3::new(0.0, -1.0, 0.0)), // Bottom
        ];

        // UV coordinates for quad faces
        let uvs = [
            DVec2::new(0.0, 1.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 0.0),
        ];

        for (face_index, (face_indices, normal)) in faces.iter().enumerate() {
            let mut polygon_indices = Vec::new();
            for (i, &vi) in face_indices.iter().enumerate() {
                let vertex = Vertex::new(vertices[vi], *normal, uvs[i]);
                let idx = mesh.add_vertex(vertex);
                polygon_indices.push(idx);
            }

            mesh.add_polygon(Polygon::new(polygon_indices, *normal, face_index as i32));
        }

        self.setup_materials(&mut mesh, 6);
        Ok(mesh)
    }

    /// Creates an octahedron polygon mesh.
    /// 6 vertices, 8 triangular faces.
    fn create_octahedron(&self) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Octahedron vertices
        let vertices = [
            DVec3::new(-1.0, 0.0, 0.0), // 0
            DVec3::new(0.0, -1.0, 0.0), // 1
            DVec3::new(0.0, 0.0, -1.0), // 2
            DVec3::new(1.0, 0.0, 0.0),  // 3
            DVec3::new(0.0, 1.0, 0.0),  // 4
            DVec3::new(0.0, 0.0, 1.0),  // 5
        ];

        // Face indices - 8 triangular faces
        let faces = [
            [0, 2, 1], // 0
            [1, 2, 3], // 1
            [3, 2, 4], // 2
            [4, 2, 0], // 3
            [0, 1, 5], // 4
            [1, 3, 5], // 5
            [3, 4, 5], // 6
            [4, 0, 5], // 7
        ];

        // Face normal factors
        let normal_factors = [
            DVec3::new(-1.0, -1.0, -1.0),
            DVec3::new(1.0, -1.0, -1.0),
            DVec3::new(1.0, 1.0, -1.0),
            DVec3::new(-1.0, 1.0, -1.0),
            DVec3::new(-1.0, -1.0, 1.0),
            DVec3::new(1.0, -1.0, 1.0),
            DVec3::new(1.0, 1.0, 1.0),
            DVec3::new(-1.0, 1.0, 1.0),
        ];

        // UV coordinates for triangular faces
        let uvs = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 0.5),
        ];

        for (face_index, face_indices) in faces.iter().enumerate() {
            let normal = normal_factors[face_index].normalize();

            let mut polygon_indices = Vec::new();
            for (i, &vi) in face_indices.iter().enumerate() {
                let vertex = Vertex::new(vertices[vi], normal, uvs[i]);
                let idx = mesh.add_vertex(vertex);
                polygon_indices.push(idx);
            }

            mesh.add_polygon(Polygon::new(polygon_indices, normal, face_index as i32));
        }

        self.setup_materials(&mut mesh, 8);
        Ok(mesh)
    }

    /// Creates a dodecahedron polygon mesh.
    /// 20 vertices, 12 pentagonal faces.
    fn create_dodecahedron(&self) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Golden ratio
        let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let phi2 = phi * phi;
        let a = 1.0;
        let b = 1.0 / phi;
        let c = 1.0 / phi2;

        // Dodecahedron vertices
        let vertices = [
            // Vertices 0-7: (+/-b, +/-b, +/-b) - cube vertices scaled by 1/phi
            DVec3::new(-b, -b, -b), // 0
            DVec3::new(b, -b, -b),  // 1
            DVec3::new(b, b, -b),   // 2
            DVec3::new(-b, b, -b),  // 3
            DVec3::new(-b, -b, b),  // 4
            DVec3::new(b, -b, b),   // 5
            DVec3::new(b, b, b),    // 6
            DVec3::new(-b, b, b),   // 7
            // Vertices 8-11: (0, +/-a, +/-c)
            DVec3::new(0.0, -a, -c), // 8
            DVec3::new(0.0, a, -c),  // 9
            DVec3::new(0.0, a, c),   // 10
            DVec3::new(0.0, -a, c),  // 11
            // Vertices 12-15: (+/-a, +/-c, 0)
            DVec3::new(-a, -c, 0.0), // 12
            DVec3::new(a, -c, 0.0),  // 13
            DVec3::new(a, c, 0.0),   // 14
            DVec3::new(-a, c, 0.0),  // 15
            // Vertices 16-19: (+/-c, 0, +/-a)
            DVec3::new(-c, 0.0, -a), // 16
            DVec3::new(c, 0.0, -a),  // 17
            DVec3::new(c, 0.0, a),   // 18
            DVec3::new(-c, 0.0, a),  // 19
        ];

        // 12 pentagonal faces
        let faces = [
            [16, 17, 1, 8, 0],
            [17, 16, 3, 9, 2],
            [19, 18, 6, 10, 7],
            [18, 19, 4, 11, 5],
            [14, 13, 1, 17, 2],
            [13, 14, 6, 18, 5],
            [15, 12, 4, 19, 7],
            [12, 15, 3, 16, 0],
            [9, 10, 6, 14, 2],
            [10, 9, 3, 15, 7],
            [8, 11, 4, 12, 0],
            [11, 8, 1, 13, 5],
        ];

        // UV coordinates for pentagonal faces (arranged in a pentagon pattern)
        let uvs = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 0.5),
            DVec2::new(0.5, 1.0),
            DVec2::new(0.0, 0.5),
        ];

        for (face_index, face_indices) in faces.iter().enumerate() {
            // Calculate face normal from centroid
            let mut centroid = DVec3::ZERO;
            for &vi in face_indices {
                centroid += vertices[vi];
            }
            let normal = centroid.normalize();

            let mut polygon_indices = Vec::new();
            for (i, &vi) in face_indices.iter().enumerate() {
                let vertex = Vertex::new(vertices[vi], normal, uvs[i]);
                let idx = mesh.add_vertex(vertex);
                polygon_indices.push(idx);
            }

            mesh.add_polygon(Polygon::new(polygon_indices, normal, face_index as i32));
        }

        self.setup_materials(&mut mesh, 12);
        Ok(mesh)
    }

    /// Creates an icosahedron polygon mesh.
    /// 12 vertices, 20 triangular faces.
    fn create_icosahedron(&self) -> Result<PolygonMesh, GeometryError> {
        let mut mesh = PolygonMesh::new();

        // Golden ratio (unused but kept for documentation)
        let _phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;

        // Icosahedron vertices (normalized)
        let vertices = [
            DVec3::new(0.0, 0.0, 1.0).normalize(),
            DVec3::new(0.894, 0.0, 0.447).normalize(),
            DVec3::new(0.276, 0.851, 0.447).normalize(),
            DVec3::new(-0.724, 0.526, 0.447).normalize(),
            DVec3::new(-0.724, -0.526, 0.447).normalize(),
            DVec3::new(0.276, -0.851, 0.447).normalize(),
            DVec3::new(0.724, 0.526, -0.447).normalize(),
            DVec3::new(-0.276, 0.851, -0.447).normalize(),
            DVec3::new(-0.894, 0.0, -0.447).normalize(),
            DVec3::new(-0.276, -0.851, -0.447).normalize(),
            DVec3::new(0.724, -0.526, -0.447).normalize(),
            DVec3::new(0.0, 0.0, -1.0).normalize(),
        ];

        // 20 triangular faces
        let faces = [
            [1, 2, 0],
            [2, 3, 0],
            [3, 4, 0],
            [4, 5, 0],
            [5, 1, 0],
            [7, 6, 11],
            [8, 7, 11],
            [9, 8, 11],
            [10, 9, 11],
            [6, 10, 11],
            [1, 6, 2],
            [2, 7, 3],
            [3, 8, 4],
            [4, 9, 5],
            [5, 10, 1],
            [7, 2, 6],
            [8, 3, 7],
            [9, 4, 8],
            [10, 5, 9],
            [6, 1, 10],
        ];

        // UV coordinates for triangular faces
        let uvs = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 1.0),
        ];

        for (face_index, face_indices) in faces.iter().enumerate() {
            // Calculate face normal from centroid
            let mut centroid = DVec3::ZERO;
            for &vi in face_indices {
                centroid += vertices[vi];
            }
            let normal = centroid.normalize();

            let mut polygon_indices = Vec::new();
            for (i, &vi) in face_indices.iter().enumerate() {
                let vertex = Vertex::new(vertices[vi], normal, uvs[i]);
                let idx = mesh.add_vertex(vertex);
                polygon_indices.push(idx);
            }

            mesh.add_polygon(Polygon::new(polygon_indices, normal, face_index as i32));
        }

        self.setup_materials(&mut mesh, 20);
        Ok(mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platonic_solid_enum_variants() {
        let tetrahedron = PlatonicSolid::Tetrahedron;
        let hexahedron = PlatonicSolid::Hexahedron;
        let octahedron = PlatonicSolid::Octahedron;
        let dodecahedron = PlatonicSolid::Dodecahedron;
        let icosahedron = PlatonicSolid::Icosahedron;

        assert_eq!(tetrahedron, PlatonicSolid::Tetrahedron);
        assert_eq!(hexahedron, PlatonicSolid::Hexahedron);
        assert_eq!(octahedron, PlatonicSolid::Octahedron);
        assert_eq!(dodecahedron, PlatonicSolid::Dodecahedron);
        assert_eq!(icosahedron, PlatonicSolid::Icosahedron);
    }

    #[test]
    fn test_platonic_solid_clone_and_copy() {
        let original = PlatonicSolid::Tetrahedron;
        let cloned = original.clone();
        let copied = original;

        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }

    #[test]
    fn test_material_default() {
        let material = Material::Default;
        assert_eq!(material, Material::Default);
    }

    #[test]
    fn test_material_uniform() {
        let material = Material::Uniform("red".to_string());
        if let Material::Uniform(name) = material {
            assert_eq!(name, "red");
        } else {
            panic!("Expected Material::Uniform");
        }
    }

    #[test]
    fn test_material_map() {
        let mut face_map = HashMap::new();
        face_map.insert(0, "front".to_string());
        face_map.insert(1, "back".to_string());

        let material = Material::Map {
            default: "default".to_string(),
            face_map,
        };

        if let Material::Map {
            default,
            face_map,
        } = material
        {
            assert_eq!(default, "default");
            assert_eq!(face_map.get(&0), Some(&"front".to_string()));
            assert_eq!(face_map.get(&1), Some(&"back".to_string()));
        } else {
            panic!("Expected Material::Map");
        }
    }

    #[test]
    fn test_material_array() {
        let materials = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
        let material = Material::Array(materials.clone());

        if let Material::Array(arr) = material {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], "red");
            assert_eq!(arr[1], "green");
            assert_eq!(arr[2], "blue");
        } else {
            panic!("Expected Material::Array");
        }
    }

    #[test]
    fn test_model_new() {
        let model = Model::new(PlatonicSolid::Hexahedron, Material::Default);
        assert_eq!(model.geometry, PlatonicSolid::Hexahedron);
        assert_eq!(model.material, Material::Default);
    }

    #[test]
    fn test_model_with_default_material() {
        let model = Model::with_default_material(PlatonicSolid::Icosahedron);
        assert_eq!(model.geometry, PlatonicSolid::Icosahedron);
        assert_eq!(model.material, Material::Default);
    }

    #[test]
    fn test_model_with_uniform_material() {
        let model = Model::with_uniform_material(PlatonicSolid::Octahedron, "gold".to_string());
        assert_eq!(model.geometry, PlatonicSolid::Octahedron);
        assert_eq!(model.material, Material::Uniform("gold".to_string()));
    }

    #[test]
    fn test_model_with_map_material() {
        let mut face_map = HashMap::new();
        face_map.insert(0, "material_a".to_string());
        face_map.insert(1, "material_b".to_string());

        let material = Material::Map {
            default: "base".to_string(),
            face_map,
        };

        let model = Model::new(PlatonicSolid::Dodecahedron, material);
        assert_eq!(model.geometry, PlatonicSolid::Dodecahedron);

        if let Material::Map {
            default,
            face_map,
        } = &model.material
        {
            assert_eq!(default, "base");
            assert_eq!(face_map.len(), 2);
        } else {
            panic!("Expected Material::Map");
        }
    }

    #[test]
    fn test_model_with_array_material() {
        let materials = vec![
            "mat1".to_string(),
            "mat2".to_string(),
            "mat3".to_string(),
            "mat4".to_string(),
        ];

        let model = Model::new(PlatonicSolid::Tetrahedron, Material::Array(materials));
        assert_eq!(model.geometry, PlatonicSolid::Tetrahedron);

        if let Material::Array(arr) = &model.material {
            assert_eq!(arr.len(), 4);
        } else {
            panic!("Expected Material::Array");
        }
    }

    #[test]
    fn test_model_clone() {
        let model = Model::new(
            PlatonicSolid::Hexahedron,
            Material::Uniform("wood".to_string()),
        );
        let cloned = model.clone();

        assert_eq!(model, cloned);
    }

    #[test]
    fn test_platonic_solid_debug() {
        let solid = PlatonicSolid::Tetrahedron;
        let debug_str = format!("{:?}", solid);
        assert_eq!(debug_str, "Tetrahedron");
    }

    #[test]
    fn test_material_debug() {
        let material = Material::Uniform("test".to_string());
        let debug_str = format!("{:?}", material);
        assert!(debug_str.contains("Uniform"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_model_debug() {
        let model = Model::with_default_material(PlatonicSolid::Icosahedron);
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("Icosahedron"));
        assert!(debug_str.contains("Default"));
    }

    // GeometryModel trait implementation tests

    #[test]
    fn test_tetrahedron_to_polygon_mesh() {
        let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Tetrahedron has 4 faces, each with 3 vertices
        assert_eq!(mesh.polygons.len(), 4);
        assert_eq!(mesh.vertices.len(), 12); // 4 faces * 3 vertices

        // Each polygon should be a triangle
        for polygon in &mesh.polygons {
            assert_eq!(polygon.indices.len(), 3);
        }

        // Check materials are set
        assert_eq!(mesh.material_map.len(), 4);
    }

    #[test]
    fn test_hexahedron_to_polygon_mesh() {
        let model = Model::with_default_material(PlatonicSolid::Hexahedron);
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Hexahedron has 6 faces, each with 4 vertices
        assert_eq!(mesh.polygons.len(), 6);
        assert_eq!(mesh.vertices.len(), 24); // 6 faces * 4 vertices

        // Each polygon should be a quad
        for polygon in &mesh.polygons {
            assert_eq!(polygon.indices.len(), 4);
        }

        // Check materials are set
        assert_eq!(mesh.material_map.len(), 6);
    }

    #[test]
    fn test_octahedron_to_polygon_mesh() {
        let model = Model::with_default_material(PlatonicSolid::Octahedron);
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Octahedron has 8 faces, each with 3 vertices
        assert_eq!(mesh.polygons.len(), 8);
        assert_eq!(mesh.vertices.len(), 24); // 8 faces * 3 vertices

        // Each polygon should be a triangle
        for polygon in &mesh.polygons {
            assert_eq!(polygon.indices.len(), 3);
        }

        // Check materials are set
        assert_eq!(mesh.material_map.len(), 8);
    }

    #[test]
    fn test_dodecahedron_to_polygon_mesh() {
        let model = Model::with_default_material(PlatonicSolid::Dodecahedron);
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Dodecahedron has 12 faces, each with 5 vertices (pentagons)
        assert_eq!(mesh.polygons.len(), 12);
        assert_eq!(mesh.vertices.len(), 60); // 12 faces * 5 vertices

        // Each polygon should be a pentagon
        for polygon in &mesh.polygons {
            assert_eq!(polygon.indices.len(), 5);
        }

        // Check materials are set
        assert_eq!(mesh.material_map.len(), 12);
    }

    #[test]
    fn test_icosahedron_to_polygon_mesh() {
        let model = Model::with_default_material(PlatonicSolid::Icosahedron);
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Icosahedron has 20 faces, each with 3 vertices
        assert_eq!(mesh.polygons.len(), 20);
        assert_eq!(mesh.vertices.len(), 60); // 20 faces * 3 vertices

        // Each polygon should be a triangle
        for polygon in &mesh.polygons {
            assert_eq!(polygon.indices.len(), 3);
        }

        // Check materials are set
        assert_eq!(mesh.material_map.len(), 20);
    }

    #[test]
    fn test_to_triangular_mesh_tetrahedron() {
        let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Tetrahedron has 4 triangular faces, so 4 materials
        assert_eq!(mesh.len(), 4);

        // Each geometry should have 3 vertices (one triangle per face)
        for (_, geometry) in &mesh.geometries {
            assert_eq!(geometry.vertex_count(), 3);
            assert_eq!(geometry.indices.len(), 3);
        }
    }

    #[test]
    fn test_to_triangular_mesh_hexahedron() {
        let model = Model::with_default_material(PlatonicSolid::Hexahedron);
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Hexahedron has 6 quad faces -> 6 materials
        assert_eq!(mesh.len(), 6);

        // Each quad becomes 2 triangles
        for (_, geometry) in &mesh.geometries {
            assert_eq!(geometry.vertex_count(), 4);
            assert_eq!(geometry.indices.len(), 6); // 2 triangles * 3 indices
        }
    }

    #[test]
    fn test_to_triangular_mesh_dodecahedron() {
        let model = Model::with_default_material(PlatonicSolid::Dodecahedron);
        let result = model.to_triangular_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // Dodecahedron has 12 pentagonal faces -> 12 materials
        assert_eq!(mesh.len(), 12);

        // Each pentagon becomes 3 triangles (n-2 triangles for n-gon)
        for (_, geometry) in &mesh.geometries {
            assert_eq!(geometry.vertex_count(), 5);
            assert_eq!(geometry.indices.len(), 9); // 3 triangles * 3 indices
        }
    }

    #[test]
    fn test_uniform_material() {
        let model = Model::with_uniform_material(PlatonicSolid::Tetrahedron, "gold".to_string());
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        // All faces should have the same material
        for face_id in 0..4 {
            assert_eq!(mesh.get_material(face_id), Some(&"gold".to_string()));
        }
    }

    #[test]
    fn test_map_material() {
        let mut face_map = HashMap::new();
        face_map.insert(0, "red".to_string());
        face_map.insert(2, "blue".to_string());

        let material = Material::Map {
            default: "gray".to_string(),
            face_map,
        };

        let model = Model::new(PlatonicSolid::Tetrahedron, material);
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        assert_eq!(mesh.get_material(0), Some(&"red".to_string()));
        assert_eq!(mesh.get_material(1), Some(&"gray".to_string()));
        assert_eq!(mesh.get_material(2), Some(&"blue".to_string()));
        assert_eq!(mesh.get_material(3), Some(&"gray".to_string()));
    }

    #[test]
    fn test_array_material() {
        let materials = vec![
            "mat0".to_string(),
            "mat1".to_string(),
            "mat2".to_string(),
            "mat3".to_string(),
        ];

        let model = Model::new(PlatonicSolid::Tetrahedron, Material::Array(materials));
        let result = model.to_polygon_mesh();

        assert!(result.is_ok());
        let mesh = result.unwrap();

        assert_eq!(mesh.get_material(0), Some(&"mat0".to_string()));
        assert_eq!(mesh.get_material(1), Some(&"mat1".to_string()));
        assert_eq!(mesh.get_material(2), Some(&"mat2".to_string()));
        assert_eq!(mesh.get_material(3), Some(&"mat3".to_string()));
    }

    #[test]
    fn test_vertex_positions_tetrahedron() {
        let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
        let mesh = model.to_polygon_mesh().unwrap();

        // All vertices should be on a unit sphere (approximately)
        for vertex in &mesh.vertices {
            let length = vertex.position.length();
            assert!((length - 1.0).abs() < 0.01, "Vertex not on unit sphere: {}", length);
        }
    }

    #[test]
    fn test_vertex_positions_hexahedron() {
        let model = Model::with_default_material(PlatonicSolid::Hexahedron);
        let mesh = model.to_polygon_mesh().unwrap();

        // All vertices should be at the same distance from origin
        let expected_length = 1.0 / 3.0_f64.sqrt() * 3.0_f64.sqrt();
        for vertex in &mesh.vertices {
            let length = vertex.position.length();
            assert!((length - expected_length).abs() < 0.01, "Unexpected vertex distance: {}", length);
        }
    }

    #[test]
    fn test_vertex_normals_are_normalized() {
        let solids = [
            PlatonicSolid::Tetrahedron,
            PlatonicSolid::Hexahedron,
            PlatonicSolid::Octahedron,
            PlatonicSolid::Dodecahedron,
            PlatonicSolid::Icosahedron,
        ];

        for solid in solids {
            let model = Model::with_default_material(solid);
            let mesh = model.to_polygon_mesh().unwrap();

            for vertex in &mesh.vertices {
                let length = vertex.normal.length();
                assert!(
                    (length - 1.0).abs() < 0.001,
                    "Normal not normalized for {:?}: {}",
                    solid,
                    length
                );
            }
        }
    }

    #[test]
    fn test_polygon_normals_are_normalized() {
        let solids = [
            PlatonicSolid::Tetrahedron,
            PlatonicSolid::Hexahedron,
            PlatonicSolid::Octahedron,
            PlatonicSolid::Dodecahedron,
            PlatonicSolid::Icosahedron,
        ];

        for solid in solids {
            let model = Model::with_default_material(solid);
            let mesh = model.to_polygon_mesh().unwrap();

            for polygon in &mesh.polygons {
                let length = polygon.normal.length();
                assert!(
                    (length - 1.0).abs() < 0.001,
                    "Polygon normal not normalized for {:?}: {}",
                    solid,
                    length
                );
            }
        }
    }

    #[test]
    fn test_icosahedron_vertices_on_unit_sphere() {
        let model = Model::with_default_material(PlatonicSolid::Icosahedron);
        let mesh = model.to_polygon_mesh().unwrap();

        for vertex in &mesh.vertices {
            let length = vertex.position.length();
            assert!(
                (length - 1.0).abs() < 0.01,
                "Icosahedron vertex not on unit sphere: {}",
                length
            );
        }
    }

    // Winding order tests for Platonic solids

    mod winding_order_tests {
        use super::*;

        /// Helper function to calculate the winding order normal from polygon vertices.
        /// Uses the cross product of the first two edges (right-hand rule).
        /// Returns the calculated normal vector.
        fn calculate_winding_normal(mesh: &PolygonMesh, polygon: &Polygon) -> DVec3 {
            let indices = &polygon.indices;
            assert!(indices.len() >= 3, "Polygon must have at least 3 vertices");

            let p0 = mesh.vertices[indices[0] as usize].position;
            let p1 = mesh.vertices[indices[1] as usize].position;
            let p2 = mesh.vertices[indices[2] as usize].position;

            let v1 = p1 - p0;
            let v2 = p2 - p0;
            v1.cross(v2).normalize()
        }

        /// Verifies that the winding order normal matches the stored polygon normal.
        /// This ensures consistent CCW winding when viewed from the outside.
        fn verify_winding_order_matches_normal(mesh: &PolygonMesh, solid_name: &str) {
            for (face_idx, polygon) in mesh.polygons.iter().enumerate() {
                let winding_normal = calculate_winding_normal(mesh, polygon);
                let stored_normal = polygon.normal;

                // The dot product should be positive (same direction)
                let dot = winding_normal.dot(stored_normal);
                assert!(
                    dot > 0.99,
                    "{} face {}: winding normal {:?} does not match stored normal {:?} (dot = {})",
                    solid_name,
                    face_idx,
                    winding_normal,
                    stored_normal,
                    dot
                );
            }
        }

        /// Verifies that all face normals point outward from the centroid.
        /// For convex solids centered at origin, the normal should point away from center.
        fn verify_normals_point_outward(mesh: &PolygonMesh, solid_name: &str) {
            for (face_idx, polygon) in mesh.polygons.iter().enumerate() {
                // Calculate face centroid
                let mut face_centroid = DVec3::ZERO;
                for &idx in &polygon.indices {
                    face_centroid += mesh.vertices[idx as usize].position;
                }
                face_centroid /= polygon.indices.len() as f64;

                // The normal should point in the same direction as the centroid
                // (for solids centered at origin)
                let dot = polygon.normal.dot(face_centroid.normalize());
                assert!(
                    dot > 0.5,
                    "{} face {}: normal {:?} does not point outward (centroid: {:?}, dot = {})",
                    solid_name,
                    face_idx,
                    polygon.normal,
                    face_centroid,
                    dot
                );
            }
        }

        /// Verifies that the winding order produces an outward-pointing normal.
        /// Combines both checks: winding order should produce normal pointing outward.
        fn verify_winding_produces_outward_normal(mesh: &PolygonMesh, solid_name: &str) {
            for (face_idx, polygon) in mesh.polygons.iter().enumerate() {
                let winding_normal = calculate_winding_normal(mesh, polygon);

                // Calculate face centroid
                let mut face_centroid = DVec3::ZERO;
                for &idx in &polygon.indices {
                    face_centroid += mesh.vertices[idx as usize].position;
                }
                face_centroid /= polygon.indices.len() as f64;

                // The winding normal should point in the same direction as the centroid
                // (for solids centered at origin)
                let dot = winding_normal.dot(face_centroid.normalize());
                assert!(
                    dot > 0.5,
                    "{} face {}: winding produces inward normal (winding_normal: {:?}, centroid: {:?}, dot = {})",
                    solid_name,
                    face_idx,
                    winding_normal,
                    face_centroid,
                    dot
                );
            }
        }

        #[test]
        fn test_tetrahedron_winding_order() {
            let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
            let mesh = model.to_polygon_mesh().unwrap();

            verify_winding_order_matches_normal(&mesh, "Tetrahedron");
            verify_normals_point_outward(&mesh, "Tetrahedron");
            verify_winding_produces_outward_normal(&mesh, "Tetrahedron");
        }

        #[test]
        fn test_hexahedron_winding_order() {
            let model = Model::with_default_material(PlatonicSolid::Hexahedron);
            let mesh = model.to_polygon_mesh().unwrap();

            verify_winding_order_matches_normal(&mesh, "Hexahedron");
            verify_normals_point_outward(&mesh, "Hexahedron");
            verify_winding_produces_outward_normal(&mesh, "Hexahedron");
        }

        #[test]
        fn test_octahedron_winding_order() {
            let model = Model::with_default_material(PlatonicSolid::Octahedron);
            let mesh = model.to_polygon_mesh().unwrap();

            verify_winding_order_matches_normal(&mesh, "Octahedron");
            verify_normals_point_outward(&mesh, "Octahedron");
            verify_winding_produces_outward_normal(&mesh, "Octahedron");
        }

        #[test]
        fn test_dodecahedron_winding_order() {
            let model = Model::with_default_material(PlatonicSolid::Dodecahedron);
            let mesh = model.to_polygon_mesh().unwrap();

            verify_winding_order_matches_normal(&mesh, "Dodecahedron");
            verify_normals_point_outward(&mesh, "Dodecahedron");
            verify_winding_produces_outward_normal(&mesh, "Dodecahedron");
        }

        #[test]
        fn test_icosahedron_winding_order() {
            let model = Model::with_default_material(PlatonicSolid::Icosahedron);
            let mesh = model.to_polygon_mesh().unwrap();

            verify_winding_order_matches_normal(&mesh, "Icosahedron");
            verify_normals_point_outward(&mesh, "Icosahedron");
            verify_winding_produces_outward_normal(&mesh, "Icosahedron");
        }

        #[test]
        fn test_all_platonic_solids_consistent_winding() {
            let solids = [
                (PlatonicSolid::Tetrahedron, "Tetrahedron"),
                (PlatonicSolid::Hexahedron, "Hexahedron"),
                (PlatonicSolid::Octahedron, "Octahedron"),
                (PlatonicSolid::Dodecahedron, "Dodecahedron"),
                (PlatonicSolid::Icosahedron, "Icosahedron"),
            ];

            for (solid, name) in solids {
                let model = Model::with_default_material(solid);
                let mesh = model.to_polygon_mesh().unwrap();

                verify_winding_order_matches_normal(&mesh, name);
                verify_normals_point_outward(&mesh, name);
                verify_winding_produces_outward_normal(&mesh, name);
            }
        }

        /// Debug test that prints detailed winding information for inspection
        #[test]
        fn test_print_winding_details() {
            let solids = [
                (PlatonicSolid::Tetrahedron, "Tetrahedron", 4),
                (PlatonicSolid::Hexahedron, "Hexahedron", 6),
                (PlatonicSolid::Octahedron, "Octahedron", 8),
                (PlatonicSolid::Dodecahedron, "Dodecahedron", 12),
                (PlatonicSolid::Icosahedron, "Icosahedron", 20),
            ];

            for (solid, name, expected_faces) in solids {
                let model = Model::with_default_material(solid);
                let mesh = model.to_polygon_mesh().unwrap();

                println!("\n{} ({} faces):", name, expected_faces);
                assert_eq!(mesh.polygons.len(), expected_faces);

                for (face_idx, polygon) in mesh.polygons.iter().enumerate() {
                    let winding_normal = calculate_winding_normal(&mesh, polygon);
                    let stored_normal = polygon.normal;

                    // Calculate face centroid
                    let mut face_centroid = DVec3::ZERO;
                    for &idx in &polygon.indices {
                        face_centroid += mesh.vertices[idx as usize].position;
                    }
                    face_centroid /= polygon.indices.len() as f64;
                    let outward_dir = face_centroid.normalize();

                    let dot_stored_winding = winding_normal.dot(stored_normal);
                    let dot_stored_outward = stored_normal.dot(outward_dir);
                    let dot_winding_outward = winding_normal.dot(outward_dir);

                    let status = if dot_winding_outward > 0.5 { "OK" } else { "WRONG" };

                    println!(
                        "  Face {}: winding·stored={:.3}, stored·outward={:.3}, winding·outward={:.3} [{}]",
                        face_idx, dot_stored_winding, dot_stored_outward, dot_winding_outward, status
                    );
                }
            }
        }
    }

    // GeometryHash tests for Platonic solids

    mod hash_tests {
        use super::*;
        use crate::geometry_hash::GeometryHash;

        #[test]
        fn test_tetrahedron_polygon_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
            let mesh = model.to_polygon_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Tetrahedron PolygonMesh hash: {}", hash_str);
        }

        #[test]
        fn test_hexahedron_polygon_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Hexahedron);
            let mesh = model.to_polygon_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Hexahedron PolygonMesh hash: {}", hash_str);
        }

        #[test]
        fn test_octahedron_polygon_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Octahedron);
            let mesh = model.to_polygon_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Octahedron PolygonMesh hash: {}", hash_str);
        }

        #[test]
        fn test_dodecahedron_polygon_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Dodecahedron);
            let mesh = model.to_polygon_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Dodecahedron PolygonMesh hash: {}", hash_str);
        }

        #[test]
        fn test_icosahedron_polygon_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Icosahedron);
            let mesh = model.to_polygon_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Icosahedron PolygonMesh hash: {}", hash_str);
        }

        #[test]
        fn test_tetrahedron_triangular_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
            let mesh = model.to_triangular_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Tetrahedron TriangularMesh hash: {}", hash_str);
        }

        #[test]
        fn test_hexahedron_triangular_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Hexahedron);
            let mesh = model.to_triangular_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Hexahedron TriangularMesh hash: {}", hash_str);
        }

        #[test]
        fn test_octahedron_triangular_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Octahedron);
            let mesh = model.to_triangular_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Octahedron TriangularMesh hash: {}", hash_str);
        }

        #[test]
        fn test_dodecahedron_triangular_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Dodecahedron);
            let mesh = model.to_triangular_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Dodecahedron TriangularMesh hash: {}", hash_str);
        }

        #[test]
        fn test_icosahedron_triangular_mesh_hash() {
            let model = Model::with_default_material(PlatonicSolid::Icosahedron);
            let mesh = model.to_triangular_mesh().unwrap();
            let hash = mesh.geometry_hash();

            assert!(hash.is_ok());
            let hash_str = hash.unwrap();
            assert_eq!(hash_str.len(), 64);
            println!("Icosahedron TriangularMesh hash: {}", hash_str);
        }

        #[test]
        fn test_all_platonic_solids_have_unique_polygon_mesh_hashes() {
            let solids = [
                PlatonicSolid::Tetrahedron,
                PlatonicSolid::Hexahedron,
                PlatonicSolid::Octahedron,
                PlatonicSolid::Dodecahedron,
                PlatonicSolid::Icosahedron,
            ];

            let hashes: Vec<String> = solids
                .iter()
                .map(|solid| {
                    let model = Model::with_default_material(*solid);
                    model.to_polygon_mesh().unwrap().geometry_hash().unwrap()
                })
                .collect();

            // Verify all hashes are unique
            for i in 0..hashes.len() {
                for j in (i + 1)..hashes.len() {
                    assert_ne!(
                        hashes[i], hashes[j],
                        "Hash collision between {:?} and {:?}",
                        solids[i], solids[j]
                    );
                }
            }
        }

        #[test]
        fn test_all_platonic_solids_have_unique_triangular_mesh_hashes() {
            let solids = [
                PlatonicSolid::Tetrahedron,
                PlatonicSolid::Hexahedron,
                PlatonicSolid::Octahedron,
                PlatonicSolid::Dodecahedron,
                PlatonicSolid::Icosahedron,
            ];

            let hashes: Vec<String> = solids
                .iter()
                .map(|solid| {
                    let model = Model::with_default_material(*solid);
                    model.to_triangular_mesh().unwrap().geometry_hash().unwrap()
                })
                .collect();

            // Verify all hashes are unique
            for i in 0..hashes.len() {
                for j in (i + 1)..hashes.len() {
                    assert_ne!(
                        hashes[i], hashes[j],
                        "Hash collision between {:?} and {:?}",
                        solids[i], solids[j]
                    );
                }
            }
        }

        #[test]
        fn test_polygon_mesh_hash_is_deterministic_for_all_solids() {
            let solids = [
                PlatonicSolid::Tetrahedron,
                PlatonicSolid::Hexahedron,
                PlatonicSolid::Octahedron,
                PlatonicSolid::Dodecahedron,
                PlatonicSolid::Icosahedron,
            ];

            for solid in solids {
                let model = Model::with_default_material(solid);
                let mesh1 = model.to_polygon_mesh().unwrap();
                let mesh2 = model.to_polygon_mesh().unwrap();

                let hash1 = mesh1.geometry_hash().unwrap();
                let hash2 = mesh2.geometry_hash().unwrap();

                assert_eq!(hash1, hash2, "Hash not deterministic for {:?}", solid);
            }
        }

        #[test]
        fn test_triangular_mesh_hash_is_deterministic_for_all_solids() {
            let solids = [
                PlatonicSolid::Tetrahedron,
                PlatonicSolid::Hexahedron,
                PlatonicSolid::Octahedron,
                PlatonicSolid::Dodecahedron,
                PlatonicSolid::Icosahedron,
            ];

            for solid in solids {
                let model = Model::with_default_material(solid);
                let mesh1 = model.to_triangular_mesh().unwrap();
                let mesh2 = model.to_triangular_mesh().unwrap();

                let hash1 = mesh1.geometry_hash().unwrap();
                let hash2 = mesh2.geometry_hash().unwrap();

                assert_eq!(hash1, hash2, "Hash not deterministic for {:?}", solid);
            }
        }
    }

    mod serde_tests {
        use super::*;

        #[test]
        fn test_platonic_solid_serialize_deserialize() {
            let solids = [
                PlatonicSolid::Tetrahedron,
                PlatonicSolid::Hexahedron,
                PlatonicSolid::Octahedron,
                PlatonicSolid::Dodecahedron,
                PlatonicSolid::Icosahedron,
            ];

            for solid in solids {
                let json = serde_json::to_string(&solid).unwrap();
                let deserialized: PlatonicSolid = serde_json::from_str(&json).unwrap();
                assert_eq!(solid, deserialized);
            }
        }

        #[test]
        fn test_material_default_serialize_deserialize() {
            let material = Material::Default;
            let json = serde_json::to_string(&material).unwrap();
            let deserialized: Material = serde_json::from_str(&json).unwrap();
            assert_eq!(material, deserialized);
        }

        #[test]
        fn test_material_uniform_serialize_deserialize() {
            let material = Material::Uniform("gold".to_string());
            let json = serde_json::to_string(&material).unwrap();
            let deserialized: Material = serde_json::from_str(&json).unwrap();
            assert_eq!(material, deserialized);
        }

        #[test]
        fn test_material_map_serialize_deserialize() {
            let mut face_map = HashMap::new();
            face_map.insert(0, "red".to_string());
            face_map.insert(1, "blue".to_string());
            face_map.insert(5, "green".to_string());

            let material = Material::Map {
                default: "gray".to_string(),
                face_map,
            };

            let json = serde_json::to_string(&material).unwrap();
            let deserialized: Material = serde_json::from_str(&json).unwrap();
            assert_eq!(material, deserialized);
        }

        #[test]
        fn test_material_array_serialize_deserialize() {
            let materials = vec![
                "mat0".to_string(),
                "mat1".to_string(),
                "mat2".to_string(),
                "mat3".to_string(),
            ];

            let material = Material::Array(materials);
            let json = serde_json::to_string(&material).unwrap();
            let deserialized: Material = serde_json::from_str(&json).unwrap();
            assert_eq!(material, deserialized);
        }

        #[test]
        fn test_model_with_default_material_serialize_deserialize() {
            let model = Model::with_default_material(PlatonicSolid::Tetrahedron);
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, deserialized);
        }

        #[test]
        fn test_model_with_uniform_material_serialize_deserialize() {
            let model = Model::with_uniform_material(PlatonicSolid::Hexahedron, "wood".to_string());
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, deserialized);
        }

        #[test]
        fn test_model_with_map_material_serialize_deserialize() {
            let mut face_map = HashMap::new();
            face_map.insert(0, "front".to_string());
            face_map.insert(1, "back".to_string());

            let material = Material::Map {
                default: "default".to_string(),
                face_map,
            };

            let model = Model::new(PlatonicSolid::Dodecahedron, material);
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, deserialized);
        }

        #[test]
        fn test_model_with_array_material_serialize_deserialize() {
            let materials = vec![
                "mat0".to_string(),
                "mat1".to_string(),
                "mat2".to_string(),
                "mat3".to_string(),
                "mat4".to_string(),
                "mat5".to_string(),
            ];

            let model = Model::new(PlatonicSolid::Hexahedron, Material::Array(materials));
            let json = serde_json::to_string(&model).unwrap();
            let deserialized: Model = serde_json::from_str(&json).unwrap();
            assert_eq!(model, deserialized);
        }

        #[test]
        fn test_all_platonic_solids_model_serialize_deserialize() {
            let solids = [
                PlatonicSolid::Tetrahedron,
                PlatonicSolid::Hexahedron,
                PlatonicSolid::Octahedron,
                PlatonicSolid::Dodecahedron,
                PlatonicSolid::Icosahedron,
            ];

            for solid in solids {
                let model = Model::with_default_material(solid);
                let json = serde_json::to_string(&model).unwrap();
                let deserialized: Model = serde_json::from_str(&json).unwrap();
                assert_eq!(model, deserialized, "Failed for {:?}", solid);
            }
        }

        #[test]
        fn test_model_json_structure() {
            let model = Model::with_uniform_material(PlatonicSolid::Icosahedron, "silver".to_string());
            let json = serde_json::to_string_pretty(&model).unwrap();

            // Verify the JSON contains expected fields
            assert!(json.contains("geometry"));
            assert!(json.contains("material"));
            assert!(json.contains("Icosahedron"));
            assert!(json.contains("silver"));
        }

        #[test]
        fn test_deserialize_from_json_string() {
            let json = r#"{"geometry":"Tetrahedron","material":"Default"}"#;
            let model: Model = serde_json::from_str(json).unwrap();
            assert_eq!(model.geometry, PlatonicSolid::Tetrahedron);
            assert_eq!(model.material, Material::Default);
        }

        #[test]
        fn test_deserialize_uniform_material_from_json_string() {
            let json = r#"{"geometry":"Hexahedron","material":{"Uniform":"bronze"}}"#;
            let model: Model = serde_json::from_str(json).unwrap();
            assert_eq!(model.geometry, PlatonicSolid::Hexahedron);
            assert_eq!(model.material, Material::Uniform("bronze".to_string()));
        }
    }
}

