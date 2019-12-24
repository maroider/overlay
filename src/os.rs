use winit::window::Window;

pub fn make_window_overlay(window: &Window) {
    #[cfg(windows)]
    windows::make_window_overlay(window);
}

pub fn make_window_overlay_clickthrough(window: &Window) {
    #[cfg(windows)]
    windows::make_window_overlay(window);
}

pub fn make_window_overlay_clickable(window: &Window) {
    #[cfg(windows)]
    windows::make_window_overlay_clickable(window);
}

#[cfg(windows)]
mod windows {
    use winapi::{
        shared::windef::HWND__,
        um::winuser::{
            GetParent, GetWindow, GetWindowLongPtrW, SetForegroundWindow,
            SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, GW_HWNDNEXT, LWA_ALPHA,
        },
    };
    use winit::{platform::windows::WindowExtWindows, window::Window};

    const WS_EX_TRANSPARENT: isize = 0x20;
    const WS_EX_LAYERED: isize = 0x80000;

    pub fn make_window_overlay(window: &Window) {
        window.set_always_on_top(true);

        let hwnd = window.hwnd() as *mut HWND__;

        let window_styles: isize = match unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } {
            0 => panic!("GetWindowLongPtrW returned 0"),
            ptr => ptr,
        };

        if unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                window_styles | WS_EX_TRANSPARENT | WS_EX_LAYERED,
            )
        } == 0
        {
            panic!("SetWindowLongPtr returned 0");
        }

        if unsafe { SetLayeredWindowAttributes(hwnd, 0, 220, LWA_ALPHA) } == 0 {
            panic!("SetLayeredWindowAttributes returned 0");
        }

        unsafe { make_last_active_window_active(hwnd) };
    }

    pub fn make_window_overlay_clickable(window: &Window) {
        window.set_always_on_top(false);

        let hwnd = window.hwnd() as *mut HWND__;

        let window_styles: isize = match unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } {
            0 => panic!("GetWindowLongPtrW returned 0"),
            ptr => ptr,
        };

        if unsafe {
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                window_styles & !WS_EX_TRANSPARENT | WS_EX_LAYERED,
            )
        } == 0
        {
            panic!("SetWindowLongPtr returned 0");
        }

        if unsafe { SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA) } == 0 {
            panic!("SetLayeredWindowAttributes returned 0");
        }
    }

    unsafe fn make_last_active_window_active(hwnd: *mut HWND__) {
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
}
