//! Настоящие системные уведомления. Вызываются только по явной настройке.

use anyhow::{Context, Result};

/// Показывает системное уведомление средствами ОС.
pub fn notify(title: &str, body: &str) -> Result<()> {
    let mut notification = notify_rust::Notification::new();
    notification
        .appname("Taskify")
        .summary(title)
        .body(body)
        .timeout(notify_rust::Timeout::Milliseconds(7000));
    // AppUserModelID есть только в Windows-бэкенде notify-rust.
    #[cfg(target_os = "windows")]
    notification.app_id("Taskify");
    notification
        .show()
        .context("система не приняла уведомление")?;
    Ok(())
}
