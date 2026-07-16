fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=bz2");

    let mut build = cxx_build::bridge("src/lib.rs");
    build
        .file("src/shim.cpp")
        .include("src")
        .include("vendor/welle.io/src")
        .include("vendor/welle.io/src/backend")
        .include("vendor/welle.io/src/various")
        .include("vendor/welle.io/src/input")
        .std("gnu++17");

    // Homebrew's fftw formula installs fftw3.h/libfftw3f into the Homebrew
    // prefix's include/lib dirs, but those aren't on clang's default search
    // path (unlike Linux's apt-installed libfftw3-dev, which lands in a
    // standard system location). Ask brew directly rather than hardcoding
    // /opt/homebrew vs /usr/local, since that differs between Apple Silicon
    // and Intel.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        if let Ok(output) = std::process::Command::new("brew")
            .args(["--prefix", "fftw"])
            .output()
        {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                build.include(format!("{prefix}/include"));
                println!("cargo:rustc-link-search=native={prefix}/lib");
            }
        }
    }

    // fftw3 has no package for the default C:\mingw64 toolchain, so CI
    // installs it via MSYS2 instead and points us at that install's root.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if let Ok(root) = std::env::var("MSYS2_MINGW64_ROOT") {
            build.include(format!("{root}/include"));
            println!("cargo:rustc-link-search=native={root}/lib");
        }
    }

    build.compile("welle-io-sys-bridge");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/shim.cpp");
    println!("cargo:rerun-if-changed=src/shim.h");
}
