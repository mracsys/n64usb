use std::env;
use std::path::PathBuf;

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
            "HintDataType" => original_variant_name.strip_prefix("HINTTYPE_"),
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
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let flashcart_sources = [
        "lib/src/device.cpp",
        "lib/src/device_usb.cpp",
        "lib/src/device_64drive.cpp",
        "lib/src/device_everdrive.cpp",
        "lib/src/device_sc64.cpp",
        "lib/src/device_gopher64.cpp",
        "lib/src/device_wii.cpp",
    ];

    let mut build = cc::Build::new();
    build.cpp(true);

	if target_os == "windows" {
        println!("cargo:rustc-link-lib=shlwapi");

        build
            .define("_CRT_SECURE_NO_WARNINGS", None)
            .define("D2XX", None)
            .define("_LIB", None)
            .define("NDEBUG", None)
            .include("lib/Include")
            .include("lib")
            .static_crt(true);

        match target_arch.as_str() {
            "x86" => {
                build.define("WIN32", None);
                println!("cargo:rustc-link-lib=static=ftd2xx");
            },
            "x86_64" => {
                println!("cargo:rustc-link-lib=static=ftd2xx_x64");
            }
            _ => {},
        };
        let include_path = std::fs::canonicalize("lib/Include").unwrap();
        println!("cargo:rustc-link-search=native={}", include_path.display());
    } else {
        build
            .cpp_set_stdlib(None)
            .flag("-std=c++11")
            .define("LINUX", None)
            .define("_XOPEN_SOURCE_EXTENDED", None)
            .flag("-Wall")
            .flag("-Wno-unknown-pragmas");

        let prefix = env::var("PREFIX").unwrap_or_else(|_| "/usr/local".to_string());
        build.include(format!("{prefix}/include"));

        if target_os == "macos" {
            let brew = String::from_utf8(
                std::process::Command::new("brew")
                .args(["--prefix"])
                .output()
                .unwrap()
                .stdout
            ).unwrap().trim().to_string();
            let brew_ncurses = String::from_utf8(
                std::process::Command::new("brew")
                .args(["--prefix", "ncurses"])
                .output()
                .unwrap()
                .stdout
            ).unwrap().trim().to_string();
            let brew_c = String::from_utf8(
                std::process::Command::new("brew")
                .args(["--prefix", "llvm"])
                .output()
                .unwrap()
                .stdout
            ).unwrap().trim().to_string();
            build.include(format!("{brew}/include"));
            println!("cargo:rustc-link-search=native={brew}/lib");
            println!("cargo:rustc-link-search=native={brew_ncurses}/lib");
            println!("cargo:rustc-link-search=native={brew_c}/lib");
            println!("cargo:rustc-link-lib=framework=IOKit");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
            println!("cargo:rustc-link-lib=framework=Security");
            println!("cargo:rustc-link-lib=static=ncurses");
            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=static=ftdi1");
            println!("cargo:rustc-link-lib=static=usb-1.0");
        } else {
			// Requires the distro to provide static versions of
			// required libraries. On Debian:
			// sudo apt install llvm libclang-dev libftdi1-dev libudev-dev
            // Additional packages for Mido's House GUI (not needed for this crate):
            // sudo apt install pkg-config libglib2.0-dev
			let lib_arch = match target_arch.as_str() {
				"x86_64"  => "x86_64-linux-gnu",
				"x86"     => "i386-linux-gnu",
				"aarch64" => "aarch64-linux-gnu",
				"arm"     => "arm-linux-gnueabihf",
				_         => "x86_64-linux-gnu",
			};
			println!("cargo:rustc-link-search=native=/usr/lib/{lib_arch}");
            println!("cargo:rustc-link-lib=udev");
            println!("cargo:rustc-link-lib=rt");
            println!("cargo:rustc-link-lib=stdc++");
            println!("cargo:rustc-link-lib=static=ftdi1");
            println!("cargo:rustc-link-lib=static=usb-1.0");
        }
        println!("cargo:rustc-link-lib=pthread");
    }

    for src in &flashcart_sources {
        build.file(src);
    }
    build.compile("flashcart");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("lib/src/device.hpp")
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

}
