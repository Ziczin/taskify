//! Настоящие системные уведомления. Вызываются только по явной настройке.

use anyhow::{Context, Result};

/// Показывает системное уведомление средствами ОС.
pub fn notify(title: &str, body: &str) -> Result<()> {
    notify_rust::Notification::new()
        .appname("Taskify")
        .app_id("Taskify")
        .summary(title)
        .body(body)
        .timeout(notify_rust::Timeout::Milliseconds(7000))
        .show()
        .context("система не приняла уведомление")?;
    Ok(())
}
