use std::path::Path;

use cmake;

fn main() {
    let dst = cmake::Config::new("./cef")
        .generator("Ninja")
        .build_target("minimal")
        .build();

    let cmake_cache_path = dst.join("build/CMakeCache.txt");
    let cmake_cache = std::fs::read_to_string(cmake_cache_path).unwrap();

    let cmake_build_type = &regex::Regex::new(r"CMAKE_BUILD_TYPE:STRING=([a-zA-Z]+)")
        .unwrap()
        .captures(&cmake_cache)
        .unwrap()[1];

    let cef_build_type = match cmake_build_type {
        "Release" => "Release",
        _ => "Debug",
    };

    let cmake_build_dir = dst.join("build");
    let cmake_build_dir_wrapper = cmake_build_dir.join("libcef_dll_wrapper");
    let cmake_build_dir_type = cmake_build_dir.join(cmake_build_type);

    link_wrapper(&cmake_build_dir_wrapper);
    link_binaries(&cmake_build_dir_type, cef_build_type);

    if cfg!(target_os = "macos") {
        copy_mac_framework(&cmake_build_dir_type);
    }
}

fn link_wrapper(wrapper_dir: &Path) {
    println!(
        "cargo:rustc-link-search=native={}",
        wrapper_dir.display()
    );
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
}

fn link_binaries(binary_dir: &Path, cef_build_type: &str) {
    println!(
        "cargo:rustc-link-search=native={}",
        binary_dir.display()
    );
    println!("cargo:rustc-link-lib=static=minimal");
    println!("cargo:rustc-link-lib=static=shared");
    println!("cargo:rustc-link-lib=static=shared_helper");

    let cef_sandbox_path = std::fs::read_dir("./cef/third_party/cef/").unwrap()
        .filter(|dir| {
            // only directories
            dir.as_ref().unwrap().file_type().unwrap().is_dir()
        })
        .max_by_key(|dir| {
        dir.as_ref().unwrap().file_name().to_owned()
    }).unwrap().unwrap().path().canonicalize().unwrap();

    let cef_sandbox_path = cef_sandbox_path.join(cef_build_type).canonicalize().unwrap();

    println!("cargo:rustc-link-search=native={}", cef_sandbox_path.display());
    println!("cargo:rustc-link-lib=static:+verbatim=cef_sandbox.a");
}

fn copy_mac_framework(binary_dir: &Path) {
    let scratch_dir = scratch::path("cef_wrapper");
    let bundle = binary_dir.join("minimal.app");

    fs_extra::dir::copy(
        bundle,
        scratch_dir,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    ).unwrap();
}