pub mod c_api;
pub mod geometry_error;
pub mod geometry_hash;
pub mod geometry_model;
pub mod material;
pub mod planar_contour;
pub mod planar_nurbs_curve;
pub mod planar_profile;
pub mod plane;
pub mod platonic_solid;
pub mod polygon_mesh;
pub mod sphere;
pub mod triangular_mesh;
pub mod vertex;
pub mod web_api;

pub fn hello() -> String {
    "Hello World!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = hello();
        assert_eq!(result, "Hello World!");
    }
}
