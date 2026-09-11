fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/error.rs");
    println!("cargo:rerun-if-changed=src/factory.rs");
    println!("cargo:rerun-if-changed=src/miner.rs");
    println!("cargo:rerun-if-changed=src/runtime.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let Ok(crate_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        println!("cargo:warning=CARGO_MANIFEST_DIR is unset; skipping cbindgen");
        return;
    };

    let config_path = std::path::Path::new(&crate_dir).join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path).unwrap_or_default();

    let out_dir = std::path::PathBuf::from(&crate_dir).join("include");
    if std::fs::create_dir_all(&out_dir).is_err() {
        println!("cargo:warning=failed to create asic-rs-ffi/include");
        return;
    }
    let header = out_dir.join("asic_rs_ffi.h");

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(header);
        }
        Err(error) => {
            println!("cargo:warning=cbindgen failed: {error}");
        }
    }
}
