fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=bz2");

    cxx_build::bridge("src/lib.rs")
        .file("src/shim.cpp")
        .include("src")
        .include("vendor/welle.io/src")
        .include("vendor/welle.io/src/backend")
        .include("vendor/welle.io/src/various")
        .include("vendor/welle.io/src/input")
        .std("c++17")
        .compile("welle-io-sys-bridge");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/shim.cpp");
    println!("cargo:rerun-if-changed=src/shim.h");
}
