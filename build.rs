#[cfg(feature = "bindgen")]
use bindgen::{builder, CargoCallbacks, EnumVariation};

#[cfg(feature = "bindgen")]
const REG: &'static str = "yaml_(event|parser|char)_.*";

fn main() {
	println!("cargo:rustc-link-lib=yaml");

	#[cfg(feature = "bindgen")]
	builder()
		.header_contents("wrapper.h", "#include <yaml.h>")
		.rustfmt_bindings(true)
		.layout_tests(false)
		.allowlist_function(REG)
		.allowlist_type(REG)
		.allowlist_var(REG)
		.default_enum_style(EnumVariation::ModuleConsts)
		.parse_callbacks(Box::new(CargoCallbacks))
		.generate()
		.expect("Compile Failed")
		.write_to_file("./src/bindings.rs")
		.expect("Compile Failed");
}
