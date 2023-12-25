use cmake;

fn main() {
    let dst = cmake::Config::new("./cef")
        .build_target("minimal")
        .build();

    let cmake_cache_path = dst.join("build/CMakeCache.txt");
    let cmake_cache = std::fs::read_to_string(cmake_cache_path).unwrap();

    let build_type = &regex::Regex::new(r"CMAKE_BUILD_TYPE:STRING=([a-zA-Z]+)")
        .unwrap()
        .captures(&cmake_cache)
        .unwrap()[1];

    let cmake_build_dir = dst.join("build").join(build_type);

    println!(
        "cargo:rustc-link-search=native={}",
        cmake_build_dir.display()
    );
    println!("cargo:rustc-link-lib=static=minimal");
}
