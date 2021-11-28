#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bindings {
	include!("bindings.rs");
}

pub use self::bindings::*;
