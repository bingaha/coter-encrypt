use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

/// 系统级消息提示：桌面横幅 + 原生弹窗（仅「我知道了」）。
///
/// Linux 桌面通知使用 `notify-send`；其它平台走 tauri-plugin-notification。
/// 同时弹出 MessageDialog，按钮仅为「我知道了」，无业务操作。
/// 返回值仅反映桌面通知结果，供调用方写业务日志；弹窗异步展示，不阻塞。
pub fn show_system_notification(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    let dialog_body = body.to_string();
    // 多行正文在部分桌面环境下展示异常，桌面通知统一压成单行。
    let banner_body = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");

    let result = {
        #[cfg(target_os = "linux")]
        {
            show_linux_notification(title, &banner_body)
        }

        #[cfg(not(target_os = "linux"))]
        {
            app.notification()
                .builder()
                .title(title)
                .body(&banner_body)
                .show()
                .map_err(|err| format!("{err}"))
        }
    };

    app.dialog()
        .message(dialog_body)
        .title(title.to_string())
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCustom("我知道了".into()))
        .show(|_| {});

    result
}

#[cfg(target_os = "linux")]
fn show_linux_notification(title: &str, body: &str) -> Result<(), String> {
    // 不传 -a / desktop-entry：GNOME 下同应用未关闭通知会挡住后续横幅；
    // 与已确认可见的「notify-send 手动测试」保持同一通路。
    let status = std::process::Command::new("notify-send")
        .args([
            "-u",
            "normal",
            "-t",
            "12000",
            "--",
            title,
            body,
        ])
        .status()
        .map_err(|err| format!("spawn notify-send failed: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("notify-send exit status: {status}"))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::show_linux_notification;

    /// 会弹出桌面通知；请单独运行，且确认上一条相关通知已关闭后再测：
    /// `cargo test linux_desktop_notification_sends_ok -- --ignored --nocapture`
    #[test]
    #[ignore = "pops a desktop notification; run manually after previous banner is closed"]
    fn linux_desktop_notification_sends_ok() {
        show_linux_notification(
            "合并监控通知测试",
            "请确认看到这一条桌面横幅",
        )
        .expect("notify-send should succeed");
    }
}
