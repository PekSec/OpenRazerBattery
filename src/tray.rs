use std::mem::size_of;

use windows::{
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{
            ANTIALIASED_QUALITY, BeginPaint, CLIP_DEFAULT_PRECIS, CreateBitmap,
            CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
            DEFAULT_CHARSET, DEFAULT_PITCH, DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
            DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, DeleteDC, DeleteObject,
            DrawTextW, Ellipse, EndPaint, FW_BOLD, FillRect, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ,
            HPEN, InvalidateRect, LineTo, MoveToEx, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID,
            Polygon, RoundRect, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
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
                DispatchMessageW, GWLP_USERDATA, GetClientRect, GetCursorPos, GetMessageW,
                GetSystemMetrics, GetWindowLongPtrW, HICON, HMENU, ICONINFO, IDC_ARROW, IsWindow,
                KillTimer, LoadCursorW, MF_DISABLED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG,
                PostMessageW, PostQuitMessage, RegisterClassW, SC_CLOSE, SC_MINIMIZE, SM_CXSCREEN,
                SM_CYSCREEN, SW_HIDE, SW_RESTORE, SW_SHOW, SWP_NOZORDER, SetForegroundWindow,
                SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_BOTTOMALIGN,
                TPM_LEFTALIGN, TPM_RIGHTBUTTON, TrackPopupMenu, WINDOW_EX_STYLE, WM_CLOSE,
                WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDBLCLK, WM_NULL,
                WM_PAINT, WM_RBUTTONUP, WM_SYSCOMMAND, WM_TIMER, WM_USER, WNDCLASSW, WS_CAPTION,
                WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
            },
        },
    },
    core::PCWSTR,
};

use crate::{
    battery::{BatterySnapshot, BatteryStatus, probe_batteries},
    error::AppError,
};

const WINDOW_CLASS_NAME: &str = "razer-bat-tray-window";
const DETAILS_CLASS_NAME: &str = "razer-bat-details-window";
const WINDOW_TITLE: &str = "razer-bat";
const DETAILS_TITLE: &str = "Razer Battery";
const TRAY_UID: u32 = 1;
const TRAY_CALLBACK_MESSAGE: u32 = WM_USER + 1;
const POLL_TIMER_ID: usize = 1;
const MENU_REFRESH_ID: usize = 1001;
const MENU_EXIT_ID: usize = 1002;
const MENU_DETAILS_ID: usize = 1003;
const DEFAULT_POLL_MS: u32 = 60_000;
const SECOND_FAILURE_POLL_MS: u32 = 120_000;
const REPEATED_FAILURE_POLL_MS: u32 = 300_000;
const ICON_SIZE: i32 = 64;
const DETAILS_WIDTH: i32 = 430;
const DETAILS_MIN_HEIGHT: i32 = 250;
const DETAILS_MAX_HEIGHT: i32 = 620;

pub fn run() -> Result<(), AppError> {
    unsafe { run_tray() }
}

struct TrayState {
    hwnd: HWND,
    details_hwnd: HWND,
    icon: HICON,
    snapshots: Vec<BatterySnapshot>,
    failure_count: u8,
}

