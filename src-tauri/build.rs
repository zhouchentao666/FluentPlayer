fn main() {
    tauri_build::build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let info_plist = format!("{}/Info.plist", manifest_dir);
        println!("cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}", info_plist);
    }

    // oboe-sys and libsqlite3-sys compile C++ code; link the C++ standard library
    if target_os == "android" {
        println!("cargo:rustc-link-lib=c++");
    }
}
