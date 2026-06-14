use std::mem::size_of;

use windows::{
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{
            ANTIALIASED_QUALITY, CLIP_DEFAULT_PRECIS, CreateBitmap, CreateCompatibleBitmap,
            CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_CHARSET,
            DEFAULT_PITCH, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW,
            FW_BOLD, FillRect, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ, HPEN, OUT_DEFAULT_PRECIS,
            PS_SOLID, RoundRect, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
                NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CS_HREDRAW, CS_VREDRAW, CreateIconIndirect, CreatePopupMenu,
                CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow,
                DispatchMessageW, GWLP_USERDATA, GetCursorPos, GetMessageW, GetWindowLongPtrW,
                HICON, HMENU, ICONINFO, IDC_ARROW, KillTimer, LoadCursorW, MF_DISABLED, MF_GRAYED,
                MF_SEPARATOR, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
                SW_HIDE, SetForegroundWindow, SetTimer, SetWindowLongPtrW, ShowWindow,
                TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON, TrackPopupMenu, WINDOW_EX_STYLE,
                WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP,
                WM_TIMER, WM_USER, WNDCLASSW, WS_OVERLAPPED,
            },
        },
    },
    core::PCWSTR,
};

use crate::{
    battery::{BatterySnapshot, probe_battery},
    error::AppError,
};

const WINDOW_CLASS_NAME: &str = "razer-bat-tray-window";
const WINDOW_TITLE: &str = "razer-bat";
const TRAY_UID: u32 = 1;
const TRAY_CALLBACK_MESSAGE: u32 = WM_USER + 1;
const POLL_TIMER_ID: usize = 1;
const MENU_REFRESH_ID: usize = 1001;
const MENU_EXIT_ID: usize = 1002;
const DEFAULT_POLL_MS: u32 = 60_000;
const SECOND_FAILURE_POLL_MS: u32 = 120_000;
const REPEATED_FAILURE_POLL_MS: u32 = 300_000;
const ICON_SIZE: i32 = 64;

pub fn run() -> Result<(), AppError> {
    unsafe { run_tray() }
}

struct TrayState {
    hwnd: HWND,
    icon: HICON,
    snapshot: Option<BatterySnapshot>,
    last_error: Option<AppError>,
    failure_count: u8,
}

