// use std::env;
// use std::path::PathBuf;

fn main() {
    // let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // let cef_version = "../cef-sys/cef_binary_115.3.11+ga61da9b+chromium-115.0.5790.114_macosarm64_minimal";
    // // // Tell cargo to look for shared libraries in the specified directory

    // // This is the directory where the `c` library is located.
    // let libdir_path = PathBuf::from(format!("{}/{}", cargo_dir, cef_version))
    //     // Canonicalize the path as `rustc-link-search` requires an absolute
    //     // path.
    //     .canonicalize()
    //     .expect("cannot canonicalize path");

    
    // let lib_path = libdir_path.join("Release");
    // let lib_path_str = lib_path.to_str().unwrap();
    
    // // println!("{lib_path_str}");
    // // panic!();

    // println!("cargo:rustc-link-search=all={lib_path_str}");
    // println!("cargo:rustc-link-lib=static:+verbatim=cef_sandbox.a");
    // println!("cargo:rustc-link-lib=framework=Chromium Embedded Framework");
}
