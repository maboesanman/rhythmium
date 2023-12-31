use std::path::Path;

use cmake;

fn main() {
    let cmake_target_dir = cmake::Config::new("./cef")
        .generator("Ninja")
        .build_target("cef")
        .build()
        .join("build");

    let lib_dir = cmake_target_dir.join("lib");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=cef");
    println!("cargo:rustc-link-lib=static=cef_sandbox");
    println!("cargo:rustc-link-lib=static=cef_dll_wrapper");

    if cfg!(target_os = "macos") {
        copy_mac_framework(&cmake_target_dir);
    }
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