impl TrayState {
    fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            icon: HICON::default(),
            snapshot: None,
            last_error: Some(AppError::NoDevice),
            failure_count: 0,
        }
    }

    unsafe fn initialize(&mut self) -> Result<(), AppError> {
        self.icon = unsafe { create_tray_icon(self.icon_label(), self.icon_accent()) }?;

        let mut data = self.notify_data(NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP);
        data.hIcon = self.icon;
        copy_wide_array(&mut data.szTip, &self.tooltip());

        if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
            if !self.icon.is_invalid() {
                let _ = unsafe { DestroyIcon(self.icon) };
                self.icon = HICON::default();
            }
            return Err(AppError::Tray);
        }

        let mut version_data = self.notify_data(Default::default());
        version_data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &version_data);
        }

        unsafe {
            self.poll();
        }
        Ok(())
    }

    unsafe fn poll(&mut self) {
        match probe_battery() {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.last_error = None;
                self.failure_count = 0;
            }
            Err(error) => {
                self.snapshot = None;
                self.last_error = Some(error);
                self.failure_count = self.failure_count.saturating_add(1);
            }
        }

        unsafe {
            self.update_icon();
            self.reset_timer();
        }
    }

    unsafe fn update_icon(&mut self) {
        let Ok(next_icon) = (unsafe { create_tray_icon(self.icon_label(), self.icon_accent()) })
        else {
            return;
        };

        let mut data = self.notify_data(NIF_ICON | NIF_TIP | NIF_SHOWTIP);
        data.hIcon = next_icon;
        copy_wide_array(&mut data.szTip, &self.tooltip());

        if unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
            let previous_icon = self.icon;
            self.icon = next_icon;

            if !previous_icon.is_invalid() {
                let _ = unsafe { DestroyIcon(previous_icon) };
            }
        } else if !next_icon.is_invalid() {
            let _ = unsafe { DestroyIcon(next_icon) };
        }
    }

    unsafe fn reset_timer(&self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), POLL_TIMER_ID) };
        unsafe {
            SetTimer(Some(self.hwnd), POLL_TIMER_ID, self.next_poll_ms(), None);
        }
    }

    fn next_poll_ms(&self) -> u32 {
        match self.failure_count {
            0 | 1 => DEFAULT_POLL_MS,
            2 => SECOND_FAILURE_POLL_MS,
            _ => REPEATED_FAILURE_POLL_MS,
        }
    }

    unsafe fn show_menu(&mut self) {
        let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
            return;
        };

        unsafe {
            append_menu_text(
                menu,
                MF_STRING | MF_DISABLED | MF_GRAYED,
                0,
                "[ Razer Battery ]",
            );
            append_menu_text(
                menu,
                MF_STRING | MF_DISABLED | MF_GRAYED,
                0,
                &self.menu_status(),
            );
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            append_menu_text(menu, MF_STRING, MENU_REFRESH_ID, "Refresh");
            append_menu_text(menu, MF_STRING, MENU_EXIT_ID, "Exit");
        }

        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            unsafe {
                let _ = SetForegroundWindow(self.hwnd);
                let _ = TrackPopupMenu(
                    menu,
                    TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    None,
                    self.hwnd,
                    None,
                );
                let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            }
        }

        let _ = unsafe { DestroyMenu(menu) };
    }

    unsafe fn cleanup(&mut self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), POLL_TIMER_ID) };
        let data = self.notify_data(Default::default());
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        }

        if !self.icon.is_invalid() {
            let _ = unsafe { DestroyIcon(self.icon) };
            self.icon = HICON::default();
        }
    }

    fn notify_data(
        &self,
        flags: windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS,
    ) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_UID,
            uFlags: flags,
            uCallbackMessage: TRAY_CALLBACK_MESSAGE,
            ..Default::default()
        }
    }

    fn tooltip(&self) -> String {
        match &self.snapshot {
            Some(snapshot) => match (snapshot.percentage, snapshot.charging) {
                (Some(percentage), Some(true)) => {
                    format!("{}: {percentage}% - charging", snapshot.device_name)
                }
                (Some(percentage), _) => format!("{}: {percentage}%", snapshot.device_name),
                (None, _) => format!("{}: unavailable", snapshot.device_name),
            },
            None => format!(
                "Razer Battery: {}",
                self.last_error
                    .as_ref()
                    .map(AppError::user_message)
                    .unwrap_or("unavailable")
            ),
        }
    }

    fn menu_status(&self) -> String {
        match &self.snapshot {
            Some(snapshot) => match (snapshot.percentage, snapshot.charging) {
                (Some(percentage), Some(true)) => format!("[ {percentage}% charging ]"),
                (Some(percentage), _) => format!("[ {percentage}% ]"),
                (None, _) => "[ unavailable ]".to_string(),
            },
            None => "[ unavailable ]".to_string(),
        }
    }

    fn icon_label(&self) -> String {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.percentage)
            .map(|percentage| percentage.to_string())
            .unwrap_or_else(|| "--".to_string())
    }

    fn icon_accent(&self) -> COLORREF {
        match self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.percentage)
        {
            Some(0..=19) => rgb(255, 72, 72),
            Some(_) => rgb(68, 214, 44),
            None => rgb(112, 112, 112),
        }
    }
}

