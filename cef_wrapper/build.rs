use std::path::Path;

fn main() {
    // set up cmake build
    println!("cargo:rerun-if-changed=cef/CMakeLists.txt");
    println!("cargo:rerun-if-changed=cef/src");
    println!("cargo:rerun-if-changed=cef/cmake");

    // build the c++ wrapper library (only used for the cef_load_library and cef_unload_library functions on macos)
    #[cfg(target_os = "macos")]
    {
        let cmake_target_dir = cmake::Config::new("./cef")
            .generator("Ninja")
            .build_target("libcef_dll_wrapper")
            .build()
            .join("build");
    
        let lib_dir = cmake_target_dir.join("lib");
    
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
    }

    // copy the sandbox library to lib_dir
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let _ = cmake::Config::new("./cef")
            .generator("Ninja")
            .build_target("copy_cef_sandbox")
            .build();
    
        println!("cargo:rustc-link-lib=sandbox");
        println!("cargo:rustc-link-lib=static=cef_sandbox");
    }

    let cmake_target_dir = cmake::Config::new("./cef")
        .generator("Ninja")
        .build_target("copy_cef_include")
        .build()
        .join("build");

    let include_dir = cmake_target_dir.join("include");
    let clang_include_arg = format!("-I{}", include_dir.display());

    // set up bindgen for cmake library
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(clang_include_arg)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("bindings_c.rs");
    bindings
        .write_to_file(out_path)
        .expect("Unable to write bindings");
}
