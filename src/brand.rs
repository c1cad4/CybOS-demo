use base64::Engine;
use eframe::egui;

fn pixels() -> image::RgbaImage {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(include_str!("../assets/cybos-logo.png.base64").trim())
        .expect("bundled logo");
    image::load_from_memory(&bytes)
        .expect("bundled PNG")
        .into_rgba8()
}

pub(crate) fn window_icon() -> egui::IconData {
    let p = pixels();
    egui::IconData {
        width: p.width(),
        height: p.height(),
        rgba: p.into_raw(),
    }
}

pub(crate) fn show(ui: &mut egui::Ui, size: f32) {
    let id = egui::Id::new("cybos_brand_texture");
    let texture = ui
        .ctx()
        .data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = texture.unwrap_or_else(|| {
        let p = pixels();
        let image =
            egui::ColorImage::from_rgba_unmultiplied([p.width() as usize, p.height() as usize], &p);
        let handle = ui
            .ctx()
            .load_texture("CybOS", image, egui::TextureOptions::LINEAR);
        ui.ctx()
            .data_mut(|data| data.insert_temp(id, handle.clone()));
        handle
    });
    ui.image((texture.id(), egui::vec2(size, size)));
}

#[cfg(test)]
mod tests {
    #[test]
    fn rounded_icon_has_transparent_corners() {
        let p = super::pixels();
        for (x, y) in [
            (0, 0),
            (p.width() - 1, 0),
            (0, p.height() - 1),
            (p.width() - 1, p.height() - 1),
        ] {
            assert_eq!(p.get_pixel(x, y)[3], 0, "Icon corner must be transparent");
        }
        assert!(p.get_pixel(p.width() / 2, p.height() / 2)[3] > 0);
    }
}
