use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hello_wasm() -> String {
    crate::hello()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_wasm() {
        let result = hello_wasm();
        assert_eq!(result, "Hello WASM!");
    }
}
