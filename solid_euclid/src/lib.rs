use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hello_wasm() -> String {
    "Hello WASM!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = hello_wasm();
        assert_eq!(result, "Hello WASM!");
    }
}
