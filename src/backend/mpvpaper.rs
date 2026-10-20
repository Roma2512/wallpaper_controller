use std::path::Path;

pub fn apply(wallpaper: &Path) -> std::io::Result<()> {
    // Пример с mpvpaper (Wayland) — раскомментируйте и адаптируйте:
    // std::process::Command::new("mpvpaper")
    //     .args(["-o", "no-audio --loop", "*", wallpaper.to_str().unwrap()])
    //     .spawn()?;
    let _ = wallpaper;
    Ok(())
}