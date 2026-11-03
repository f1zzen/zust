use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;

/// MorphShape: контур r(θ) = 1 + amp·cos(lobes·θ) - круг при amp = 0, "печенька" M3E при amp > 0.
fn morph_shape() -> String {
    const N: usize = 120;
    let mut s = String::from(
        "export component MorphShape inherits Path {\n\
         \x20   in property <float> amp: 0;\n\
         \x20   in property <float> lobes: 9;\n\
         \x20   in property <angle> rotation: 0deg;\n\
         \x20   in property <float> sx: 1;\n\
         \x20   viewbox-width: 100 * sx;\n\
         \x20   viewbox-height: 100;\n",
    );
    for i in 0..N {
        let theta = i as f64 * 360.0 / N as f64;
        let el = if i == 0 { "MoveTo" } else { "LineTo" };
        // радиус нормирован, чтобы фигура не выходила за 100×100
        let r = format!("(50 / (1 + root.amp)) * (1 + root.amp * cos(root.lobes * {theta}deg))");
        writeln!(
            s,
            "    {el} {{ x: (50 + {r} * cos({theta}deg + root.rotation)) * root.sx; y: 50 + {r} * sin({theta}deg + root.rotation); }}"
        )
        .unwrap();
    }
    s.push_str("    Close { }\n}\n");
    s
}

/// Ресурс Windows с иконкой exe - собирается из icon.ico без rc.exe (lld-link принимает .res).
fn windows_icon_res(ico: &[u8]) -> Vec<u8> {
    let u16le = |b: &[u8], o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32le = |b: &[u8], o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    fn entry(out: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(32u32.to_le_bytes());
        out.extend([0xff, 0xff]);
        out.extend(kind.to_le_bytes());
        out.extend([0xff, 0xff]);
        out.extend(name.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(flags.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    let mut res = Vec::new();
    entry(&mut res, 0, 0, 0, &[]);
    let count = u16le(ico, 4);
    let mut group = vec![0, 0, 1, 0];
    group.extend(count.to_le_bytes());
    for i in 0..count as usize {
        let e = 6 + i * 16;
        let size = u32le(ico, e + 8) as usize;
        let offset = u32le(ico, e + 12) as usize;
        entry(
            &mut res,
            3,
            i as u16 + 1,
            0x1010,
            &ico[offset..offset + size],
        );
        group.extend(&ico[e..e + 12]);
        group.extend((i as u16 + 1).to_le_bytes());
    }
    entry(&mut res, 14, 1, 0x1030, &group);
    res
}

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("morph.slint"), morph_shape()).unwrap();

    let ico_path = "../src-tauri/icons/icon.ico";
    println!("cargo:rerun-if-changed={ico_path}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let res = out.join("icon.res");
        std::fs::write(&res, windows_icon_res(&std::fs::read(ico_path).unwrap())).unwrap();
        println!("cargo:rustc-link-arg-bins={}", res.display());
    }

    // material - стиль служебных виджетов (прокрутка), остальное - свои компоненты
    let config = slint_build::CompilerConfiguration::new()
        .with_style("material".into())
        .with_library_paths(HashMap::from([("gen".to_string(), out)]));
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
