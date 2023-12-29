use std::path::Path;

use cmake;

fn main() {
    let cmake_target_dir = cmake::Config::new("./cef")
        .generator("Ninja")
        .build_target("rhythmium")
        .build()
        .join("build");

    link_wrapper(&cmake_target_dir);
    link_binaries(&cmake_target_dir);
    link_sandbox(&cmake_target_dir);

    if cfg!(target_os = "macos") {
        copy_mac_framework(&cmake_target_dir);
    }
}

fn get_cef_build_type(binary_dir: &Path) -> &'static str {
    let cmake_cache_path = binary_dir.join("CMakeCache.txt");
    let cmake_cache = std::fs::read_to_string(cmake_cache_path).unwrap();

    let cmake_build_type = &regex::Regex::new(r"CMAKE_BUILD_TYPE:STRING=([a-zA-Z]+)")
        .unwrap()
        .captures(&cmake_cache)
        .unwrap()[1];

    match cmake_build_type {
        "Release" => "Release",
        _ => "Debug",
    }
}

fn link_wrapper(binary_dir: &Path) {
    let wrapper_dir = binary_dir.join("libcef_dll_wrapper");
    println!(
        "cargo:rustc-link-search=native={}",
        wrapper_dir.display()
    );
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");
}

fn link_binaries(binary_dir: &Path) {
    let target_out = binary_dir.join("target_out");
    println!(
        "cargo:rustc-link-search=native={}",
        target_out.display()
    );
    println!("cargo:rustc-link-lib=static=rhythmium");
    println!("cargo:rustc-link-lib=static=shared");
    println!("cargo:rustc-link-lib=static=shared_helper");
}

fn link_sandbox(binary_dir: &Path) {
    let cef_sandbox_path = std::fs::read_dir("./cef/third_party/cef/").unwrap()
        .filter(|dir| {
            // only directories
            dir.as_ref().unwrap().file_type().unwrap().is_dir()
        })
        .max_by_key(|dir| {
        dir.as_ref().unwrap().file_name().to_owned()
    }).unwrap().unwrap().path().canonicalize().unwrap();

    let cef_sandbox_path = cef_sandbox_path.join(get_cef_build_type(binary_dir)).canonicalize().unwrap();

    println!("cargo:rustc-link-search=native={}", cef_sandbox_path.display());
    println!("cargo:rustc-link-lib=static:+verbatim=cef_sandbox.a");
}

fn copy_mac_framework(binary_dir: &Path) {
    let scratch_dir = scratch::path("cef_wrapper");
    let bundle = binary_dir.join("target_out/rhythmium.app");

    fs_extra::dir::copy(
        bundle,
        scratch_dir,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            ..Default::default()
        },
    ).unwrap();
}