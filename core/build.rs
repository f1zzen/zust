//! SHA-256 файлов движка из src-tauri/zapret/bin. Zust не запустит winws.exe/WinDivert,
//! если файл в рабочей папке отличается от того, что был в сборке.

fn main() {
    use sha2::{Digest, Sha256};
    let mut out = String::from("pub const ENGINE_FILES: &[(&str, &str)] = &[\n");
    for name in [
        "winws.exe",
        "WinDivert.dll",
        "WinDivert64.sys",
        "cygwin1.dll",
    ] {
        let path = format!("../src-tauri/zapret/bin/{name}");
        println!("cargo:rerun-if-changed={path}");
        if let Ok(data) = std::fs::read(&path) {
            let hash: String = Sha256::digest(&data)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            out.push_str(&format!("    ({name:?}, {hash:?}),\n"));
        }
    }
    out.push_str("];\n");
    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("engine_hashes.rs");
    std::fs::write(dest, out).unwrap();
}
