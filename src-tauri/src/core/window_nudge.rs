use std::time::Duration;

use crate::core::keyboard_driver::Platform;

pub const NUDGE_PIXELS: u32 = 1;
pub const NUDGE_DELAY: Duration = Duration::from_millis(500);
pub const NUDGE_HOLD: Duration = Duration::from_millis(50);
pub const MAIN_WINDOW_LABEL: &str = "main";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

pub trait NudgeTarget {
    fn inner_size(&self) -> Result<WindowSize, String>;
    fn is_maximized(&self) -> Result<bool, String>;
    fn is_fullscreen(&self) -> Result<bool, String>;
    fn set_inner_size(&self, size: WindowSize) -> Result<(), String>;
}

pub fn should_nudge(label: &str, finished: bool) -> bool {
    finished && label == MAIN_WINDOW_LABEL
}

pub fn nudge(platform: Platform, target: &impl NudgeTarget, mut hold: impl FnMut()) -> Result<(), String> {
    if platform != Platform::Linux {
        return Ok(());
    }
    let original = target.inner_size()?;
    if target.is_maximized()? || target.is_fullscreen()? {
        return Ok(());
    }
    let grown = WindowSize {
        width: original.width.saturating_add(NUDGE_PIXELS),
        height: original.height,
    };
    let grew = target.set_inner_size(grown);
    if grew.is_ok() {
        hold();
    }
    let restored = target.set_inner_size(original);
    grew.and(restored)
}

impl<R: tauri::Runtime> NudgeTarget for tauri::Window<R> {
    fn inner_size(&self) -> Result<WindowSize, String> {
        let size = tauri::Window::inner_size(self).map_err(|e| e.to_string())?;
        Ok(WindowSize {
            width: size.width,
            height: size.height,
        })
    }

    fn is_maximized(&self) -> Result<bool, String> {
        tauri::Window::is_maximized(self).map_err(|e| e.to_string())
    }

    fn is_fullscreen(&self) -> Result<bool, String> {
        tauri::Window::is_fullscreen(self).map_err(|e| e.to_string())
    }

    fn set_inner_size(&self, size: WindowSize) -> Result<(), String> {
        self.set_size(tauri::PhysicalSize::new(size.width, size.height))
            .map_err(|e| e.to_string())
    }
}

pub fn nudge_after_load<R: tauri::Runtime>(webview: &tauri::Webview<R>, event: tauri::webview::PageLoadEvent) {
    let finished = matches!(event, tauri::webview::PageLoadEvent::Finished);
    if !should_nudge(webview.label(), finished) {
        return;
    }
    let window = webview.window();
    std::thread::spawn(move || {
        std::thread::sleep(NUDGE_DELAY);
        let _ = nudge(Platform::current(), &window, || std::thread::sleep(NUDGE_HOLD));
    });
}

#[cfg(test)]
mod tests;