impl TrayState {
    fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            details_hwnd: HWND::default(),
            icon: HICON::default(),
            snapshots: Vec::new(),
            failure_count: 0,
        }
    }

    unsafe fn initialize(&mut self) -> Result<(), AppError> {
        self.icon = unsafe { create_tray_icon(self.tray_accent()) }?;

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
        self.snapshots = probe_batteries();

        if self
            .snapshots
            .iter()
            .any(|snapshot| snapshot.status == BatteryStatus::Ok)
        {
            self.failure_count = 0;
        } else {
            self.failure_count = self.failure_count.saturating_add(1);
        }

        unsafe {
            self.update_icon();
            self.invalidate_details();
            self.reset_timer();
        }
    }

    unsafe fn update_icon(&mut self) {
        let Ok(next_icon) = (unsafe { create_tray_icon(self.tray_accent()) }) else {
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
            append_menu_text(menu, MF_STRING, MENU_DETAILS_ID, "Details");
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

    unsafe fn show_details_window(&mut self) {
        let (x, y, height) = unsafe { self.details_window_position() };

        if unsafe { IsWindow(Some(self.details_hwnd)) }.as_bool() {
            unsafe {
                let _ = SetWindowPos(
                    self.details_hwnd,
                    None,
                    x,
                    y,
                    DETAILS_WIDTH,
                    height,
                    SWP_NOZORDER,
                );
                let _ = InvalidateRect(Some(self.details_hwnd), None, true);
                let _ = ShowWindow(self.details_hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(self.details_hwnd);
            }
            return;
        }

        let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
            return;
        };
        let hinstance = HINSTANCE(module.0);
        let class_name = wide_null(DETAILS_CLASS_NAME);
        let title = wide_null(DETAILS_TITLE);
        let state_ptr = self as *mut TrayState;

        let Ok(hwnd) = (unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                x,
                y,
                DETAILS_WIDTH,
                height,
                Some(self.hwnd),
                None,
                Some(hinstance),
                None,
            )
        }) else {
            return;
        };

        self.details_hwnd = hwnd;
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
    }

    unsafe fn invalidate_details(&self) {
        if unsafe { IsWindow(Some(self.details_hwnd)) }.as_bool() {
            unsafe {
                let _ = InvalidateRect(Some(self.details_hwnd), None, true);
            }
        }
    }

    unsafe fn details_window_position(&self) -> (i32, i32, i32) {
        let height = self.details_window_height();
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_err() {
            point.x = unsafe { GetSystemMetrics(SM_CXSCREEN) } - 24;
            point.y = unsafe { GetSystemMetrics(SM_CYSCREEN) } - 24;
        }

        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let x = (point.x - DETAILS_WIDTH + 24).clamp(0, (screen_width - DETAILS_WIDTH).max(0));
        let y = (point.y - height - 16).clamp(0, (screen_height - height).max(0));

        (x, y, height)
    }

    fn details_window_height(&self) -> i32 {
        let row_count = self.snapshots.len().max(1) as i32;
        (142 + row_count * 74).clamp(DETAILS_MIN_HEIGHT, DETAILS_MAX_HEIGHT)
    }

    unsafe fn cleanup(&mut self) {
        let _ = unsafe { KillTimer(Some(self.hwnd), POLL_TIMER_ID) };

        if unsafe { IsWindow(Some(self.details_hwnd)) }.as_bool() {
            let _ = unsafe { DestroyWindow(self.details_hwnd) };
            self.details_hwnd = HWND::default();
        }

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
        if self.snapshots.is_empty() {
            return "Razer Battery\r\nNo battery data available.".to_string();
        }

        let mut lines = Vec::with_capacity(self.snapshots.len() + 1);
        lines.push("Razer Battery".to_string());
        lines.extend(self.snapshots.iter().map(compact_snapshot_line));
        lines.join("\r\n")
    }

    fn menu_status(&self) -> String {
        match self.available_snapshot_count() {
            0 => "[ unavailable ]".to_string(),
            1 => "[ 1 device ]".to_string(),
            count => format!("[ {count} devices ]"),
        }
    }

    fn tray_accent(&self) -> COLORREF {
        if self.snapshots.iter().any(|snapshot| {
            snapshot.status == BatteryStatus::Ok
                && snapshot.charging != Some(true)
                && matches!(snapshot.percentage, Some(0..=19))
        }) {
            rgb(255, 72, 72)
        } else if self
            .snapshots
            .iter()
            .any(|snapshot| snapshot.status == BatteryStatus::Ok)
        {
            rgb(68, 214, 44)
        } else {
            rgb(88, 88, 88)
        }
    }

    fn available_snapshot_count(&self) -> usize {
        self.snapshots
            .iter()
            .filter(|snapshot| snapshot.status == BatteryStatus::Ok)
            .count()
    }
}

