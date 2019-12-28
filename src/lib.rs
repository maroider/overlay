use std::{error::Error, fmt};

use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod os;

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
            .with_visible(false)
            .build(&event_loop)?;

        os::make_window_overlay(&window);

        let hidpi_factor = window.hidpi_factor();
        window.set_outer_position(LogicalPosition { x: 0.0, y: 0.0 });
        window.set_inner_size(window.current_monitor().size().to_logical(hidpi_factor));

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
    visible: bool,
    active: bool,
    active_opacity: u8,
    inactive_opacity: u8,
}

impl Overlay {
    fn new(window: Window, active_opacity: u8, inactive_opacity: u8) -> Self {
        Self {
            window,
            visible: false,
            active: false,
            active_opacity,
            inactive_opacity,
        }
    }

    pub fn size(&self) -> PhysicalSize {
        self.window
            .inner_size()
            .to_physical(self.window.hidpi_factor())
    }

    pub fn toggle(&mut self) {
        if self.active {
            os::make_window_overlay_clickthrough(&self.window, self.inactive_opacity);
        } else {
            if !self.visible {
                self.window.set_visible(true);
                self.visible = true;
            }
            os::make_window_overlay_clickable(&self.window, self.active_opacity);
        }
        self.active = !self.active;
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
