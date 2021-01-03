use std::{error::Error, fmt};

use winit::{
    dpi::LogicalPosition,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod platform_impl;

// TODO: Provide a method which lets you chose which monitor the overlay spawns on top of.

/// Object that lets you build overlays.
pub struct OverlayBuilder {
    window_builder: WindowBuilder,
    active_opacity: Option<u8>,
    inactive_opacity: Option<u8>,
}

impl OverlayBuilder {
    pub fn new() -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let window_builder = WindowBuilder::new().with_title(&format!("overlay {}", version));

        Self {
            window_builder,
            active_opacity: None,
            inactive_opacity: None,
        }
    }

    /// Set the overlay's window title.
    pub fn with_title<T: Into<String>>(self, title: T) -> Self {
        Self {
            window_builder: self.window_builder.with_title(title),
            ..self
        }
    }

    /// Set the opacity of the overlay when it is active.
    pub fn with_active_opacity(self, opacity: u8) -> Self {
        Self {
            active_opacity: Some(opacity),
            ..self
        }
    }

    /// Set the opacity of the overlay when it is not active.
    pub fn with_inactive_opacity(self, opacity: u8) -> Self {
        Self {
            inactive_opacity: Some(opacity),
            ..self
        }
    }

    /// Create the overlay window. The overlay will be inactive upon creation.
    pub fn build<T: 'static>(
        self,
        event_loop: &EventLoop<T>,
    ) -> Result<Overlay, OverlayCreationError> {
        let window = self
            .window_builder
            .with_transparent(true)
            .with_decorations(false)
            .build(&event_loop)?;

        platform_impl::make_window_overlay(&window);
        platform_impl::set_window_overlay_opacity(&window, self.inactive_opacity.unwrap_or(0));

        window.set_outer_position(LogicalPosition { x: 0.0, y: 0.0 });
        window.set_inner_size(window.current_monitor().unwrap().size());

        Ok(Overlay::new(
            window,
            self.active_opacity.unwrap_or(255),
            self.inactive_opacity.unwrap_or(0),
        ))
    }
}

/// An overlay.
pub struct Overlay {
    window: Window,
    active: bool,
    active_opacity: u8,
    inactive_opacity: u8,
}

impl Overlay {
    fn new(window: Window, active_opacity: u8, inactive_opacity: u8) -> Self {
        Self {
            window,
            active: false,
            active_opacity,
            inactive_opacity,
        }
    }

    /// Toggle the overlay.
    pub fn toggle(&mut self) {
        if !self.active {
            self.activate();
        } else {
            self.deactivate();
        }
    }

    /// Make the overlay clickable.
    pub fn activate(&mut self) {
        if !self.active {
            self.active = true;
            platform_impl::make_window_overlay_clickable(&self.window);
            platform_impl::set_window_overlay_opacity(&self.window, self.active_opacity);
        }
    }

    /// Make the overlay transparent to inputs.
    pub fn deactivate(&mut self) {
        if self.active {
            self.active = false;
            platform_impl::make_window_overlay(&self.window);
            platform_impl::set_window_overlay_opacity(&self.window, self.inactive_opacity);
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the underlying window.
    ///
    /// # Remarks
    ///
    /// Be careful when manipulating the window by hand. You may inadvertantly
    /// leave the overlay in an invalid or unexpected state.
    pub fn window(&self) -> &Window {
        &self.window
    }
}

/// For when you can't give away ownership of the window.
pub struct BorrowedOverlay {
    active: bool,
    pub active_opacity: u8,
    pub inactive_opacity: u8,
}

impl BorrowedOverlay {
    pub fn new(active: bool, active_opacity: u8, inactive_opacity: u8) -> Self {
        Self {
            active,
            active_opacity,
            inactive_opacity,
        }
    }

    pub fn toggle(&mut self, window: &Window) {
        if !self.active {
            self.activate(window)
        } else {
            self.deactivate(window)
        }
    }

    /// Make the overlay clickable.
    pub fn activate(&mut self, window: &Window) {
        if !self.active {
            self.active = true;
            platform_impl::make_window_overlay_clickable(&window);
            platform_impl::set_window_overlay_opacity(&window, self.active_opacity);
        }
    }

    /// Make the overlay transparent to inputs.
    pub fn deactivate(&mut self, window: &Window) {
        if self.active {
            self.active = false;
            platform_impl::make_window_overlay(window);
            platform_impl::set_window_overlay_opacity(window, self.inactive_opacity);
        }
    }

    /// Returns true if the overlay can be interacted with through mouse clicks.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

#[derive(Debug)]
pub enum OverlayCreationError {
    Winit(winit::error::OsError),
}

impl fmt::Display for OverlayCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Winit(err) => write!(f, "Could not initialize the overlay: '{}'", err),
        }
    }
}

impl Error for OverlayCreationError {}

impl From<winit::error::OsError> for OverlayCreationError {
    fn from(from: winit::error::OsError) -> Self {
        Self::Winit(from)
    }
}
