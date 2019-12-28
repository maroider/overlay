# Overlays

Game overlays are usually created by hooking into the game's process. That kind of technique is beyond my current skillset. Luckilly, I've managed to find a technique that works well enough for me by using my search engine of choice.

## Usage

```Rust
use overlay::OverlayBuilder;
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new();
    let mut overlay = OverlayBuilder::new().build();

    event_loop.run(..);
}
```

To toggle the overlay, use `Overlay::toggle(..)`.

## Limitiations
 * The game has to be in either Windowed or Windowed Borderless.
 * Currently only works on Windows.
 * There's currenty no way to specify which monitor the overlay spawns on.
 * The window that the overlay is based on will show as an icon in the task bar.
 * The window that the overlay is based on behaves like a window in other inconvenient ways.

## How does it work?

You can see how it works by looking at `src/os.rs` and [reading this CodeProject article from 2007](https://www.codeproject.com/Articles/12877/Transparent-Click-Through-Forms). Some bits are (presumably) there to work around the way `winit` sets things up.
