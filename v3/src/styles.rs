//! Свой стиль: цвета, скругление и шрифт поверх выбранной темы.
//! Настраивается в приложении; файлами .json в папке themes стилем можно поделиться.

use crate::notify::Kind;
use crate::{AppWindow, Backend, Theme};
use serde::{Deserialize, Serialize};
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const BASES: [&str; 5] = ["m3e", "warm", "mono", "classic", "lite"];

/// Свой стиль в настройках интерфейса (ui-v3.json).
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Custom {
    pub enabled: bool,
    /// имя цвета темы -> 0xAARRGGBB
    pub colors: BTreeMap<String, u32>,
    /// 0 - как в теме
    pub radius: f32,
    /// "" - как в теме
    pub font: String,
    /// свой шрифт: файл в папке fonts
    pub font_file: String,
    /// картинки фона: файлы в папке backgrounds
    pub bg_main: String,
    pub bg_panel: String,
    /// затемнение картинок, 0..1; None - по умолчанию
    pub bg_dim: Option<f32>,
}

thread_local! {
    /// Файлы шрифта и картинок: их нет в Theme, а сохранять в настройки нужно.
    static FILES: std::cell::RefCell<(String, String, String)> = Default::default();
}

/// Файл стиля: то же самое плюс тема, светлая/тёмная и акцент.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct StyleFile {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dark: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accent: Option<String>,
    colors: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    radius: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String>,
}

/// Цвета темы, которые можно задать: имя в файле и поля Theme (задан ли, значение).
macro_rules! colors {
    ($($k:literal => $sh:ident $sc:ident $gh:ident $gc:ident),* $(,)?) => {
        fn set_color(t: &Theme, key: &str, v: Option<Color>) {
            match key {
                $($k => {
                    if let Some(v) = v {
                        t.$sc(v);
                    }
                    t.$sh(v.is_some());
                })*
                _ => {}
            }
        }

        fn custom_colors(t: &Theme) -> BTreeMap<String, u32> {
            let mut m = BTreeMap::new();
            $(if t.$gh() {
                m.insert($k.to_string(), t.$gc().as_argb_encoded());
            })*
            m
        }

        const KEYS: &[&str] = &[$($k),*];
    };
}

colors!(
    "background" => set_h_background set_c_background get_h_background get_c_background,
    "surface" => set_h_surface set_c_surface get_h_surface get_c_surface,
    "surface-high" => set_h_surface_high set_c_surface_high get_h_surface_high get_c_surface_high,
    "on-surface" => set_h_on_surface set_c_on_surface get_h_on_surface get_c_on_surface,
    "on-surface-variant" => set_h_on_surface_variant set_c_on_surface_variant get_h_on_surface_variant get_c_on_surface_variant,
    "outline" => set_h_outline set_c_outline get_h_outline get_c_outline,
    "primary" => set_h_primary set_c_primary get_h_primary get_c_primary,
    "on-primary" => set_h_on_primary set_c_on_primary get_h_on_primary get_c_on_primary,
    "primary-container" => set_h_primary_container set_c_primary_container get_h_primary_container get_c_primary_container,
    "on-primary-container" => set_h_on_primary_container set_c_on_primary_container get_h_on_primary_container get_c_on_primary_container,
    "success" => set_h_success set_c_success get_h_success get_c_success,
    "error" => set_h_error set_c_error get_h_error get_c_error,
);

/// Записать свой стиль в Theme.
pub fn apply(ui: &AppWindow, c: &Custom) {
    let t = ui.global::<Theme>();
    for k in KEYS {
        set_color(
            &t,
            k,
            c.colors.get(*k).map(|v| Color::from_argb_encoded(*v)),
        );
    }
    t.set_c_radius(c.radius.max(0.0));
    t.set_c_font(c.font.as_str().into());
    t.set_custom(c.enabled);
}

/// Шрифт, картинки и затемнение - при запуске.
fn apply_files(ui: &AppWindow, c: &Custom) {
    if !c.font_file.is_empty() {
        match register_font(&fonts_dir().join(&c.font_file)) {
            Ok(name) => ui.global::<Backend>().set_font_file(name.into()),
            Err(e) => {
                crate::notify::toast(ui, Kind::Error, format!("Свой шрифт не загрузился: {e}"))
            }
        }
    }
    let t = ui.global::<Theme>();
    t.set_bg_main(load_image(&c.bg_main));
    t.set_bg_panel(load_image(&c.bg_panel));
    t.set_bg_dim(c.bg_dim.unwrap_or(0.4).clamp(0.0, 1.0));
    FILES.with(|f| *f.borrow_mut() = (c.font_file.clone(), c.bg_main.clone(), c.bg_panel.clone()));
}

fn fonts_dir() -> PathBuf {
    crate::backend::data_dir().join("fonts")
}

fn bg_dir() -> PathBuf {
    crate::backend::data_dir().join("backgrounds")
}