unsafe fn run_tray() -> Result<(), AppError> {
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|_| AppError::Tray)?;
    let hinstance = HINSTANCE(instance.0);
    let class_name = wide_null(WINDOW_CLASS_NAME);
    let window_title = wide_null(WINDOW_TITLE);

    let window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: hinstance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };

    if unsafe { RegisterClassW(&window_class) } == 0 {
        return Err(AppError::Tray);
    }

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_title.as_ptr()),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance),
            None,
        )
    }
    .map_err(|_| AppError::Tray)?;

    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }

    let state = Box::new(TrayState::new(hwnd));
    let state_ptr = Box::into_raw(state);
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
    }

    let init_result = unsafe { (*state_ptr).initialize() };
    if init_result.is_err() {
        unsafe {
            let _ = Box::from_raw(state_ptr);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(hwnd);
        }
        return init_result;
    }

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        unsafe {
            DispatchMessageW(&message);
        }
    }

    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState };

    match message {
        WM_TIMER if wparam.0 == POLL_TIMER_ID => {
            if let Some(state) = unsafe { state_ptr.as_mut() } {
                unsafe {
                    state.poll();
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            if let Some(state) = unsafe { state_ptr.as_mut() } {
                match low_word(wparam.0) as usize {
                    MENU_REFRESH_ID => unsafe {
                        state.poll();
                    },
                    MENU_EXIT_ID => unsafe {
                        let _ = DestroyWindow(hwnd);
                    },
                    _ => {}
                }
            }
            LRESULT(0)
        }
        TRAY_CALLBACK_MESSAGE => {
            if let Some(state) = unsafe { state_ptr.as_mut() } {
                match low_word(lparam.0 as usize) {
                    WM_RBUTTONUP | WM_CONTEXTMENU => unsafe {
                        state.show_menu();
                    },
                    WM_LBUTTONDBLCLK => unsafe {
                        state.poll();
                    },
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                unsafe {
                    let mut state = Box::from_raw(state_ptr);
                    state.cleanup();
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
            }
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe fn append_menu_text(
    menu: HMENU,
    flags: windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS,
    id: usize,
    text: &str,
) {
    let wide = wide_null(text);
    let _ = unsafe { AppendMenuW(menu, flags, id, PCWSTR(wide.as_ptr())) };
}

unsafe fn create_tray_icon(label: String, accent: COLORREF) -> Result<HICON, AppError> {
    let screen_dc = unsafe { windows::Win32::Graphics::Gdi::GetDC(None) };
    if screen_dc.is_invalid() {
        return Err(AppError::Tray);
    }

    let color_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    let mask_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    if color_dc.is_invalid() || mask_dc.is_invalid() {
        unsafe {
            cleanup_dc(screen_dc, color_dc, mask_dc);
        }
        return Err(AppError::Tray);
    }

    let color_bitmap = unsafe { CreateCompatibleBitmap(screen_dc, ICON_SIZE, ICON_SIZE) };
    let mask_bitmap = unsafe { CreateBitmap(ICON_SIZE, ICON_SIZE, 1, 1, None) };
    if color_bitmap.is_invalid() || mask_bitmap.is_invalid() {
        unsafe {
            cleanup_bitmaps(color_bitmap, mask_bitmap);
            cleanup_dc(screen_dc, color_dc, mask_dc);
        }
        return Err(AppError::Tray);
    }

    unsafe {
        let previous_color_bitmap = SelectObject(color_dc, HGDIOBJ::from(color_bitmap));
        let previous_mask_bitmap = SelectObject(mask_dc, HGDIOBJ::from(mask_bitmap));
        draw_color_icon(color_dc, &label, accent);
        draw_mask_icon(mask_dc);
        SelectObject(color_dc, previous_color_bitmap);
        SelectObject(mask_dc, previous_mask_bitmap);
    }

    let icon_info = ICONINFO {
        fIcon: true.into(),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask_bitmap,
        hbmColor: color_bitmap,
    };

    let icon = unsafe { CreateIconIndirect(&icon_info) }.map_err(|_| AppError::Tray)?;

    unsafe {
        cleanup_bitmaps(color_bitmap, mask_bitmap);
        cleanup_dc(screen_dc, color_dc, mask_dc);
    }

    Ok(icon)
}

unsafe fn draw_color_icon(dc: HDC, label: &str, accent: COLORREF) {
    let background = unsafe { CreateSolidBrush(rgb(8, 8, 8)) };
    let pill = unsafe { CreateSolidBrush(rgb(18, 18, 18)) };
    let pen = unsafe { CreatePen(PS_SOLID, 4, accent) };
    let font_name = wide_null("Segoe UI");
    let font = unsafe {
        CreateFontW(
            -30,
            0,
            0,
            0,
            FW_BOLD.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            DEFAULT_PITCH.0 as u32,
            PCWSTR(font_name.as_ptr()),
        )
    };

    let full = RECT {
        left: 0,
        top: 0,
        right: ICON_SIZE,
        bottom: ICON_SIZE,
    };
    unsafe {
        FillRect(dc, &full, background);
        let previous_brush = SelectObject(dc, HGDIOBJ::from(pill));
        let previous_pen = SelectObject(dc, HGDIOBJ::from(pen));
        let _ = RoundRect(dc, 3, 12, ICON_SIZE - 3, ICON_SIZE - 12, 20, 20);
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, accent);

        let previous_font = if font.is_invalid() {
            HGDIOBJ::default()
        } else {
            SelectObject(dc, HGDIOBJ::from(font))
        };
        let mut text = wide_no_null(label);
        let mut text_rect = RECT {
            left: 4,
            top: 12,
            right: ICON_SIZE - 4,
            bottom: ICON_SIZE - 12,
        };
        DrawTextW(
            dc,
            &mut text,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        if !previous_font.is_invalid() {
            SelectObject(dc, previous_font);
        }
        SelectObject(dc, previous_pen);
        SelectObject(dc, previous_brush);
        cleanup_gdi(background, pill, pen, font);
    }
}

unsafe fn draw_mask_icon(dc: HDC) {
    let transparent = unsafe { CreateSolidBrush(rgb(255, 255, 255)) };
    let opaque = unsafe { CreateSolidBrush(rgb(0, 0, 0)) };
    let pen = unsafe { CreatePen(PS_SOLID, 1, rgb(0, 0, 0)) };
    let full = RECT {
        left: 0,
        top: 0,
        right: ICON_SIZE,
        bottom: ICON_SIZE,
    };

    unsafe {
        FillRect(dc, &full, transparent);
        let previous_brush = SelectObject(dc, HGDIOBJ::from(opaque));
        let previous_pen = SelectObject(dc, HGDIOBJ::from(pen));
        let _ = RoundRect(dc, 3, 12, ICON_SIZE - 3, ICON_SIZE - 12, 20, 20);
        SelectObject(dc, previous_pen);
        SelectObject(dc, previous_brush);
        cleanup_gdi(transparent, opaque, pen, HFONT::default());
    }
}

unsafe fn cleanup_dc(screen_dc: HDC, color_dc: HDC, mask_dc: HDC) {
    if !color_dc.is_invalid() {
        unsafe {
            let _ = DeleteDC(color_dc);
        }
    }
    if !mask_dc.is_invalid() {
        unsafe {
            let _ = DeleteDC(mask_dc);
        }
    }
    if !screen_dc.is_invalid() {
        unsafe {
            windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
        }
    }
}

unsafe fn cleanup_bitmaps(color_bitmap: HBITMAP, mask_bitmap: HBITMAP) {
    if !color_bitmap.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(color_bitmap));
        }
    }
    if !mask_bitmap.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(mask_bitmap));
        }
    }
}

unsafe fn cleanup_gdi(background: HBRUSH, pill: HBRUSH, pen: HPEN, font: HFONT) {
    if !background.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(background));
        }
    }
    if !pill.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(pill));
        }
    }
    if !pen.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(pen));
        }
    }
    if !font.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(font));
        }
    }
}

fn copy_wide_array<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);

    for (slot, code_unit) in target
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(text.encode_utf16())
    {
        *slot = code_unit;
    }
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn wide_no_null(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn low_word(value: usize) -> u32 {
    (value & 0xFFFF) as u32
}

fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    COLORREF(u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16))
}
