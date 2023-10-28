use std::env;
use std::path::PathBuf;
use glob::glob;

fn main() {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    env::vars().for_each(|(key, value)| {
        println!("cargo:rerun-if-env-changed={}", key);
        println!("{}={}", key, value);
    });

    let vendor_cef_folder = PathBuf::from(cargo_dir)
        .parent()
        .unwrap()
        .join("vendor/cef");

    let cef_path = match env::var("CEF_DISTRIBUTION") {
        // CMake specifies a distribution
        Ok(cef_distribution) => vendor_cef_folder
            .join(cef_distribution)
            .canonicalize()
            .expect("cannot canonicalize path"),
        
        // we want to appease rust analyzer by making this find something at least
        Err(_) => {
            let vendor_path_str = vendor_cef_folder.to_str().unwrap();
            let mut paths = glob(&format!("{vendor_path_str}/cef_binary_*")).unwrap();
    
            let out = match paths.next() {
                Some(out) => out.unwrap(),
                None => panic!("No cef binary found in {}", vendor_path_str),
            };
    
            out.canonicalize().expect("Failed to canonicalize cef path")
        }
    };

    let cef_path_str = cef_path.to_str().unwrap();
    let cef_path_arg = format!("--include-directory={cef_path_str}");

    let lib_path = cef_path.join("Release");
    let lib_path_str = lib_path.to_str().unwrap();

    println!("cargo:rustc-link-search={lib_path_str}/");
    println!("cargo:rustc-link-lib=static:+verbatim=cef_sandbox.a");

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=framework={lib_path_str}");
        println!("cargo:rustc-link-lib=framework=Chromium Embedded Framework");
    }

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .clang_arg(cef_path_arg)
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
