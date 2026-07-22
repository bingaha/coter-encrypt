use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// 系统级消息提示（无按钮、不阻塞）。失败只打日志，不回退 Dialog。
pub fn show_system_notification(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        eprintln!("[system_notify] failed: {err}; title={title}; body={body}");
    }
}
