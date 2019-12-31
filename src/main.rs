use std::{ops, ptr};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use winapi::{
    shared::{
        dxgi::{IDXGISwapChain, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD},
        dxgiformat::DXGI_FORMAT_R8G8B8A8_UNORM,
        dxgitype::DXGI_USAGE_RENDER_TARGET_OUTPUT,
        minwindef::{BOOL, HINSTANCE},
        windef::HWND,
        winerror::{FAILED, SUCCEEDED},
    },
    um::{
        d3d11::{
            D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext,
            ID3D11RenderTargetView, ID3D11Resource, ID3D11Texture2D, D3D11_SDK_VERSION,
            D3D11_VIEWPORT,
        },
        d3dcommon::{
            D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_REFERENCE,
            D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_10_0,
            D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_9_3,
        },
        // debugapi::OutputDebugStringW,
    },
    Interface,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let (event_loop, window, width, height) = {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_visible(false)
            .build(&event_loop)
            .unwrap();

        let dpi_factor = window.hidpi_factor();
        let (width, height) = window.inner_size().to_physical(dpi_factor).into();

        (event_loop, window, width, height)
    };

    let renderer = D3D11Renderer::create(&window, width, height).unwrap();

    window.set_visible(true);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::EventsCleared => renderer.render(),
        _ => {}
    });
}

#[allow(dead_code)]
struct D3D11Renderer {
    hwnd: HWND,
    hinstance: HINSTANCE,

    device: *const ID3D11Device,
    device_context: *const ID3D11DeviceContext,
    swap_chain: *const IDXGISwapChain,
    render_target_view: *mut ID3D11RenderTargetView,
    driver_type: D3D_DRIVER_TYPE,
    feature_level: D3D_FEATURE_LEVEL,
    viewport: D3D11_VIEWPORT,
}

impl D3D11Renderer {
    fn create(
        window: &impl HasRawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, FailedToCreateSwapChain> {
        let (hwnd, hinstance) = {
            match window.raw_window_handle() {
                RawWindowHandle::Windows(windows_handle) => (
                    windows_handle.hwnd as HWND,
                    windows_handle.hinstance as HINSTANCE,
                ),
                _ => panic!(""),
            }
        };

        let create_device_flags = 0;

        let driver_types = [
            D3D_DRIVER_TYPE_HARDWARE,
            D3D_DRIVER_TYPE_WARP,
            D3D_DRIVER_TYPE_REFERENCE,
        ];

        let feature_levels = [
            D3D_FEATURE_LEVEL_11_0,
            D3D_FEATURE_LEVEL_10_1,
            D3D_FEATURE_LEVEL_10_0,
            D3D_FEATURE_LEVEL_9_3,
        ];

        let mut sc_desc = DXGI_SWAP_CHAIN_DESC::default();
        sc_desc.BufferCount = 1; // Double buffering
        sc_desc.BufferDesc.Width = width;
        sc_desc.BufferDesc.Height = height;
        sc_desc.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        sc_desc.BufferDesc.RefreshRate.Numerator = 60;
        sc_desc.BufferDesc.RefreshRate.Denominator = 1;
        sc_desc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
        sc_desc.OutputWindow = hwnd;
        sc_desc.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;
        sc_desc.Windowed = true as BOOL;
        sc_desc.SampleDesc.Count = 1; // Multisampling
        sc_desc.SampleDesc.Quality = 0;
        sc_desc.Flags = 0;

        let mut result = 0;
        let mut swap_chain = ptr::null_mut();
        let mut device = ptr::null_mut();
        let mut feature_level = 0;
        let mut device_context = ptr::null_mut();
        let mut driver_type = 0;
        for driver_type_ref in driver_types.iter() {
            result = unsafe {
                D3D11CreateDeviceAndSwapChain(
                    ptr::null_mut(),
                    *driver_type_ref,
                    ptr::null_mut(),
                    create_device_flags,
                    feature_levels.as_ptr(),
                    feature_levels.len() as u32,
                    D3D11_SDK_VERSION,
                    &sc_desc,
                    &mut swap_chain,
                    &mut device,
                    &mut feature_level,
                    &mut device_context,
                )
            };
            if SUCCEEDED(result) {
                driver_type = *driver_type_ref;
            }
        }
        if FAILED(result) {
            // let message = OsStr::new("FAILED TO CREATE DEVICE AND SWAP CHAIN\0")
            //     .encode_wide()
            //     .collect::<Vec<_>>();
            // unsafe { OutputDebugStringW(message.as_ptr()) };
            return Err(FailedToCreateSwapChain);
        }

        let mut back_buffer_texture = ptr::null_mut();
        let id3d11texture2d_uuid = ID3D11Texture2D::uuidof();
        unsafe {
            swap_chain.as_mut().unwrap().GetBuffer(
                0,
                &id3d11texture2d_uuid,
                &mut back_buffer_texture,
            )
        };
        let mut render_target_view = ptr::null_mut();
        unsafe {
            device.as_mut().unwrap().CreateRenderTargetView(
                back_buffer_texture as *mut ID3D11Resource,
                ptr::null(),
                &mut render_target_view,
            )
        };

        unsafe {
            device_context.as_mut().unwrap().OMSetRenderTargets(
                1,
                &render_target_view,
                ptr::null_mut(),
            )
        };

        let mut viewport = D3D11_VIEWPORT::default();
        viewport.Width = width as f32;
        viewport.Height = height as f32;
        viewport.TopLeftX = 0.0;
        viewport.TopLeftY = 0.0;
        viewport.MinDepth = 0.0;
        viewport.MaxDepth = 1.0;

        unsafe {
            device_context
                .as_ref()
                .unwrap()
                .RSSetViewports(1, &viewport)
        };

        Ok(Self {
            hwnd,
            hinstance,
            device,
            device_context,
            swap_chain,
            render_target_view,
            driver_type,
            feature_level,
            viewport,
        })
    }

    fn render(&self) {
        unsafe {
            self.device_context
                .as_ref()
                .unwrap()
                .ClearRenderTargetView(self.render_target_view, &[1.0, 0.5, 0.0, 1.0])
        };
        unsafe { self.swap_chain.as_ref().unwrap().Present(0, 0) };
    }
}

impl ops::Drop for D3D11Renderer {
    fn drop(&mut self) {
        if let Some(device_context) = unsafe { self.device_context.as_ref() } {
            unsafe { device_context.ClearState() };
        }
        if let Some(render_target_view) = unsafe { self.render_target_view.as_ref() } {
            unsafe { render_target_view.Release() };
        }
        if let Some(swap_chain) = unsafe { self.swap_chain.as_ref() } {
            unsafe { swap_chain.Release() };
        }
        if let Some(device_context) = unsafe { self.device_context.as_ref() } {
            unsafe { device_context.Release() };
        }
        if let Some(device) = unsafe { self.device.as_ref() } {
            unsafe { device.Release() };
        }
    }
}

#[derive(Debug)]
struct FailedToCreateSwapChain;
