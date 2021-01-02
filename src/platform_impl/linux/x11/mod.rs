use std::convert::TryInto;

use winit::{platform::unix::WindowExtUnix, window::Window};
use xcb::{
    ffi::xcb_connection_t,
    shape,
    xproto::{self, Rectangle},
    Connection,
};

pub fn make_window_overlay(window: &Window, opacity: u8) {
    window.set_always_on_top(true);

    let (xcb_connection, xcb_window) = winit_to_xcb(window);

    shape::rectangles(
        &xcb_connection,
        shape::SO_SET as u8,
        shape::SK_INPUT as u8,
        xproto::CLIP_ORDERING_YX_BANDED as u8,
        xcb_window,
        0,
        0,
        &[Rectangle::new(0, 0, 0, 0)],
    );

    set_window_overlay_opacity(window, opacity);

    xproto::set_input_focus(
        &xcb_connection,
        xproto::INPUT_FOCUS_PARENT as u8,
        xproto::INPUT_FOCUS_POINTER_ROOT,
        xproto::TIME_CURRENT_TIME,
    );
}

pub fn make_window_overlay_clickable(window: &Window, opacity: u8) {
    let (xcb_connection, xcb_window) = winit_to_xcb(window);

    shape::mask(
        &xcb_connection,
        shape::SO_SET as u8,
        shape::SK_INPUT as u8,
        xcb_window,
        0,
        0,
        xproto::PIXMAP_NONE,
    );

    set_window_overlay_opacity(window, opacity);
}

pub fn set_window_overlay_opacity(window: &Window, opacity: u8) {
    let (xcb_connection, xcb_window) = winit_to_xcb(window);

    if opacity == 255 {
        let cookie = xproto::intern_atom(&xcb_connection, false, "_NET_WM_WINDOW_OPACITY");

        if let Ok(reply) = cookie.get_reply() {
            let property = unsafe { (*(reply.ptr)).atom };
            xproto::delete_property(&xcb_connection, xcb_window, property);
        }
    } else {
        let opacity = opacity as f32 / 256.0;
        let opacity = opacity * std::u32::MAX as f32;
        let opacity = opacity as u32;
        let cookie = xproto::intern_atom(&xcb_connection, false, "_NET_WM_WINDOW_OPACITY");

        if let Ok(reply) = cookie.get_reply() {
            let property = unsafe { (*(reply.ptr)).atom };
            xproto::change_property(
                &xcb_connection,
                xproto::PROP_MODE_REPLACE as u8,
                xcb_window,
                property,
                xproto::ATOM_CARDINAL,
                32,
                &[opacity],
            );
        }
    }
}

fn winit_to_xcb(window: &Window) -> (std::mem::ManuallyDrop<Connection>, u32) {
    let xlib_window = window.xlib_window().expect("xlib not used");
    let xcb_window = xlib_window
        .try_into()
        .expect("overflowed casting XID from u64 to u32");
    let xcb_connection = window.xcb_connection().expect("xlib not used") as *mut xcb_connection_t;
    let xcb_connection = unsafe { Connection::from_raw_conn(xcb_connection) };

    // Wrap Connection with ManuallyDrop not to call destructor of Connection,
    // which disconnect the Connection with X server.
    // Because we use a connection borrowed from winit,
    // it makes winit unable to innteract with the server.
    let xcb_connection = std::mem::ManuallyDrop::new(xcb_connection);

    (xcb_connection, xcb_window)
}
