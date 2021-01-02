use winit::{
    dpi::{LogicalPosition, Position, Size},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

mod platform_impl;

/// Object that lets you build overlays.
pub struct OverlayBuilder {
    title: Option<String>,
    active_opacity: Option<u8>,
    inactive_opacity: Option<u8>,
    position: Option<Position>,
    size: Option<Size>,
}

impl OverlayBuilder {
    pub fn new() -> Self {
        Self {
            title: None,
            active_opacity: None,
            inactive_opacity: None,
            position: None,
            size: None,
        }
    }

    /// Set the overlay's window title.
    pub fn with_title<T: Into<String>>(self, title: T) -> Self {
        Self {
            title: Some(title.into()),
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

    /// Set the overlay's position.
    pub fn with_position<P: Into<Position>>(self, position: P) -> Self {
        Self {
            position: Some(position.into()),
            ..self
        }
    }

    /// Set the overlay's size
    pub fn with_size<S: Into<Size>>(self, size: S) -> Self {
        Self {
            size: Some(size.into()),
            ..self
        }
    }

    /// Create the overlay window. The overlay will be inactive upon creation.
    pub fn build<T: 'static>(
        self,
        event_loop: &EventLoop<T>,
    ) -> Result<Overlay, winit::error::OsError> {
        let window = WindowBuilder::new()
            .with_title(
                self.title
                    .as_deref()
                    .unwrap_or_else(|| concat!("overlay ", env!("CARGO_PKG_VERSION"))),
            )
            .with_transparent(true)
            .with_decorations(false)
            .build(&event_loop)?;

        Ok(self.build_from_window(window))
    }

    /// Turn an already created window into an overlay window.
    ///
    /// # Remarks
    ///
    /// The window must be constructed with transparency enabled for opacity to have an effect.
    pub fn build_from_window(self, window: Window) -> Overlay {
        if let Some(title) = &self.title {
            window.set_title(title);
        }

        window.set_decorations(false);

        platform_impl::make_window_overlay(&window);
        platform_impl::set_window_overlay_opacity(&window, self.inactive_opacity.unwrap_or(0));

        window.set_outer_position(
            self.position
                .unwrap_or(LogicalPosition { x: 0.0, y: 0.0 }.into()),
        );
        window.set_inner_size(
            self.size
                .unwrap_or(window.current_monitor().unwrap().size().into()),
        );

        Overlay::new(
            window,
            self.active_opacity.unwrap_or(255),
            self.inactive_opacity.unwrap_or(0),
        )
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
            self.activate()
        } else {
            self.deactivate()
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
