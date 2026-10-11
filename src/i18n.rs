//! Local, deterministic UI translations. User content is never sent for translation.
use eframe::egui;
use std::sync::OnceLock;

pub(crate) const LANGUAGES: [(&str, &str); 11] = [
    ("en", "English"),
    ("zh", "中文"),
    ("hi", "हिन्दी"),
    ("es", "Español"),
    ("ar", "العربية"),
    ("fr", "Français"),
    ("bn", "বাংলা"),
    ("pt", "Português"),
    ("ru", "Русский"),
    ("ur", "اردو"),
    ("id", "Bahasa Indonesia"),
];
pub(crate) fn language(ctx: &egui::Context) -> String {
    ctx.data(|d| d.get_temp::<String>(egui::Id::new("interface_language")))
        .unwrap_or_else(|| "ru".into())
}
pub(crate) fn set_language(ctx: &egui::Context, code: &str) {
    let code = if LANGUAGES.iter().any(|(c, _)| *c == code) {
        code
    } else {
        "ru"
    };
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("interface_language"), code.to_string()));
}
pub(crate) fn translated(code: &str, text: &str) -> String {
    static CATALOG: OnceLock<serde_json::Value> = OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../locales/interface.json"))
            .expect("validated embedded translations")
    });
    catalog
        .get(text)
        .and_then(|row| row.get(code))
        .and_then(|v| v.as_str())
        .unwrap_or(text)
        .to_string()
}
pub(crate) fn tr(ctx: &egui::Context, text: &str) -> String {
    translated(&language(ctx), text)
}
pub(crate) fn selector(ui: &mut egui::Ui, store: &crate::store::Store) {
    if ui
        .ctx()
        .data(|d| d.get_temp::<String>(egui::Id::new("interface_language")))
        .is_none()
    {
        set_language(
            ui.ctx(),
            &store
                .get("interface_language")
                .unwrap_or_else(|| "ru".into()),
        );
    }
    let mut code = language(ui.ctx());
    let selected = LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
        .unwrap_or("Русский");
    egui::ComboBox::from_id_salt("language_selector")
        .selected_text(selected)
        .width(110.0)
        .show_ui(ui, |ui| {
            for (c, name) in LANGUAGES {
                ui.selectable_value(&mut code, c.to_string(), name);
            }
        });
    if code != language(ui.ctx()) {
        match store.set_checked("interface_language", &code) {
            Ok(()) => set_language(ui.ctx(), &code),
            Err(e) => {
                ui.label(e);
            }
        }
    }
}

/// Use OS-installed fallback glyphs; do not redistribute proprietary fonts.
pub(crate) fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    let mut fonts = egui::FontDefinitions::default();
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "system-unicode".into(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("system-unicode".into());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_catalog_key_has_all_nonempty_translations() {
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("../locales/interface.json")).unwrap();
        for (key, row) in catalog.as_object().unwrap() {
            for (code, _) in LANGUAGES {
                assert!(
                    !row.get(code)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .is_empty(),
                    "{key}: {code}"
                );
            }
        }
    }
    #[test]
    fn unknown_language_and_content_are_safe() {
        let ctx = egui::Context::default();
        set_language(&ctx, "../../unknown");
        assert_eq!(language(&ctx), "ru");
        assert_eq!(
            translated("es", "user-written secret"),
            "user-written secret"
        );
        assert_eq!(translated("es", "Граф знаний"), "Grafo de conocimiento");
    }
}
