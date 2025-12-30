pub mod web_api;

pub fn hello() -> String {
    "Hello WASM!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = hello();
        assert_eq!(result, "Hello WASM!");
    }
}
