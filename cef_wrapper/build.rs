use std::path::Path;

fn main() {
    // set up cmake build
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=../cmake/CEFVersion.cmake");

    let cef_dir = get_cef_dir();

    println!(
        "cargo:rerun-if-changed={}",
        cef_dir.join("include/cef_api_versions.h").display()
    );

    let api_version = get_cef_api_version();
    let api_hash = get_cef_api_hash(&cef_dir, api_version);

    let consts_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("consts.rs");
    std::fs::write(
        &consts_path,
        format!(
            "pub const CEF_API_VERSION_VALUE: i32 = {};\npub const CEF_API_HASH_PLATFORM: &str = \"{}\";\n",
            api_version, api_hash
        ),
    )
    .expect("Unable to write consts.rs");

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
        .clang_arg(format!("-DCEF_API_VERSION={}", get_cef_api_version()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("bindings_c.rs");
    bindings
        .write_to_file(out_path)
        .expect("Unable to write bindings");
}

fn read_cef_version_file() -> (std::path::PathBuf, String) {
    let current_dir = std::env::current_dir().unwrap();
    let path = current_dir.join("../cmake/CEFVersion.cmake");
    let contents = std::fs::read_to_string(&path).unwrap();
    (current_dir, contents)
}

fn get_cef_api_version() -> u32 {
    let (_, version_file) = read_cef_version_file();
    let re = regex::Regex::new(r#"set\(CEF_API_VERSION_VALUE (\d+)\)"#).unwrap();
    re.captures(&version_file)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str()
        .parse()
        .unwrap()
}

fn get_cef_api_hash(cef_dir: &std::path::Path, version: u32) -> String {
    // Write a stub that expands to just the platform hash string literal.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let stub_path = Path::new(&out_dir).join("cef_hash_probe.h");
    std::fs::write(&stub_path, "#include \"include/cef_api_hash.h\"\nCEF_API_HASH_PLATFORM\n")
        .expect("Unable to write cef_hash_probe.h");

    // Run the C preprocessor on the stub so the macros get expanded.
    let compiler = cc::Build::new()
        .flag(&format!("-DCEF_API_VERSION={}", version))
        .include(cef_dir)
        .get_compiler();

    let output = compiler
        .to_command()
        .args(["-E", "-P"])
        .arg(&stub_path)
        .output()
        .expect("Failed to run C preprocessor");

    assert!(
        output.status.success(),
        "C preprocessor failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The preprocessed output is a quoted string literal like `"abcdef..."`.
    let preprocessed = String::from_utf8(output.stdout).expect("Non-UTF8 preprocessor output");
    let hash_re = regex::Regex::new(r#""([0-9a-f]{40})""#).unwrap();
    hash_re
        .captures(&preprocessed)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .expect("Could not find CEF_API_HASH_PLATFORM in preprocessor output")
}

fn get_cef_dir() -> std::path::PathBuf {
    // read version from CEFVersion.cmake
    let (current_dir, version_file) = read_cef_version_file();

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
