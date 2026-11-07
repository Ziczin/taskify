//! Короткие звуки интерфейса: файлы лежат в `assets/sounds`, выбранный вариант
//! приходит из косметики (`sound.ui`).

/// Проигрывает файл пака асинхронно (не блокирует кадр).
pub fn play(path: &std::path::Path) {
    if !path.exists() {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        const SND_ASYNC: u32 = 0x0001;
        const SND_FILENAME: u32 = 0x0002_0000;
        // SAFETY: строка с завершающим нулём живёт до конца вызова, звук играет
        // асинхронно и читает файл сам.
        unsafe {
            PlaySoundW(wide.as_ptr(), 0, SND_FILENAME | SND_ASYNC);
        }
    }
    #[cfg(not(windows))]
    {
        // На Linux и macOS пользуемся системным плеером: звуки короткие,
        // а тянуть отдельный аудиодвижок ради кликов не хочется.
        let players: &[&str] = if cfg!(target_os = "macos") {
            &["afplay"]
        } else {
            &["paplay", "aplay"]
        };
        for player in players {
            if std::process::Command::new(player)
                .arg(path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return;
            }
        }
    }
}

#[cfg(windows)]
#[link(name = "winmm")]
unsafe extern "system" {
    fn PlaySoundW(name: *const u16, module: isize, flags: u32) -> i32;
}
