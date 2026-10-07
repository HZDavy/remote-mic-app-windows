//! 前台窗口观测：#213「按前台应用切换快捷键映射方案」的前台检测基座。
//!
//! 只采样用于 profile 匹配的判据（进程名 + 窗口标题），不带隐私主体信息
//! （不以完整路径 / URL / 用户文件为键）。匹配逻辑在
//! [`crate::send_input::WindowMatcher`]。

/// 一次前台采样：进程名（如 `chrome.exe`，`None` = 采样失败）与窗口标题
/// （`None` = 无标题或多标签页未暴露标题的应用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundInfo {
    /// 前台进程的可执行文件名（`QueryFullProcessImageNameW` 取最后一段）。
    pub process_name: Option<String>,
    /// 前台窗口文本（`GetWindowTextW`，不含进程路径）。
    pub title: Option<String>,
}

impl ForegroundInfo {
    /// 无任何可作判据的信息（采样彻底失败）：当作无前台，任何 matcher 都不命中。
    pub fn is_empty(&self) -> bool {
        self.process_name.is_none() && self.title.is_none()
    }
}

/// 采样当前前台窗口。`None` = 无前台窗口或提取失败（引擎保持上一次生效态）。
pub fn current_foreground() -> Option<ForegroundInfo> {
    #[cfg(windows)]
    {
        foreground_windows()
    }
    #[cfg(not(windows))]
    {
        let _ = ();
        None
    }
}

#[cfg(windows)]
fn foreground_windows() -> Option<ForegroundInfo> {
    use windows::core::PWSTR;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.0.is_null() {
            return None;
        }
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(foreground, Some(&mut process_id));

        let mut process_name = None;
        if process_id != 0 {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok();
            if let Some(handle) = handle {
                let mut buffer = [0u16; 512];
                let mut size = buffer.len() as u32;
                let queried = QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_FORMAT(0),
                    PWSTR(buffer.as_mut_ptr()),
                    &mut size,
                )
                .is_ok();
                let _ = windows::Win32::Foundation::CloseHandle(handle);
                if queried {
                    let full = String::from_utf16_lossy(&buffer[..size as usize]);
                    process_name = Some(
                        full.rsplit(['\\', '/'])
                            .next()
                            .unwrap_or("unknown")
                            .to_owned(),
                    );
                }
            }
        }

        let mut title = None;
        let mut title_buffer = [0u16; 1024];
        let title_len = GetWindowTextW(foreground, &mut title_buffer);
        if title_len > 0 {
            title = Some(String::from_utf16_lossy(
                &title_buffer[..title_len as usize],
            ));
        }

        let info = ForegroundInfo {
            process_name,
            title,
        };
        if info.is_empty() {
            None
        } else {
            Some(info)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_info_has_no_criteria() {
        assert!(ForegroundInfo {
            process_name: None,
            title: None,
        }
        .is_empty());
        assert!(!ForegroundInfo {
            process_name: Some("notepad.exe".to_owned()),
            title: None,
        }
        .is_empty());
    }
}
