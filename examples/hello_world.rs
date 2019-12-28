//! This example is a lightly modified version of `imgui-wgpu`'s `hello_world` example.

use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support;
use overlay::OverlayBuilder;
use std::time::Instant;
use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    env_logger::init();

    // Set up window and GPU
    let event_loop = EventLoop::new();
    let (mut overlay, mut size, surface, hidpi_factor) = {
        let overlay = OverlayBuilder::new()
            .with_inactive_opacity(220)
            .build(&event_loop)
            .unwrap();
        let window = overlay.window();
        let hidpi_factor = window.hidpi_factor();
        let size = window.inner_size().to_physical(hidpi_factor);

        let surface = wgpu::Surface::create(window);

        (overlay, size, surface, hidpi_factor)
    };

    let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        backends: wgpu::BackendBit::PRIMARY,
    })
    .unwrap();

    let (mut device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });

    // Set up swap chain
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::NoVsync,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        overlay.window(),
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);

    //
    // Set up dear imgui wgpu renderer
    //
    let clear_color = wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    let mut renderer = Renderer::new(
        &mut imgui,
        &device,
        &mut queue,
        sc_desc.format,
        Some(clear_color),
    );

    let mut last_frame = Instant::now();
    let mut demo_open = true;

    overlay.init();

    let mut overlay_toggle_lockout = false;

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                size = overlay.window().inner_size().to_physical(hidpi_factor);

                sc_desc = wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    width: size.width as u32,
                    height: size.height as u32,
                    present_mode: wgpu::PresentMode::NoVsync,
                };

                swap_chain = device.create_swap_chain(&surface, &sc_desc);
            }
            Event::DeviceEvent {
                event:
                    DeviceEvent::Key(KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Pressed,
                        modifiers: ModifiersState { shift: true, .. },
                        ..
                    }),
                ..
            } => {
                if !overlay_toggle_lockout {
                    overlay.toggle();
                    overlay_toggle_lockout = true;
                }
            }
            Event::DeviceEvent {
                event:
                    DeviceEvent::Key(KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Released,
                        ..
                    }),
                ..
            } => {
                if overlay_toggle_lockout {
                    overlay_toggle_lockout = false;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::EventsCleared => {
                let now = Instant::now();
                let delta = now - last_frame;
                let delta_s = delta.as_micros();
                last_frame = now;

                let frame = swap_chain.get_next_texture();
                platform
                    .prepare_frame(imgui.io_mut(), overlay.window())
                    .expect("Failed to prepare frame");
                let ui = imgui.frame();

                {
                    let window = imgui::Window::new(im_str!("Hello world"));
                    window
                        .size([300.0, 100.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.text(im_str!("Hello world!"));
                            ui.text(im_str!("This...is...imgui-rs on WGPU on an overlay!"));
                            ui.separator();
                            let mouse_pos = ui.io().mouse_pos;
                            ui.text(im_str!(
                                "Mouse Position: ({:.1},{:.1})",
                                mouse_pos[0],
                                mouse_pos[1]
                            ));
                        });

                    let window = imgui::Window::new(im_str!("Hello too"));
                    window
                        .size([400.0, 200.0], Condition::FirstUseEver)
                        .position([400.0, 200.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.text(im_str!("Frametime: {}us", delta_s));
                        });

                    ui.show_demo_window(&mut demo_open);
                }

                let mut encoder: wgpu::CommandEncoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

                platform.prepare_render(&ui, overlay.window());
                renderer
                    .render(ui, &mut device, &mut encoder, &frame.view)
                    .expect("Rendering failed");

                queue.submit(&[encoder.finish()]);
            }
            _ => (),
        }

        platform.handle_event(imgui.io_mut(), overlay.window(), &event);
    });
}
