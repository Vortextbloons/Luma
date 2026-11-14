fn main() {
    tauri_build::build();
    if cfg!(windows) {
        println!(
            "cargo:rustc-link-arg-examples={}/resource.lib",
            std::env::var("OUT_DIR").unwrap()
        );
    }
}
