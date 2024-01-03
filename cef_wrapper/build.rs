use std::path::Path;

use cmake;

fn main() {
    // set up bindgen for cmake library
    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings
        .write_to_file(out_path)
        .expect("Unable to write bindings");

    // set up cmake build
    println!("cargo:rerun-if-changed=cef/CMakeLists.txt");
    println!("cargo:rerun-if-changed=cef/src/CmakeLists.txt");
    println!("cargo:rerun-if-changed=cef/cmake");

    let cmake_target_dir = cmake::Config::new("./cef")
        .generator("Ninja")
        .build_target("cef_wrapper")
        .build()
        .join("build");

    let lib_dir = cmake_target_dir.join("lib");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=cef_wrapper");
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    println!("cargo:rustc-link-lib=static=cef_sandbox");
}
