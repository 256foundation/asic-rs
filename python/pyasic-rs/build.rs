fn main() {
    println!("cargo:rerun-if-env-changed=PYO3_BUILD_EXTENSION_MODULE");

    if std::env::var_os("PYO3_BUILD_EXTENSION_MODULE").is_some() {
        pyo3_build_config::add_extension_module_link_args();
    }
}
