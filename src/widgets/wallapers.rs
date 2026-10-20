use eframe::egui;
use std::path::{Path, PathBuf};

/// Состояние виджета: список обоев, поиск, выбранный элемент.
pub struct WallpapersWidget {
    /// Все доступные обои (пути к файлам).
    pub items: Vec<PathBuf>,
    /// Отфильтрованный список (по поиску).
    filtered: Vec<usize>,
    /// Индекс выбранного элемента в `items`.
    pub selected: Option<usize>,
    /// Строка поиска.
    search: String,
    /// Кеш текстур (изображения и превью видео).
    textures: std::collections::HashMap<PathBuf, egui::TextureHandle>,
}

impl Default for WallpapersWidget {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            filtered: Vec::new(),
            selected: None,
            search: String::new(),
            textures: Default::default(),
        }
    }
}

impl WallpapersWidget {
    pub fn load_from_dir(&mut self, dir: &Path) {
        self.items.clear();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_ascii_lowercase();
                    // Добавляем видеоформаты в список поддерживаемых
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp" | "mp4" | "mkv" | "webm" | "mov") {
                        self.items.push(p);
                    }
                }
            }
        }
        self.items.sort();
        self.refilter();
    }

    fn refilter(&mut self) {
        let q = self.search.to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                q.is_empty()
                    || p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase().contains(&q))
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();
    }

    fn texture_for(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        if let Some(t) = self.textures.get(path) {
            return Some(t.clone());
        }

        let img = if is_video(path) {
            extract_first_frame(path).ok()?
        } else {
            image::open(path).ok()?
        };

        let thumb = img.thumbnail(256, 256).to_rgba8();
        let size = [thumb.width() as usize, thumb.height() as usize];
        let color_img = egui::ColorImage::from_rgba_unmultiplied(size, thumb.as_raw());
        let handle = ctx.load_texture(
            path.to_string_lossy(),
            color_img,
            egui::TextureOptions::LINEAR,
        );
        self.textures.insert(path.to_path_buf(), handle.clone());
        Some(handle)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("🔍");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Поиск обоев…")
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                self.refilter();
            }
        });

        ui.separator();

        // Фиксированный размер иконки (зум убран)
        let thumb_size = 160.0;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.filtered.is_empty() {
                    ui.weak("Нет обоев. Укажите папку с изображениями или видео.");
                    return;
                }

                let spacing = 8.0;
                let avail = ui.available_width();
                let cell = thumb_size + spacing;
                let cols = ((avail + spacing) / cell).floor().max(1.0) as usize;

                let ctx = ui.ctx().clone();
                let filtered_snapshot: Vec<usize> = self.filtered.clone();
                let mut handles: Vec<Option<egui::TextureHandle>> =
                    Vec::with_capacity(filtered_snapshot.len());
                for &idx in &filtered_snapshot {
                    let path = self.items[idx].clone();
                    handles.push(self.texture_for(&ctx, &path));
                }

                egui::Grid::new("wallpapers_grid")
                    .num_columns(cols)
                    .spacing([spacing, spacing])
                    .show(ui, |ui| {
                        for (i, &idx) in filtered_snapshot.iter().enumerate() {
                            let path = &self.items[idx];
                            let is_selected = self.selected == Some(idx);

                            let mut frame = egui::Frame::new().inner_margin(4.0);
                            if is_selected {
                                frame = frame
                                    .stroke(egui::Stroke::new(3.0, egui::Color32::from_rgb(120, 170, 255)))
                                    .corner_radius(6.0);
                            } else {
                                frame = frame.corner_radius(6.0);
                            }

                            let resp = frame.show(ui, |ui| {
                                ui.set_width(thumb_size);
                                ui.vertical(|ui| {
                                    if let Some(t) = &handles[i] {
                                        let size = egui::vec2(thumb_size, thumb_size * 0.65);
                                        ui.add(
                                            egui::Image::new(t)
                                                .fit_to_exact_size(size)
                                                .corner_radius(4.0),
                                        );
                                    } else {
                                        ui.allocate_space(egui::vec2(
                                            thumb_size,
                                            thumb_size * 0.65,
                                        ));
                                        ui.weak("(ошибка загрузки)");
                                    }
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("?");
                                    ui.label(
                                        egui::RichText::new(name)
                                            .size(12.0)
                                            .color(egui::Color32::LIGHT_GRAY),
                                    );
                                });
                            })
                                .response
                                .interact(egui::Sense::click());

                            if resp.clicked() {
                                self.selected = Some(idx);
                                changed = true;
                            }
                            if resp.double_clicked() {
                                self.selected = Some(idx);
                                changed = true;
                            }

                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });

        changed
    }

    /// Текущий выбранный путь.
    pub fn selected_path(&self) -> Option<&Path> {
        self.selected.and_then(|i| self.items.get(i)).map(|p| p.as_path())
    }
}

/// Проверка, является ли файл видео.
fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "mp4" | "mkv" | "webm" | "mov" | "avi"))
        .unwrap_or(false)
}

/// Извлечь первый кадр из видео через ffmpeg-next.
/// Возвращает `DynamicImage` для единообразной обработки с изображениями.
fn extract_first_frame(path: &Path) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    use ffmpeg_next as ffmpeg;
    use ffmpeg::format::input;
    use ffmpeg::media::Type;
    use ffmpeg::software::scaling::{context::Context as ScalingContext, flag::Flags};
    use ffmpeg::util::frame::video::Video;

    ffmpeg::init()?;

    let mut ictx = input(path)?;
    let input_stream = ictx
        .streams()
        .best(Type::Video)
        .ok_or("no video stream")?;
    let stream_index = input_stream.index();

    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    let mut scaler = ScalingContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGBA,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    let mut frame = Video::empty();
    let mut rgb_frame = Video::empty();

    // Читаем пакеты, пока не декодируем первый кадр
    for (stream, packet) in ictx.packets() {
        if stream.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet)?;
        if decoder.receive_frame(&mut frame).is_ok() {
            scaler.run(&frame, &mut rgb_frame)?;
            let data = rgb_frame.data(0);
            let width = rgb_frame.width() as u32;
            let height = rgb_frame.height() as u32;
            let img = image::RgbaImage::from_raw(width, height, data.to_vec())
                .ok_or("failed to create image from frame")?;
            return Ok(image::DynamicImage::ImageRgba8(img));
        }
    }

    Err("no frame decoded".into())
}