fn main() {
    println!("cargo:rustc-link-lib=gphoto2");
    println!("cargo:rustc-link-lib=gphoto2_port");
    println!("cargo:rerun-if-changed=build.rs");
}