unsafe fn run_tray() -> Result<(), AppError> {
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|_| AppError::Tray)?;
    let hinstance = HINSTANCE(instance.0);
    let class_name = wide_null(WINDOW_CLASS_NAME);
    let details_class_name = wide_null(DETAILS_CLASS_NAME);
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

    let details_window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(details_window_proc),
        hInstance: hinstance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: PCWSTR(details_class_name.as_ptr()),
        ..Default::default()
    };

    if unsafe { RegisterClassW(&details_window_class) } == 0 {
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
    loop {
        let message_result = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        if message_result == -1 {
            return Err(AppError::Tray);
        }
        if message_result == 0 {
            break;
        }

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
                    MENU_DETAILS_ID => unsafe {
                        state.show_details_window();
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
                        state.show_details_window();
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

unsafe extern "system" fn details_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState };

    match message {
        WM_CLOSE => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_SYSCOMMAND if matches!(system_command(wparam.0), SC_MINIMIZE | SC_CLOSE) => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            let mut rect = RECT::default();
            let _ = unsafe { GetClientRect(hwnd, &mut rect) };

            if let Some(state) = unsafe { state_ptr.as_ref() } {
                unsafe {
                    draw_details_window(dc, &rect, state);
                }
            }

            unsafe {
                let _ = EndPaint(hwnd, &paint);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = unsafe { state_ptr.as_mut() } {
                if state.details_hwnd == hwnd {
                    state.details_hwnd = HWND::default();
                }
            }
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
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

unsafe fn create_tray_icon(accent: COLORREF) -> Result<HICON, AppError> {
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
        draw_color_icon(color_dc, accent);
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

    let icon = unsafe { CreateIconIndirect(&icon_info) }.map_err(|_| AppError::Tray);

    unsafe {
        cleanup_bitmaps(color_bitmap, mask_bitmap);
        cleanup_dc(screen_dc, color_dc, mask_dc);
    }

    icon
}

unsafe fn draw_color_icon(dc: HDC, accent: COLORREF) {
    let background = unsafe { CreateSolidBrush(rgb(0, 0, 0)) };
    let head = unsafe { CreateSolidBrush(accent) };
    let outline = unsafe { CreatePen(PS_SOLID, 2, rgb(176, 255, 164)) };
    let detail = unsafe { CreatePen(PS_SOLID, 3, rgb(5, 5, 5)) };
    let eye = unsafe { CreateSolidBrush(rgb(5, 5, 5)) };
    let full = RECT {
        left: 0,
        top: 0,
        right: ICON_SIZE,
        bottom: ICON_SIZE,
    };

    unsafe {
        FillRect(dc, &full, background);
        let previous_brush = SelectObject(dc, HGDIOBJ::from(head));
        let previous_pen = SelectObject(dc, HGDIOBJ::from(outline));
        let _ = Polygon(dc, &snake_head_points());

        SelectObject(dc, HGDIOBJ::from(detail));
        SelectObject(dc, HGDIOBJ::from(eye));
        let _ = Ellipse(dc, 21, 24, 28, 30);
        let _ = Ellipse(dc, 36, 24, 43, 30);
        let _ = Ellipse(dc, 28, 39, 31, 42);
        let _ = Ellipse(dc, 33, 39, 36, 42);

        SelectObject(dc, HGDIOBJ::from(detail));
        let _ = MoveToEx(dc, 32, 33, None);
        let _ = LineTo(dc, 32, 47);

        SelectObject(dc, previous_pen);
        SelectObject(dc, previous_brush);
        cleanup_gdi(background, head, outline, HFONT::default());
        cleanup_gdi(eye, HBRUSH::default(), detail, HFONT::default());
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
        let _ = Polygon(dc, &snake_head_points());
        SelectObject(dc, previous_pen);
        SelectObject(dc, previous_brush);
        cleanup_gdi(transparent, opaque, pen, HFONT::default());
    }
}

unsafe fn draw_details_window(dc: HDC, rect: &RECT, state: &TrayState) {
    let background = unsafe { CreateSolidBrush(rgb(5, 7, 5)) };
    let panel = unsafe { CreateSolidBrush(rgb(12, 16, 12)) };
    let panel_pen = unsafe { CreatePen(PS_SOLID, 1, rgb(38, 84, 34)) };
    let accent_pen = unsafe { CreatePen(PS_SOLID, 2, rgb(68, 214, 44)) };
    let title_font = unsafe { create_font(24, FW_BOLD.0 as i32) };
    let body_font = unsafe { create_font(15, 400) };
    let small_font = unsafe { create_font(12, 400) };

    unsafe {
        SetBkMode(dc, TRANSPARENT);
        FillRect(dc, rect, background);

        draw_round_rect(dc, 18, 18, rect.right - 18, 92, 18, panel, panel_pen);
        draw_small_snake_mark(dc, 34, 34);

        draw_text(
            dc,
            "Razer Battery",
            RECT {
                left: 84,
                top: 30,
                right: rect.right - 28,
                bottom: 58,
            },
            rgb(68, 214, 44),
            title_font,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        draw_text(
            dc,
            &details_summary(state),
            RECT {
                left: 85,
                top: 60,
                right: rect.right - 28,
                bottom: 82,
            },
            rgb(180, 196, 178),
            body_font,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        if state.snapshots.is_empty() {
            draw_empty_state(dc, rect, state, body_font, panel, panel_pen);
        } else {
            let available_height = rect.bottom - 128;
            let max_rows = (available_height / 74).max(1) as usize;
            let visible_rows = state.snapshots.len().min(max_rows);

            for (index, snapshot) in state.snapshots.iter().take(visible_rows).enumerate() {
                draw_snapshot_card(
                    dc,
                    rect,
                    snapshot,
                    112 + index as i32 * 74,
                    body_font,
                    small_font,
                    panel,
                    accent_pen,
                );
            }

            if state.snapshots.len() > visible_rows {
                draw_text(
                    dc,
                    &format!("+{} more devices", state.snapshots.len() - visible_rows),
                    RECT {
                        left: 28,
                        top: rect.bottom - 30,
                        right: rect.right - 28,
                        bottom: rect.bottom - 10,
                    },
                    rgb(180, 196, 178),
                    small_font,
                    DT_RIGHT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
                );
            }
        }

        cleanup_gdi(background, panel, panel_pen, title_font);
        cleanup_gdi(HBRUSH::default(), HBRUSH::default(), accent_pen, body_font);
        cleanup_gdi(
            HBRUSH::default(),
            HBRUSH::default(),
            HPEN::default(),
            small_font,
        );
    }
}

unsafe fn draw_empty_state(
    dc: HDC,
    rect: &RECT,
    state: &TrayState,
    font: HFONT,
    panel: HBRUSH,
    pen: HPEN,
) {
    unsafe {
        draw_round_rect(
            dc,
            22,
            112,
            rect.right - 22,
            rect.bottom - 24,
            16,
            panel,
            pen,
        );
        draw_text(
            dc,
            &empty_state_message(state),
            RECT {
                left: 40,
                top: 128,
                right: rect.right - 40,
                bottom: rect.bottom - 40,
            },
            rgb(180, 196, 178),
            font,
            DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
}

unsafe fn draw_snapshot_card(
    dc: HDC,
    rect: &RECT,
    snapshot: &BatterySnapshot,
    top: i32,
    body_font: HFONT,
    small_font: HFONT,
    panel: HBRUSH,
    pen: HPEN,
) {
    let card_left = 22;
    let card_right = rect.right - 22;
    let card_bottom = top + 62;
    let pill_color = status_color(snapshot);
    let pill = unsafe { CreateSolidBrush(pill_color) };
    let pill_pen = unsafe { CreatePen(PS_SOLID, 1, pill_color) };

    unsafe {
        draw_round_rect(dc, card_left, top, card_right, card_bottom, 14, panel, pen);

        draw_text(
            dc,
            &snapshot.device_name,
            RECT {
                left: card_left + 16,
                top: top + 9,
                right: card_right - 150,
                bottom: top + 33,
            },
            rgb(230, 244, 228),
            body_font,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        draw_text(
            dc,
            &format!("VID 0x{:04X}  PID 0x{:04X}", snapshot.vid, snapshot.pid),
            RECT {
                left: card_left + 16,
                top: top + 34,
                right: card_right - 150,
                bottom: top + 54,
            },
            rgb(125, 146, 122),
            small_font,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        draw_round_rect(
            dc,
            card_right - 136,
            top + 17,
            card_right - 16,
            top + 45,
            14,
            pill,
            pill_pen,
        );
        draw_text(
            dc,
            &status_label(snapshot),
            RECT {
                left: card_right - 132,
                top: top + 17,
                right: card_right - 20,
                bottom: top + 45,
            },
            status_text_color(snapshot),
            small_font,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );

        cleanup_gdi(pill, HBRUSH::default(), pill_pen, HFONT::default());
    }
}

unsafe fn draw_round_rect(
    dc: HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    radius: i32,
    brush: HBRUSH,
    pen: HPEN,
) {
    unsafe {
        let previous_brush = SelectObject(dc, HGDIOBJ::from(brush));
        let previous_pen = SelectObject(dc, HGDIOBJ::from(pen));
        let _ = RoundRect(dc, left, top, right, bottom, radius, radius);
        SelectObject(dc, previous_pen);
        SelectObject(dc, previous_brush);
    }
}

unsafe fn draw_small_snake_mark(dc: HDC, left: i32, top: i32) {
    let accent = unsafe { CreateSolidBrush(rgb(68, 214, 44)) };
    let outline = unsafe { CreatePen(PS_SOLID, 2, rgb(176, 255, 164)) };
    let detail = unsafe { CreatePen(PS_SOLID, 2, rgb(5, 7, 5)) };
    let eye = unsafe { CreateSolidBrush(rgb(5, 7, 5)) };
    let points = [
        POINT {
            x: left + 18,
            y: top,
        },
        POINT {
            x: left + 29,
            y: top + 4,
        },
        POINT {
            x: left + 36,
            y: top + 15,
        },
        POINT {
            x: left + 34,
            y: top + 27,
        },
        POINT {
            x: left + 26,
            y: top + 38,
        },
        POINT {
            x: left + 18,
            y: top + 34,
        },
        POINT {
            x: left + 10,
            y: top + 38,
        },
        POINT {
            x: left + 2,
            y: top + 27,
        },
        POINT {
            x: left,
            y: top + 15,
        },
        POINT {
            x: left + 7,
            y: top + 4,
        },
    ];

    unsafe {
        let previous_brush = SelectObject(dc, HGDIOBJ::from(accent));
        let previous_pen = SelectObject(dc, HGDIOBJ::from(outline));
        let _ = Polygon(dc, &points);
        SelectObject(dc, HGDIOBJ::from(detail));
        SelectObject(dc, HGDIOBJ::from(eye));
        let _ = Ellipse(dc, left + 11, top + 15, left + 16, top + 19);
        let _ = Ellipse(dc, left + 20, top + 15, left + 25, top + 19);
        SelectObject(dc, HGDIOBJ::from(detail));
        let _ = MoveToEx(dc, left + 18, top + 22, None);
        let _ = LineTo(dc, left + 18, top + 31);
        SelectObject(dc, previous_pen);
        SelectObject(dc, previous_brush);
        cleanup_gdi(accent, eye, outline, HFONT::default());
        cleanup_gdi(
            HBRUSH::default(),
            HBRUSH::default(),
            detail,
            HFONT::default(),
        );
    }
}

unsafe fn draw_text(
    dc: HDC,
    text: &str,
    mut rect: RECT,
    color: COLORREF,
    font: HFONT,
    format: DRAW_TEXT_FORMAT,
) {
    let mut wide = wide_no_null(text);
    unsafe {
        SetTextColor(dc, color);
        let previous_font = if font.is_invalid() {
            HGDIOBJ::default()
        } else {
            SelectObject(dc, HGDIOBJ::from(font))
        };
        DrawTextW(dc, &mut wide, &mut rect, format);
        if !previous_font.is_invalid() {
            SelectObject(dc, previous_font);
        }
    }
}

unsafe fn create_font(size: i32, weight: i32) -> HFONT {
    let font_name = wide_null("Segoe UI");
    unsafe {
        CreateFontW(
            -size,
            0,
            0,
            0,
            weight,
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

fn compact_snapshot_line(snapshot: &BatterySnapshot) -> String {
    let name = snapshot
        .device_name
        .strip_prefix("Razer ")
        .unwrap_or(&snapshot.device_name);

    if snapshot.status != BatteryStatus::Ok {
        return format!("{name}: {}", snapshot.status.user_message());
    }

    match (snapshot.percentage, snapshot.charging) {
        (Some(percentage), Some(true)) => format!("{name}: {percentage}% charging"),
        (Some(percentage), _) => format!("{name}: {percentage}%"),
        (None, _) => format!("{name}: unavailable"),
    }
}

fn details_summary(state: &TrayState) -> String {
    match state.available_snapshot_count() {
        0 => empty_state_message(state),
        1 => "1 OpenRazer battery device".to_string(),
        count => format!("{count} OpenRazer battery devices"),
    }
}

fn status_label(snapshot: &BatterySnapshot) -> String {
    if snapshot.status != BatteryStatus::Ok {
        return snapshot.status.short_label().to_string();
    }

    match (snapshot.percentage, snapshot.charging) {
        (Some(percentage), Some(true)) => format!("{percentage}% charging"),
        (Some(percentage), _) => format!("{percentage}%"),
        (None, _) => "unavailable".to_string(),
    }
}

fn status_color(snapshot: &BatterySnapshot) -> COLORREF {
    if snapshot.status != BatteryStatus::Ok {
        return match snapshot.status {
            BatteryStatus::DeviceBusy => rgb(238, 184, 68),
            BatteryStatus::AccessDenied
            | BatteryStatus::ProtocolError
            | BatteryStatus::TransportError => rgb(180, 64, 64),
            BatteryStatus::DeviceNotFound | BatteryStatus::UnsupportedDevice => rgb(88, 88, 88),
            BatteryStatus::Ok => rgb(68, 214, 44),
        };
    }

    match (snapshot.percentage, snapshot.charging) {
        (_, Some(true)) => rgb(68, 214, 44),
        (Some(0..=19), _) => rgb(255, 72, 72),
        (Some(_), _) => rgb(68, 214, 44),
        (None, _) => rgb(88, 88, 88),
    }
}

fn status_text_color(snapshot: &BatterySnapshot) -> COLORREF {
    if snapshot.status != BatteryStatus::Ok {
        return rgb(255, 255, 255);
    }

    match snapshot.percentage {
        Some(0..=19) if snapshot.charging != Some(true) => rgb(255, 255, 255),
        Some(_) => rgb(5, 7, 5),
        None => rgb(230, 244, 228),
    }
}

fn empty_state_message(state: &TrayState) -> String {
    state
        .snapshots
        .first()
        .map(|snapshot| snapshot.status.user_message())
        .unwrap_or("No battery data available.")
        .to_string()
}

fn snake_head_points() -> [POINT; 12] {
    [
        POINT { x: 32, y: 5 },
        POINT { x: 44, y: 10 },
        POINT { x: 54, y: 24 },
        POINT { x: 56, y: 39 },
        POINT { x: 48, y: 53 },
        POINT { x: 37, y: 59 },
        POINT { x: 32, y: 54 },
        POINT { x: 27, y: 59 },
        POINT { x: 16, y: 53 },
        POINT { x: 8, y: 39 },
        POINT { x: 10, y: 24 },
        POINT { x: 20, y: 10 },
    ]
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

fn system_command(value: usize) -> u32 {
    (value & 0xFFF0) as u32
}

fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    COLORREF(u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16))
}
