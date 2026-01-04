use std::ffi::CString;
use std::os::raw::c_char;

/// Returns a C-compatible string with the hello message.
/// The caller is responsible for freeing the returned string using `hello_c_free`.
#[unsafe(no_mangle)]
pub extern "C" fn hello_c() -> *mut c_char {
    let result = crate::hello();
    match CString::new(result) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a string allocated by `hello_c`.
/// # Safety
/// The pointer must be a valid pointer returned by `hello_c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hello_c_free(s: *mut c_char) { unsafe {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_hello_c() {
        let result_ptr = hello_c();
        assert!(!result_ptr.is_null());

        let result = unsafe { CStr::from_ptr(result_ptr) };
        assert_eq!(result.to_str().unwrap(), "Hello World!");

        unsafe { hello_c_free(result_ptr) };
    }
}
