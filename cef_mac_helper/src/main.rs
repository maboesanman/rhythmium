#[cfg(target_os = "macos")]
mod main_mac;

fn main() -> Result<(), i32> {
    #[cfg(target_os = "macos")]
    return main_mac::main();

    #[cfg(not(target_os = "macos"))]
    panic!("This program can only be built for macOS.");
}
