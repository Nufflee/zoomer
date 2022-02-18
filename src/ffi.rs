macro_rules! c_str_ptr {
    ($str:expr) => {
        crate::ffi::c_str!($str).as_ptr()
    };
}

macro_rules! c_str {
    ($str:expr) => {
        &std::ffi::CStr::from_bytes_with_nul(concat!($str, '\0').as_bytes()).unwrap()
    };
}

pub(crate) use c_str;
pub(crate) use c_str_ptr;
