fn main() {
    #[cfg(windows)]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let obj_path = format!("{}/../../assets/icon_x64.o", manifest_dir).replace('\\', "/");
        if std::path::Path::new(&obj_path).exists() {
            println!("cargo:rustc-link-arg={}", obj_path);
            println!("cargo:rerun-if-changed={}", obj_path);
        }
    }
}

