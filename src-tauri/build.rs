fn main() {
    let timestamp = std::env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    });
    println!("cargo:rustc-env=LENZ_BUILD_DATE={timestamp}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-changed=native/AppleTranslate.swift");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let arch = if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64") {
            "arm64"
        } else {
            "x86_64"
        };
        let status = std::process::Command::new("xcrun")
            .args(["swiftc", "-parse-as-library", "-O", "-target"])
            .arg(format!("{arch}-apple-macosx12.0"))
            .arg("-module-cache-path")
            .arg(out.join("swift-cache"))
            .arg("native/AppleTranslate.swift")
            .arg("-o")
            .arg(out.join("lenz-apple-translate"))
            .status()
            .expect("Swift compiler is required for Apple translation");
        assert!(
            status.success(),
            "could not compile Apple translation bridge"
        );
    }
    tauri_build::build()
}
