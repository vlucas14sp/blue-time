//! Autostart on login via a desktop entry in `~/.config/autostart`.

use std::fs;
use std::path::PathBuf;

const FILE_NAME: &str = "io.github.vlucas14sp.BlueTime.desktop";

fn path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("autostart").join(FILE_NAME))
}

pub fn is_enabled() -> bool {
    path().is_some_and(|p| p.exists())
}

pub fn set_enabled(enabled: bool) {
    let Some(path) = path() else { return };
    if !enabled {
        let _ = fs::remove_file(path);
        return;
    }
    let Ok(exec) = std::env::current_exe() else {
        return;
    };
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Blue Time\n\
         Comment=Pomodoro timer\n\
         Comment[pt_BR]=Timer pomodoro\n\
         Exec={} --hidden\n\
         Icon=io.github.vlucas14sp.BlueTime\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exec.display()
    );
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(path, entry);
}