fn load_image(file: &str) -> slint::Image {
    if file.is_empty() {
        return slint::Image::default();
    }
    slint::Image::load_from_path(&bg_dir().join(file)).unwrap_or_default()
}

/// Регистрирует шрифт в Slint; возвращает имя семейства для font-family.
fn register_font(path: &Path) -> Result<String, String> {
    use slint::fontique_011::fontique;
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut coll = slint::fontique_011::shared_collection();
    let fonts = coll.register_fonts(fontique::Blob::new(std::sync::Arc::new(data)), None);
    let (id, _) = fonts.first().ok_or("это не файл шрифта")?;
    coll.family_name(*id)
        .map(str::to_string)
        .ok_or_else(|| "у шрифта нет названия".into())
}

/// Копия выбранного файла в папку данных: исходный могут удалить или переместить.
fn copy_in(src: &Path, dir: &Path, name: &str) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let file = format!("{name}.{ext}");
    // прежняя картинка могла быть другого формата
    for old in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if old.path().file_stem().is_some_and(|s| s == name) {
            let _ = std::fs::remove_file(old.path());
        }
    }
    std::fs::copy(src, dir.join(&file)).map_err(|e| e.to_string())?;
    Ok(file)
}

/// Прочитать свой стиль из Theme (для сохранения настроек).
pub fn read(ui: &AppWindow) -> Custom {
    let t = ui.global::<Theme>();
    let (font_file, bg_main, bg_panel) = FILES.with(|f| f.borrow().clone());
    Custom {
        enabled: t.get_custom(),
        colors: custom_colors(&t),
        radius: t.get_c_radius(),
        font: t.get_c_font().into(),
        font_file,
        bg_main,
        bg_panel,
        bg_dim: Some(t.get_bg_dim()),
    }
}

pub fn dir() -> PathBuf {
    crate::backend::data_dir().join("themes")
}

fn files() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    v.sort();
    v
}

fn hex(c: Color) -> String {
    let v = c.as_argb_encoded();
    if v >> 24 == 0xff {
        format!("#{:06x}", v & 0xff_ffff)
    } else {
        format!("#{:08x}", v.rotate_left(8))
    }
}

/// "#rrggbb" или "#rrggbbaa"
fn parse(s: &str) -> Option<Color> {
    let h = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).ok()?;
    match h.len() {
        6 => Some(Color::from_argb_encoded(0xff00_0000 | v)),
        8 => Some(Color::from_argb_encoded(v.rotate_right(8))),
        _ => None,
    }
}

fn load_file(p: &Path) -> Result<StyleFile, String> {
    let text = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn file_title(p: &Path) -> String {
    load_file(p)
        .ok()
        .map(|f| f.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| p.file_stem().unwrap_or_default().to_string_lossy().into())
}

fn refresh(ui: &AppWindow) {
    let names: Vec<SharedString> = files().iter().map(|p| file_title(p).into()).collect();
    ui.global::<Backend>()
        .set_style_files(ModelRc::new(VecModel::from(names)));
}

/// Стиль из файла: тема, акцент и свои цвета; неизвестные поля - ошибка с понятным текстом.
fn load_into(ui: &AppWindow, p: &Path) -> Result<(), String> {
    let f = load_file(p)?;
    let t = ui.global::<Theme>();
    if let Some(b) = &f.base {
        let i = BASES
            .iter()
            .position(|x| x.eq_ignore_ascii_case(b))
            .ok_or(format!("base: нет темы \"{b}\""))?;
        t.set_style(i as i32);
    }
    if let Some(d) = f.dark {
        t.set_dark(d);
    }
    if let Some(a) = &f.accent {
        t.set_accent(parse(a).ok_or(format!("accent: не цвет \"{a}\""))?);
    }
    let mut colors = BTreeMap::new();
    for (k, v) in &f.colors {
        if !KEYS.contains(&k.as_str()) {
            return Err(format!("нет такого цвета: \"{k}\""));
        }
        let c = parse(v).ok_or(format!("{k}: не цвет \"{v}\""))?;
        colors.insert(k.clone(), c.as_argb_encoded());
    }
    apply(
        ui,
        &Custom {
            enabled: true,
            colors,
            radius: f.radius.unwrap_or(0.0),
            font: f.font.unwrap_or_default(),
            ..Default::default()
        },
    );
    Ok(())
}

fn save(ui: &AppWindow) -> Result<PathBuf, String> {
    let t = ui.global::<Theme>();
    let c = read(ui);
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let n = (1..)
        .find(|i| !dir().join(format!("Мой стиль {i}.json")).exists())
        .unwrap_or(1);
    let file = StyleFile {
        name: format!("Мой стиль {n}"),
        base: BASES.get(t.get_style() as usize).map(|s| s.to_string()),
        dark: Some(t.get_dark()),
        accent: Some(hex(t.get_accent())),
        colors: c
            .colors
            .iter()
            .map(|(k, v)| (k.clone(), hex(Color::from_argb_encoded(*v))))
            .collect(),
        radius: (c.radius > 0.0).then_some(c.radius),
        font: (!c.font.is_empty()).then_some(c.font),
    };
    let path = dir().join(format!("Мой стиль {n}.json"));
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn connect(ui: &AppWindow, custom: &Custom) {
    apply(ui, custom);
    apply_files(ui, custom);
    refresh(ui);
    connect_files(ui);
    let b = ui.global::<Backend>();
    b.on_color_hex(hex_ss);
    b.on_hex_valid(|s| parse(&s).is_some());
    b.on_hex_color(|s| parse(&s).unwrap_or_default());
    b.on_styles_reload({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                refresh(&ui);
            }
        }
    });
    b.on_style_load({
        let weak = ui.as_weak();
        move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let Some(p) = usize::try_from(i)
                .ok()
                .and_then(|i| files().get(i).cloned())
            else {
                return;
            };
            match load_into(&ui, &p) {
                Ok(()) => crate::notify::toast(
                    &ui,
                    Kind::Info,
                    format!("Стиль \"{}\" применён", file_title(&p)),
                ),
                Err(e) => {
                    crate::notify::toast(&ui, Kind::Error, format!("Стиль не загрузился: {e}"))
                }
            }
            ui.global::<Backend>().invoke_settings_changed();
        }
    });
    b.on_style_save({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            match save(&ui) {
                Ok(p) => {
                    refresh(&ui);
                    let _ = open::that_detached(dir());
                    crate::notify::toast(
                        &ui,
                        Kind::Info,
                        format!(
                            "Сохранено: {}",
                            p.file_name().unwrap_or_default().to_string_lossy()
                        ),
                    );
                }
                Err(e) => crate::notify::toast(
                    &ui,
                    Kind::Error,
                    format!("Не удалось сохранить стиль: {e}"),
                ),
            }
        }
    });
}

