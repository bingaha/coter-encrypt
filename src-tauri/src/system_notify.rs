use serde::Serialize;
use tauri::{AppHandle, Emitter};
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

const SYSTEM_NOTIFY_EVENT: &str = "system-notify";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemNotifyPayload {
    title: String,
    body: String,
    ok: bool,
}

/// 系统级消息提示（无按钮、不阻塞）。
///
/// Linux 使用 `notify-send`（与本机已验证可见的通路一致，且不绑定易被挡住的应用槽位）；
/// 其它平台走 tauri-plugin-notification。
/// 无论桌面通知是否成功，都会再发 `system-notify` 事件，供应用内提示。
/// 失败返回 Err，由调用方写业务日志；不回退 MessageDialog。
pub fn show_system_notification(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    // 多行正文在部分桌面环境下展示异常，统一压成单行。
    let body = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");

    let result = {
        #[cfg(target_os = "linux")]
        {
            let _ = app;
            show_linux_notification(title, &body)
        }

        #[cfg(not(target_os = "linux"))]
        {
            app.notification()
                .builder()
                .title(title)
                .body(&body)
                .show()
                .map_err(|err| format!("{err}"))
        }
    };

    let _ = app.emit(
        SYSTEM_NOTIFY_EVENT,
        SystemNotifyPayload {
            title: title.to_string(),
            body: body.clone(),
            ok: result.is_ok(),
        },
    );

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
