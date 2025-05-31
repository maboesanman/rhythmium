use std::path::Path;

fn main() {
    // set up cmake build
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=wrapper.h");

    let cef_dir = get_cef_dir();

    let profile = std::env::var("PROFILE").unwrap();
    let cef_target_dir = if profile == "debug" {
        cef_dir.join("Debug")
    } else {
        cef_dir.join("Release")
    };

    println!(
        "cargo:rustc-link-search=native={}",
        cef_target_dir.display()
    );

    // build the c++ wrapper library (only used for the cef_load_library and cef_unload_library functions on macos)
    #[cfg(target_os = "macos")]
    {
        // macos uses the cef_load_library and cef_unload_library functions to load the CEF framework,
        // so we need to build the c++ wrapper library to provide these functions
        let cmake_target_dir = cmake::Config::new(".")
            .generator("Ninja")
            .build_target("libcef_dll_wrapper")
            .build()
            .join("build");

        let lib_dir = cmake_target_dir.join("lib");

        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
        println!("cargo:rustc-link-lib+verbatim=static+verbatim=cef_sandbox.a");
    }

    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dylib=cef");
    }

    // set up bindgen for capi
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", cef_dir.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("bindings_c.rs");
    bindings
        .write_to_file(out_path)
        .expect("Unable to write bindings");
}

fn get_cef_dir() -> std::path::PathBuf {
    // read version from CEFVersion.cmake
    let current_dir = std::env::current_dir().unwrap();
    let version_file = current_dir.join("../cmake/CEFVersion.cmake");
    let version_file = std::fs::read_to_string(version_file).unwrap();

    // regex to extract version
    let re = regex::Regex::new(r#"set\(CEF_VERSION "([a-z0-9-\.\+]+)"\)"#).unwrap();
    let version = re.captures(&version_file).unwrap().get(1).unwrap().as_str();

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let platform = "macosarm64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let platform = "macosx64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let platform = "windows64";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let platform = "linux64";

    current_dir.join(format!(
        "../third_party/cef/cef_binary_{version}_{platform}"
    ))
}
