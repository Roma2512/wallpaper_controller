mod backend;
mod widgets;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use eframe::egui;

use widgets::info::InfoWidget;
use widgets::wallapers::WallpapersWidget;

#[derive(Clone)]
enum TrayCommand {
    ShowWindow,
    Quit,
}

struct App {
    rx: Arc<Mutex<Receiver<TrayCommand>>>,
    wallpapers: WallpapersWidget,
    info: InfoWidget,
    wallpapers_dir: PathBuf,
}

impl App {
    fn new(rx: Arc<Mutex<Receiver<TrayCommand>>>) -> Self {
        let wallpapers_dir = dirs_wallpapers_dir();
        let mut wallpapers = WallpapersWidget::default();
        wallpapers.load_from_dir(&wallpapers_dir);
        Self {
            rx,
            wallpapers,
            info: InfoWidget::default(),
            wallpapers_dir,
        }
    }
}

fn dirs_wallpapers_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(&home).join("Pictures").join("Wallpapers");
        if p.is_dir() {
            return p;
        }
        return PathBuf::from(home).join(".local/share/wallpapers");
    }
    PathBuf::from("wallpapers")
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint();

        // Обработка команд системного трея
        if let Ok(mut rx) = self.rx.try_lock() {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    TrayCommand::ShowWindow => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    TrayCommand::Quit => {
                        std::process::exit(0);
                    }
                }
            }
        }

        // Правая панель: информация о выбранных обоях
        egui::Panel::right("info_panel")
            .resizable(true)
            .default_size(340.0)
            .min_size(260.0)
            .show(ui, |ui| {
                let selected = self.wallpapers.selected_path().map(|p| p.to_path_buf());
                self.info.set_selected(&ctx, selected.as_deref());

                if self.info.ui(ui) {
                    if let Some(p) = self.wallpapers.selected_path() {
                        backend::mpvpaper::apply(p);
                        self.info.status = Some(format!(
                            "✓ Применено: {}",
                            p.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                        ));
                    }
                }
            });
        
        egui::CentralPanel::default().show(ui, |ui| {
            self.wallpapers.ui(ui);
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
            .row(muri::Row::new("show").label("Show Window"))
            .row(muri::Row::new("quit").label("Quit"));

        let tx_clone = tx.clone();
        let tray = muri::Tray::new(icon)
            .tooltip("test")
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
                .with_inner_size([1100.0, 700.0])
                .with_transparent(true),
            ..Default::default()
        };

        let rx_clone = Arc::clone(&rx);
        let _ = eframe::run_native(
            "Wallpaper Manager",
            options,
            Box::new(move |_cc| Ok(Box::new(App::new(rx_clone)))),
        );

        match rx.lock().unwrap().recv() {
            Ok(TrayCommand::ShowWindow) => continue,
            Ok(TrayCommand::Quit) | Err(_) => break,
        }
    }
}