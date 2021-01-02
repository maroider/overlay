//! This example is a lightly modified version of `imgui-wgpu`'s `hello_world` example.

use std::time::Instant;

use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig};
use overlay::OverlayBuilder;
use pollster::block_on;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    // Set up window and GPU
    let event_loop = EventLoop::new();

    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    let (mut overlay, size, surface) = {
        let overlay = OverlayBuilder::new()
            .with_inactive_opacity(220)
            .build(&event_loop)
            .unwrap();
        let window = overlay.window();
        let size = window.inner_size();

        let surface = unsafe { instance.create_surface(window) };

        (overlay, size, surface)
    };

    let scale_factor = overlay.window().scale_factor();

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
    }))
    .unwrap();

    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();

    // Set up swap chain
    let sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Mailbox,
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

    let font_size = (13.0 * scale_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / scale_factor) as f32;

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
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
    };

    let renderer_config = RendererConfig {
        texture_format: sc_desc.format,
        ..Default::default()
    };

    let mut renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

    let mut last_frame = Instant::now();
    let mut demo_open = true;

    let mut last_cursor = None;

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
                let size = overlay.window().inner_size();

                let sc_desc = wgpu::SwapChainDescriptor {
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    width: size.width as u32,
                    height: size.height as u32,
                    present_mode: wgpu::PresentMode::Mailbox,
                };

                swap_chain = device.create_swap_chain(&surface, &sc_desc);
            }
            #[allow(deprecated)]
            Event::DeviceEvent {
                event:
                    DeviceEvent::Key(KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Pressed,
                        modifiers,
                        ..
                    }),
                ..
            } if modifiers.shift() => {
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
            Event::MainEventsCleared => overlay.window().request_redraw(),
            Event::RedrawEventsCleared => {
                let delta_s = last_frame.elapsed();
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;

                let frame = match swap_chain.get_current_frame() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("dropped frame: {:?}", e);
                        return;
                    }
                };
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
                            ui.text(im_str!("This...is...imgui-rs on WGPU!"));
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
                            ui.text(im_str!("Frametime: {:?}", delta_s));
                        });

                    ui.show_demo_window(&mut demo_open);
                }

                let mut encoder: wgpu::CommandEncoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                if last_cursor != Some(ui.mouse_cursor()) {
                    last_cursor = Some(ui.mouse_cursor());
                    platform.prepare_render(&ui, overlay.window());
                }

                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

                renderer
                    .render(ui.render(), &queue, &device, &mut rpass)
                    .expect("Rendering failed");

                drop(rpass);

                queue.submit(Some(encoder.finish()));
            }
            _ => (),
        }

        platform.handle_event(imgui.io_mut(), overlay.window(), &event);
    });
}
