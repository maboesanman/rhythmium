use std::env;
use std::path::PathBuf;

fn main() {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cef_version = "cef_binary_115.3.11+ga61da9b+chromium-115.0.5790.114_macosarm64_minimal";
    // // Tell cargo to look for shared libraries in the specified directory

    // This is the directory where the `c` library is located.
    let libdir_path = PathBuf::from(format!("{}/{}", cargo_dir, cef_version))
        // Canonicalize the path as `rustc-link-search` requires an absolute
        // path.
        .canonicalize()
        .expect("cannot canonicalize path");
    let libdir_path_str = libdir_path.to_str().unwrap();
    let libdir_path_arg = format!("--include-directory={libdir_path_str}");

    let lib_path = libdir_path.join("Release");
    let lib_path_str = lib_path.to_str().unwrap();

    println!("cargo:rustc-link-search=static={lib_path_str}");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .clang_arg(libdir_path_arg)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
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
