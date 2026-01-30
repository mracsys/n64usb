use std::env;
use std::path::PathBuf;
use std::path::Path;
use std::fs;

use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};

#[derive(Debug)]
struct StripEnumPrefix;

impl ParseCallbacks for StripEnumPrefix {
    fn enum_variant_name(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _value: EnumVariantValue,
    ) -> Option<String> {
        let enum_name = _enum_name?;
        let stripped = match enum_name {
            "DeviceError" => original_variant_name.strip_prefix("DEVICEERR_"),
            "CartType" => original_variant_name.strip_prefix("CART_"),
            "SaveType" => original_variant_name.strip_prefix("SAVE_"),
            "USBDataType" => original_variant_name.strip_prefix("DATATYPE_"),
            "ProtocolVer" => original_variant_name.strip_prefix("PROTOCOL_"),
            _ => None,
        }?;
        let escaped = if stripped.chars().next().unwrap().is_ascii_digit() {
            format!("_{}", stripped)
        } else {
            stripped.to_string()
        };
        Some(escaped)
    }
}


fn main() {
	#[cfg(target_os = "windows")] {
        println!("cargo:rustc-link-lib=shlwapi");
    }
    // Tell cargo to look for shared libraries in the specified directory
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search={}", Path::new(&dir).join("lib").display());

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=dylib=Flashcart_x64");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("lib/device.hpp")
        .allowlist_function("device_.*")
        .parse_callbacks(Box::new(StripEnumPrefix))
        .rustified_enum(".*")
        .raw_line("#[allow(non_upper_case_globals)]")
        .raw_line("#[allow(non_camel_case_types)]")
        .raw_line("#[allow(non_snake_case)]")
        .raw_line("#[allow(dead_code)]")
        .raw_line("#[allow(unnecessary_transmutes)]")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Locate the source and destination
    let lib_name = "Flashcart_x64.dll";
    let source = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("lib").join(lib_name);
    
    // We want to place it in target/debug or target/release
    // Path looks like: target/debug/deps/../libflashcart.so
    let dest_dir = out_path.join("../../../").canonicalize().unwrap();
    let destination = dest_dir.join(lib_name);

    if source.exists() {
        fs::copy(&source, &destination).expect("Could not copy shared library to target directory");
    }

    // Locate the source and destination
    let lib_name = "Flashcart_x64.pdb";
    let source = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("lib").join(lib_name);
    
    // We want to place it in target/debug or target/release
    // Path looks like: target/debug/deps/../libflashcart.so
    let dest_dir = out_path.join("../../../").canonicalize().unwrap();
    let destination = dest_dir.join(lib_name);

    if source.exists() {
        fs::copy(&source, &destination).expect("Could not copy shared library to target directory");
    }
}
