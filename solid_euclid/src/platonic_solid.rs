use std::collections::HashMap;

/// Represents the five Platonic solids - convex regular polyhedra with equivalent faces
/// composed of congruent convex regular polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
}
