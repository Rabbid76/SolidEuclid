pub mod c_api;
pub mod geometry_error;
pub mod polygon_mesh;
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