fn hex_ss(c: Color) -> SharedString {
    hex(c).into()
}

fn connect_files(ui: &AppWindow) {
    let b = ui.global::<Backend>();
    b.on_font_pick({
        let weak = ui.as_weak();
        move || {
            let weak = weak.clone();
            crate::backend::spawn(async move {
                let Some(f) = rfd::AsyncFileDialog::new()
                    .set_title("Свой шрифт")
                    .add_filter("Шрифты", &["ttf", "otf"])
                    .pick_file()
                    .await
                else {
                    return;
                };
                let src = f.path().to_path_buf();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let r = copy_in(&src, &fonts_dir(), "font").and_then(|file| {
                        register_font(&fonts_dir().join(&file)).map(|name| (file, name))
                    });
                    match r {
                        Ok((file, name)) => {
                            FILES.with(|f| f.borrow_mut().0 = file);
                            ui.global::<Backend>().set_font_file(name.as_str().into());
                            ui.global::<Theme>().set_c_font(name.as_str().into());
                            ui.global::<Backend>().invoke_settings_changed();
                        }
                        Err(e) => {
                            crate::notify::toast(&ui, Kind::Error, format!("Шрифт не подошёл: {e}"))
                        }
                    }
                });
            });
        }
    });
    b.on_bg_pick({
        let weak = ui.as_weak();
        move |slot| {
            let weak = weak.clone();
            crate::backend::spawn(async move {
                let Some(f) = rfd::AsyncFileDialog::new()
                    .set_title("Картинка фона")
                    .add_filter("Картинки", &["png", "jpg", "jpeg"])
                    .pick_file()
                    .await
                else {
                    return;
                };
                let src = f.path().to_path_buf();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let r = copy_in(&src, &bg_dir(), &slot).and_then(|file| {
                        let img = load_image(&file);
                        if img.size().width == 0 {
                            Err("картинка не открылась".to_string())
                        } else {
                            Ok((file, img))
                        }
                    });
                    match r {
                        Ok((file, img)) => {
                            let t = ui.global::<Theme>();
                            if slot == "panel" {
                                t.set_bg_panel(img);
                                FILES.with(|f| f.borrow_mut().2 = file);
                            } else {
                                t.set_bg_main(img);
                                FILES.with(|f| f.borrow_mut().1 = file);
                            }
                            ui.global::<Backend>().invoke_settings_changed();
                        }
                        Err(e) => crate::notify::toast(
                            &ui,
                            Kind::Error,
                            format!("Картинка не подошла: {e}"),
                        ),
                    }
                });
            });
        }
    });
    b.on_bg_clear({
        let weak = ui.as_weak();
        move |slot| {
            let Some(ui) = weak.upgrade() else { return };
            let t = ui.global::<Theme>();
            let file = FILES.with(|f| {
                let mut f = f.borrow_mut();
                std::mem::take(if slot == "panel" { &mut f.2 } else { &mut f.1 })
            });
            if slot == "panel" {
                t.set_bg_panel(slint::Image::default());
            } else {
                t.set_bg_main(slint::Image::default());
            }
            if !file.is_empty() {
                let _ = std::fs::remove_file(bg_dir().join(file));
            }
            ui.global::<Backend>().invoke_settings_changed();
        }
    });
}
