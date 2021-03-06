use std::convert::TryInto;

use winapi::{
    shared::{basetsd::LONG_PTR, windef::HWND},
    um::winuser::{
        GetParent, GetWindow, GetWindowLongPtrW, SetForegroundWindow, SetLayeredWindowAttributes,
        SetWindowLongPtrW, GWL_EXSTYLE, GW_HWNDNEXT, LWA_ALPHA,
    },
};
use winit::{platform::windows::WindowExtWindows, window::Window};

const WS_EX_TRANSPARENT: isize = 0x20;
const WS_EX_LAYERED: isize = 0x80000;

pub fn make_window_overlay(window: &Window) {
    window.set_always_on_top(true);

    let hwnd = window.hwnd() as HWND;

    let window_styles: isize = match unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } {
        0 => panic!("GetWindowLongPtrW returned 0"),
        ptr => ptr.try_into().unwrap(),
    };
    let window_styles: LONG_PTR = window_styles | WS_EX_TRANSPARENT | WS_EX_LAYERED;

    if unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, window_styles.try_into().unwrap()) } == 0 {
        panic!("SetWindowLongPtr returned 0");
    }

    unsafe { make_last_active_window_active(hwnd) };
}

pub fn make_window_overlay_clickable(window: &Window) {
    window.set_always_on_top(false);

    let hwnd = window.hwnd().cast() as HWND;

    let window_styles: isize = match unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } {
        0 => panic!("GetWindowLongPtrW returned 0"),
        ptr => ptr.try_into().unwrap(),
    };
    let window_styles = window_styles & !WS_EX_TRANSPARENT | WS_EX_LAYERED;

    if unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, window_styles.try_into().unwrap()) } == 0 {
        panic!("SetWindowLongPtr returned 0");
    }
}

unsafe fn make_last_active_window_active(hwnd: HWND) {
    let mut last_window = GetWindow(hwnd, GW_HWNDNEXT);
    if last_window.is_null() {
        panic!("GetWindow returned 0");
    }

    let get_parent = |window| {
        let parent = GetParent(window);
        if parent.is_null() {
            None
        } else {
            Some(parent)
        }
    };

    while let Some(parent) = get_parent(last_window) {
        last_window = parent;
    }

    SetForegroundWindow(last_window);
}

pub fn set_window_overlay_opacity(window: &Window, opacity: u8) {
    let hwnd = window.hwnd() as HWND;

    if unsafe { SetLayeredWindowAttributes(hwnd, 0, opacity, LWA_ALPHA) } == 0 {
        panic!("SetLayeredWindowAttributes returned 0");
    }
}
