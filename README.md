# Overlays

Game overlays are usually created in one of two ways:

- by injecting code into the game and hooking rendering API functions (OpenGL, Direct3D, Vulkan, etc.)
- by creating a specially crafted window

This crate uses the second technique, and I'm working on a yet-to-be-announced crate utilizing the first technique.

There is also (potentially) a third way to draw an overlay which involves messing with the compositor, but I haven't really looked into that.

## Usage

```Rust
use overlay::OverlayBuilder;
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new();
    let mut overlay = OverlayBuilder::new().build();

    // Initialize renderer

    event_loop.run(..);
}
```

To toggle the overlay, use `Overlay::toggle(..)`.

## Limitiations

- The game has to be in either Windowed or Windowed Borderless.
- Currently only works on Windows and X11.
- There's currenty no way to specify which monitor the overlay spawns on.
- "Hugging" the target window isn't implemented yet ([#2]).
- (Windows) The window that the overlay is based on will show as an icon in the task bar.
- (Windows) The window that the overlay is based on behaves like a window in other inconvenient ways.

## How does it work?

The Windows implementation is based on [this CodeProject article from 2007](https://www.codeproject.com/Articles/12877/Transparent-Click-Through-Forms).

[#2]: https://github.com/maroider/overlay/issues/2
