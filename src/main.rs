use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::process::{Child, Command};
use eframe::egui;

#[derive(Clone)]
enum TrayCommand {
    ShowWindow,
    Quit,
}

#[derive(PartialEq)]
enum WallpaperType {
    Static,
    Live,
}

struct App {
    rx: Arc<Mutex<Receiver<TrayCommand>>>,
    current_mode: WallpaperType,
    static_path: String,
    live_path: String,
    // Указатель на запущенный процесс живых обоев (например, mpvpaper или live-paper)
    live_process: Option<Child>,
}

impl App {
    // Функция применения статических обоев (пример для GNOME / feh)
    fn apply_static_wallpaper(&mut self, path: &str) {
        self.stop_live_wallpaper();
        println!("Установка статических обоев: {}", path);

        // Пример для GNOME (можно заменить на feh, nitrogen и т.д.):
        let _ = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.desktop.background",
                "picture-uri",
                &format!("file://{}", path),
            ])
            .spawn();
    }

    // Функция запуска живых обоев
    fn start_live_wallpaper(&mut self, path: &str) {
        self.stop_live_wallpaper();
        println!("Запуск живых обоев: {}", path);

        // Пример запуска через mpvpaper или вашу утилиту live-paper:
        // Замените "mpvpaper" на ту команду, которую использует live-paper в вашей системе
        let child = Command::new("mpvpaper")
            .args(["-o", "no-audio loop", "DP-1", path]) // Пример для mpvpaper
            .spawn();

        match child {
            Ok(process) => {
                self.live_process = Some(process);
            }
            Err(e) => {
                eprintln!("Не удалось запустить живые обои: {}", e);
            }
        }
    }

    // Остановка процесса живых обоев
    fn stop_live_wallpaper(&mut self) {
        if let Some(mut process) = self.live_process.take() {
            let _ = process.kill();
            let _ = process.wait();
            println!("Живые обои остановлены.");
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Гарантированно гасим процесс обоев при выходе из приложения
        self.stop_live_wallpaper();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint();

        // Обработка команд из трея
        if let Ok(mut rx) = self.rx.try_lock() {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    TrayCommand::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    TrayCommand::Quit => {
                        self.stop_live_wallpaper();
                        std::process::exit(0);
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Контроллер обоев (Linux)");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.selectable_label(self.current_mode == WallpaperType::Static, "Статические обои").clicked() {
                    self.current_mode = WallpaperType::Static;
                }
                if ui.selectable_label(self.current_mode == WallpaperType::Live, "Живые обои").clicked() {
                    self.current_mode = WallpaperType::Live;
                }
            });

            ui.separator();

            match self.current_mode {
                WallpaperType::Static => {
                    ui.label("Путь к изображению:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.static_path);
                        if ui.button("Применить").clicked() {
                            let path = self.static_path.clone();
                            self.apply_static_wallpaper(&path);
                        }
                    });
                }
                WallpaperType::Live => {
                    ui.label("Путь к видео / анимации:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.live_path);
                        if ui.button("Запустить").clicked() {
                            let path = self.live_path.clone();
                            self.start_live_wallpaper(&path);
                        }
                    });
                    if ui.button("Остановить живые обои").clicked() {
                        self.stop_live_wallpaper();
                    }
                }
            }

            ui.add_space(20.0);
            if ui.button("Свернуть в трей").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        });
    }
}

fn spawn_tray(tx: Sender<TrayCommand>) {
    thread::spawn(move || {
        let mut fb = muri::render::Framebuffer::new(16, 16);
        let px = fb.pixels_mut();
        for i in 0..256usize {
            let (x, y) = ((i % 16) as i32 - 8, (i / 16) as i32 - 8);
            let a: u16 = if x * x + y * y <= 49 { 255 } else { 0 };
            let o = i * 4;
            px[o] = (120 * a / 255) as u8;
            px[o + 1] = (170 * a / 255) as u8;
            px[o + 2] = (255 * a / 255) as u8;
            px[o + 3] = a as u8;
        }
        let icon = muri::Icon::from_png(fb.encode_png());

        let menu = muri::Menu::new()
            .row(muri::Row::new("show").label("Показать окно"))
            .row(muri::Row::new("quit").label("Выход"));

        let tx_clone = tx.clone();
        let tray = muri::Tray::new(icon)
            .tooltip("Wallpaper Controller")
            .menu(menu)
            .on_click(move |id| {
                match id.as_str() {
                    "show" => { let _ = tx_clone.send(TrayCommand::ShowWindow); }
                    "quit" => { let _ = tx_clone.send(TrayCommand::Quit); std::process::exit(0); }
                    _ => {}
                }
            });

        let m = muri::MainThreadMarker::new().expect("main thread");
        tray.run(m).expect("tray run");
    });
}

fn main() {
    let (tx, rx) = mpsc::channel::<TrayCommand>();
    let rx = Arc::new(Mutex::new(rx));

    spawn_tray(tx.clone());

    loop {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([400.0, 300.0])
                .with_resizable(false),
            ..Default::default()
        };

        let rx_clone = Arc::clone(&rx);
        let _ = eframe::run_native(
            "Wallpaper Controller",
            options,
            Box::new(move |_cc| Ok(Box::new(App {
                rx: rx_clone,
                current_mode: WallpaperType::Static,
                static_path: String::new(),
                live_path: String::new(),
                live_process: None,
            }))),
        );

        match rx.lock().unwrap().recv() {
            Ok(TrayCommand::ShowWindow) => continue,
            Ok(TrayCommand::Quit) | Err(_) => break,
        }
    }
}