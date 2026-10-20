use eframe::egui;
use std::path::Path;

/// Виджет с информацией о выбранных обоях.
pub struct InfoWidget {
    /// Кеш превью для отображения.
    preview: Option<egui::TextureHandle>,
    preview_path: Option<std::path::PathBuf>,
    /// Метаданные выбранного файла.
    meta: Option<FileMeta>,
    /// Сообщение статуса (например, "Применено").
    pub status: Option<String>,
}

struct FileMeta {
    name: String,
    size_px: Option<(u32, u32)>,
    size_bytes: u64,
    format: String,
}

impl Default for InfoWidget {
    fn default() -> Self {
        Self {
            preview: None,
            preview_path: None,
            meta: None,
            status: None,
        }
    }
}

impl InfoWidget {
    /// Обновить панель под выбранный файл.
    pub fn set_selected(&mut self, ctx: &egui::Context, path: Option<&Path>) {
        let new_path = path.map(|p| p.to_path_buf());
        if self.preview_path == new_path {
            return;
        }
        self.preview_path = new_path.clone();
        self.preview = None;
        self.meta = None;

        let Some(p) = new_path else { return };

        // Метаданные.
        if let Ok(meta) = std::fs::metadata(&p) {
            let format = image::ImageFormat::from_path(&p)
                .map(|f| format!("{f:?}"))
                .unwrap_or_else(|_| "unknown".into());
            let size_px = image::image_dimensions(&p).ok();
            self.meta = Some(FileMeta {
                name: p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string(),
                size_px,
                size_bytes: meta.len(),
                format,
            });
        }

        // Превью.
        if let Ok(img) = image::open(&p) {
            let thumb = img.thumbnail(512, 512).to_rgba8();
            let size = [thumb.width() as usize, thumb.height() as usize];
            let ci = egui::ColorImage::from_rgba_unmultiplied(size, thumb.as_raw());
            self.preview = Some(ctx.load_texture(
                format!("info_preview_{}", p.display()),
                ci,
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    /// Отрисовать панель. Возвращает true, если пользователь нажал «Применить».
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut apply = false;

        ui.heading("Информация");
        ui.separator();

        if self.meta.is_none() {
            ui.weak("Выберите обои слева, чтобы увидеть детали.");
            return false;
        }

        if let Some(prev) = &self.preview {
            let avail = ui.available_width();
            let h = (avail * 0.6).min(360.0);
            ui.add(egui::Image::new(prev).fit_to_exact_size(egui::vec2(avail, h)));
            ui.add_space(8.0);
        }

        let meta = self.meta.as_ref().unwrap();
        egui::Grid::new("info_meta")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Имя:");
                ui.label(&meta.name);
                ui.end_row();

                ui.strong("Формат:");
                ui.label(&meta.format);
                ui.end_row();

                ui.strong("Размер:");
                if let Some((w, h)) = meta.size_px {
                    ui.label(format!("{w} × {h} px"));
                } else {
                    ui.weak("неизвестно");
                }
                ui.end_row();

                ui.strong("Файл:");
                ui.label(format_bytes(meta.size_bytes));
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("🖼 Применить как обои").clicked() {
                apply = true;
            }
            if ui.button("📂 Открыть папку").clicked() {
                if let Some(p) = &self.preview_path {
                    if let Some(dir) = p.parent() {
                        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                    }
                }
            }
        });

        if let Some(msg) = &self.status {
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::from_rgb(120, 200, 120), msg);
        }

        apply
    }
}

fn format_bytes(b: u64) -> String {
    const K: f64 = 1024.0;
    let f = b as f64;
    if f < K {
        format!("{b} B")
    } else if f < K * K {
        format!("{:.1} KB", f / K)
    } else if f < K * K * K {
        format!("{:.1} MB", f / (K * K))
    } else {
        format!("{:.2} GB", f / (K * K * K))
    }
}