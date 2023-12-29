fn main() {
    if cfg!(target_os = "macos") {
        get_mac_bundle();
    }
}

fn get_mac_bundle() {
    let scratch_dir = scratch::path("cef_wrapper");

    println!("scratch_dir: {}", scratch_dir.display());
    // panic!()
}