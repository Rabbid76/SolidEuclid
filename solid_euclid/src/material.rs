use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

impl Material {
    /// Gets the material name for a face index based on the material assignment.
    pub fn get_material_for_face(&self, face_index: i32, _total_faces: i32) -> String {
        match self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

        if let Material::Map { default, face_map } = material {
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
    fn test_material_clone() {
        let material = Material::Uniform("wood".to_string());
        let cloned = material.clone();
        assert_eq!(material, cloned);
    }

    #[test]
    fn test_material_debug() {
        let material = Material::Uniform("test".to_string());
        let debug_str = format!("{:?}", material);
        assert!(debug_str.contains("Uniform"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_get_material_for_face_default() {
        let material = Material::Default;
        assert_eq!(material.get_material_for_face(0, 4), "material_0");
        assert_eq!(material.get_material_for_face(3, 4), "material_3");
    }

    #[test]
    fn test_get_material_for_face_uniform() {
        let material = Material::Uniform("gold".to_string());
        assert_eq!(material.get_material_for_face(0, 4), "gold");
        assert_eq!(material.get_material_for_face(3, 4), "gold");
    }

    #[test]
    fn test_get_material_for_face_map() {
        let mut face_map = HashMap::new();
        face_map.insert(0, "special".to_string());

        let material = Material::Map {
            default: "default".to_string(),
            face_map,
        };

        assert_eq!(material.get_material_for_face(0, 4), "special");
        assert_eq!(material.get_material_for_face(1, 4), "default");
    }

    #[test]
    fn test_get_material_for_face_array() {
        let material = Material::Array(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(material.get_material_for_face(0, 4), "a");
        assert_eq!(material.get_material_for_face(1, 4), "b");
        assert_eq!(material.get_material_for_face(2, 4), "a"); // cyclic
        assert_eq!(material.get_material_for_face(3, 4), "b"); // cyclic
    }

    #[test]
    fn test_get_material_for_face_empty_array() {
        let material = Material::Array(vec![]);
        assert_eq!(material.get_material_for_face(0, 4), "material_0");
    }
}
