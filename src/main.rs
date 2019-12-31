use std::{convert::TryInto, ffi::c_void, mem, ops, ptr};

use imgui_winit_support::{HiDpiMode, WinitPlatform};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use winapi::{
    shared::{
        dxgi::{
            IDXGIAdapter, IDXGIDevice, IDXGIFactory, IDXGISwapChain, DXGI_SWAP_CHAIN_DESC,
            DXGI_SWAP_EFFECT_DISCARD,
        },
        dxgiformat::{DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM},
        dxgitype::DXGI_USAGE_RENDER_TARGET_OUTPUT,
        minwindef::{BOOL, HINSTANCE},
        windef::HWND,
        winerror::{FAILED, HRESULT, S_OK},
    },
    um::{
        d3d11::{
            D3D11CreateDeviceAndSwapChain, D3D11_SHADER_RESOURCE_VIEW_DESC_u, ID3D11BlendState,
            ID3D11Buffer, ID3D11DepthStencilState, ID3D11Device, ID3D11DeviceContext,
            ID3D11InputLayout, ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView,
            ID3D11Resource, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D,
            ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER, D3D11_BLEND_DESC,
            D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_OP_ADD, D3D11_BLEND_SRC_ALPHA, D3D11_BLEND_ZERO,
            D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS,
            D3D11_CPU_ACCESS_WRITE, D3D11_CULL_NONE, D3D11_DEPTH_STENCIL_DESC,
            D3D11_DEPTH_WRITE_MASK_ALL, D3D11_FILL_SOLID, D3D11_INPUT_ELEMENT_DESC,
            D3D11_INPUT_PER_VERTEX_DATA, D3D11_RASTERIZER_DESC, D3D11_SDK_VERSION,
            D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SRV_DIMENSION_TEXTURE2D, D3D11_STENCIL_OP_KEEP,
            D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC,
            D3D11_VIEWPORT,
        },
        d3dcommon::{
            D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_REFERENCE, D3D_DRIVER_TYPE_WARP,
            D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0,
            D3D_FEATURE_LEVEL_9_3,
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
use wio::com::ComPtr;

const VERTEX_SHADER_BLOB: &[u8] = include_bytes!(env!("IMGUI_VERTEX_SHADER_BLOB"));
const PIXEL_SHADER_BLOB: &[u8] = include_bytes!(env!("IMGUI_PIXEL_SHADER_BLOB"));

fn main() {
    let event_loop = EventLoop::new();
    let (window, width, height, hidpi_factor) = {
        let window = WindowBuilder::new()
            .with_visible(false)
            .build(&event_loop)
            .unwrap();

        let hidpi_factor = window.hidpi_factor();
        let (width, height) = window.inner_size().to_physical(hidpi_factor).into();

        (window, width, height, hidpi_factor)
    };

    let mut imgui = imgui::Context::create();
    let mut platform = WinitPlatform::init(&mut imgui);
    platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);
    imgui.set_ini_filename(None);

    let mut renderer = D3D11Renderer::create(&window, width, height).unwrap();

    window.set_visible(true);

    event_loop.run(move |event, _, control_flow| {
        platform.handle_event(imgui.io_mut(), &window, &event);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                let (width, height) = window.inner_size().to_physical(hidpi_factor).into();
                renderer.resize(width, height);
            }
            Event::EventsCleared => {
                platform
                    .prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame");
                let _ui = imgui.frame();
                renderer.render();
                unimplemented!();
                renderer.present();
            }
            _ => {}
        }
    });
}

macro_rules! release_com_object {
    ($var:expr, $null_kind:ident) => {
        if let Some(com_object) = unsafe { $var.as_ref() } {
            unsafe { com_object.Release() };
            $var = ptr::$null_kind();
        }
    };
}

#[allow(dead_code)]
struct D3D11Renderer {
    device: *const ID3D11Device,
    device_context: *const ID3D11DeviceContext,
    swap_chain: *const IDXGISwapChain,
    render_target_view: *mut ID3D11RenderTargetView,
}

impl D3D11Renderer {
    fn create(
        window: &impl HasRawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self, FailedToCreateSwapChain> {
        let (hwnd, _) = {
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
            device,
            device_context,
            swap_chain,
            render_target_view,
        })
    }

    fn resize(&mut self, _width: u32, _heigth: u32) {
        unimplemented!()
    }

    fn render(&self) {
        unsafe {
            self.device_context
                .as_ref()
                .unwrap()
                .ClearRenderTargetView(self.render_target_view, &[1.0, 0.5, 0.0, 1.0])
        };
    }

    fn present(&self) {
        unsafe { self.swap_chain.as_ref().unwrap().Present(0, 0) };
    }
}

impl ops::Drop for D3D11Renderer {
    fn drop(&mut self) {
        if let Some(device_context) = unsafe { self.device_context.as_ref() } {
            unsafe { device_context.ClearState() };
        }
        release_com_object!(self.render_target_view, null_mut);
        release_com_object!(self.swap_chain, null);
        release_com_object!(self.device_context, null);
        release_com_object!(self.device, null);
    }
}

#[derive(Debug)]
struct FailedToCreateSwapChain;

/// Renders imgui with Direct3D 11
///
/// # Safety
///
/// This struct should be dropped before the `ID3D11Device` is released.
/// I have no idea what will happen if it is dropped after.
struct D3D11ImGuiRenderer {
    device: ComPtr<ID3D11Device>,
    device_context: ComPtr<ID3D11DeviceContext>,
    factory: ComPtr<IDXGIFactory>,
    vertex_buffer: ComPtr<ID3D11Buffer>,
    index_buffer: ComPtr<ID3D11Buffer>,
    vertex_shader: ComPtr<ID3D11VertexShader>,
    input_layout: ComPtr<ID3D11InputLayout>,
    vertex_constant_buffer: ComPtr<ID3D11Buffer>,
    pixel_shader: ComPtr<ID3D11PixelShader>,
    font_sampler: ComPtr<ID3D11SamplerState>,
    font_texture_view: ComPtr<ID3D11ShaderResourceView>,
    rasterizer_state: ComPtr<ID3D11RasterizerState>,
    blend_state: ComPtr<ID3D11BlendState>,
    depth_stencil_state: ComPtr<ID3D11DepthStencilState>,
    vertex_buffer_size: i32,
    index_buffer_size: i32,
}

#[allow(dead_code)]
impl D3D11ImGuiRenderer {
    fn init(
        device: &mut ID3D11Device,
        device_context: &mut ID3D11DeviceContext,
        imgui: &mut imgui::Context,
    ) -> Result<Self, HRESULT> {
        let io = imgui.io_mut();
        io.backend_flags |= imgui::BackendFlags::RENDERER_HAS_VTX_OFFSET;

        // FIXME: Check more COM pointers for NUL. Might be worth investigating
        //        wether or not a macro would be a good idea.

        let (factory, device, device_context) = {
            let dxgi_device = {
                let mut dxgi_device = ptr::null_mut();
                let hresult =
                    unsafe { device.QueryInterface(&IDXGIDevice::uuidof(), &mut dxgi_device) };
                if hresult != S_OK {
                    return Err(hresult);
                }
                unsafe { ComPtr::from_raw(dxgi_device as *mut IDXGIDevice) }
            };
            let dxgi_adapter = {
                let mut dxgi_adapter = ptr::null_mut();
                let hresult =
                    unsafe { dxgi_device.GetParent(&IDXGIAdapter::uuidof(), &mut dxgi_adapter) };
                if hresult != S_OK {
                    return Err(hresult);
                }
                unsafe { ComPtr::from_raw(dxgi_adapter as *mut IDXGIAdapter) }
            };
            let factory = {
                let mut factory = ptr::null_mut();
                let hresult =
                    unsafe { dxgi_adapter.GetParent(&IDXGIFactory::uuidof(), &mut factory) };
                if hresult != S_OK {
                    return Err(hresult);
                }
                unsafe { ComPtr::from_raw(factory as *mut IDXGIFactory) }
            };

            mem::drop(dxgi_adapter);
            mem::drop(dxgi_device);

            unsafe { device.AddRef() };
            unsafe { device_context.AddRef() };

            let device = unsafe { ComPtr::from_raw(device) };
            let device_context = unsafe { ComPtr::from_raw(device_context) };

            (factory, device, device_context)
        };

        // CreateDeviceObjects

        let vertex_shader = {
            let mut vertex_shader = ptr::null_mut();
            let hresult = unsafe {
                device.CreateVertexShader(
                    VERTEX_SHADER_BLOB.as_ptr() as *const c_void,
                    VERTEX_SHADER_BLOB.len(),
                    ptr::null_mut(),
                    &mut vertex_shader,
                )
            };
            if hresult != S_OK {
                return Err(hresult);
            }
            unsafe { ComPtr::from_raw(vertex_shader as *mut ID3D11VertexShader) }
        };

        let input_layout = {
            let local_layout = [
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: b"POSITION\0".as_ptr() as *const i8,
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    // The official documentation says that this field is optional, yet
                    // Dear ImGui's imgui_impl_dx11 example sets this field.
                    // TODO: Figure out if this field needs to be set.
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: b"TEXCOORD\0".as_ptr() as *const i8,
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 8,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: b"COLOR\0".as_ptr() as *const i8,
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    InputSlot: 0,
                    AlignedByteOffset: 16,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
            ];

            let mut input_layout = ptr::null_mut();
            let hresult = unsafe {
                device.CreateInputLayout(
                    local_layout.as_ptr(),
                    local_layout.len().try_into().unwrap(),
                    VERTEX_SHADER_BLOB.as_ptr() as *const c_void,
                    VERTEX_SHADER_BLOB.len(),
                    &mut input_layout,
                )
            };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(input_layout) }
        };

        let constant_buffer = {
            let mut desc = D3D11_BUFFER_DESC::default();
            desc.ByteWidth = mem::size_of::<VertexConstantBuffer>().try_into().unwrap();
            desc.Usage = D3D11_USAGE_DYNAMIC;
            desc.BindFlags = D3D11_BIND_CONSTANT_BUFFER;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
            desc.MiscFlags = 0;

            let mut vertex_constant_buffer = ptr::null_mut();
            let hresult =
                unsafe { device.CreateBuffer(&desc, ptr::null_mut(), &mut vertex_constant_buffer) };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(vertex_constant_buffer) };
        };

        let pixel_shader = {
            let mut pixel_shader = ptr::null_mut();
            let hresult = unsafe {
                device.CreatePixelShader(
                    PIXEL_SHADER_BLOB.as_ptr() as *const c_void,
                    PIXEL_SHADER_BLOB.len().try_into().unwrap(),
                    ptr::null_mut(),
                    &mut pixel_shader,
                )
            };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(pixel_shader) }
        };

        let blend_state = {
            let mut desc = D3D11_BLEND_DESC::default();
            desc.AlphaToCoverageEnable = false as BOOL;
            desc.RenderTarget[0].BlendEnable = true as BOOL;
            desc.RenderTarget[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
            desc.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
            desc.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
            desc.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
            desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ZERO;
            desc.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
            desc.RenderTarget[0].RenderTargetWriteMask =
                D3D11_COLOR_WRITE_ENABLE_ALL.try_into().unwrap();

            let mut blend_state = ptr::null_mut();
            let hresult = unsafe { device.CreateBlendState(&desc, &mut blend_state) };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(blend_state) }
        };

        let rasterizer_state = {
            let mut desc = D3D11_RASTERIZER_DESC::default();
            desc.FillMode = D3D11_FILL_SOLID;
            desc.CullMode = D3D11_CULL_NONE;
            desc.ScissorEnable = true as BOOL;
            desc.DepthClipEnable = true as BOOL;

            let mut rasterizer_state = ptr::null_mut();
            let hresult = unsafe { device.CreateRasterizerState(&desc, &mut rasterizer_state) };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(rasterizer_state) };
        };

        let depth_stencil_state = {
            let mut desc = D3D11_DEPTH_STENCIL_DESC::default();
            desc.DepthEnable = false as BOOL;
            desc.DepthWriteMask = D3D11_DEPTH_WRITE_MASK_ALL;
            desc.DepthFunc = D3D11_COMPARISON_ALWAYS;
            desc.StencilEnable = false as BOOL;
            desc.FrontFace.StencilFailOp = D3D11_STENCIL_OP_KEEP;
            desc.FrontFace.StencilDepthFailOp = D3D11_STENCIL_OP_KEEP;
            desc.FrontFace.StencilPassOp = D3D11_STENCIL_OP_KEEP;
            desc.FrontFace.StencilFunc = D3D11_COMPARISON_ALWAYS;
            desc.BackFace = desc.FrontFace;

            let mut depth_stencil_state = ptr::null_mut();
            let hresult =
                unsafe { device.CreateDepthStencilState(&desc, &mut depth_stencil_state) };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(depth_stencil_state) }
        };

        let font_texture_view = {
            let font_texture = {
                let text_data = imgui.fonts().build_rgba32_texture();

                let mut desc = D3D11_TEXTURE2D_DESC::default();
                desc.Width = text_data.width;
                desc.Height = text_data.height;
                desc.MipLevels = 1;
                desc.ArraySize = 1;
                desc.Format = DXGI_FORMAT_R8B8B8A8_UNORM;
                desc.SampleDesc.Count = 1;
                desc.Usage = D3D11_USAGE_DEFAULT;
                desc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
                desc.CPUAccessFlags = 0;

                let mut sub_resource = D3D11_SUBRESOURCE_DATA::default();
                sub_resource.pSysMem = text_data.data.as_ptr() as *const c_void;
                sub_resource.SysMemPitch = desc.Width * 4;
                sub_resource.SysMemSlicePitch = 0;

                let mut texture = ptr::null_mut();
                let hresult = unsafe { device.CreateTexture2D(&desc, &sub_resource, &mut texture) };
                if hresult != S_OK {
                    return Err(hresult);
                }

                unsafe { ComPtr::from_raw(texture) }
            };

            let mut desc = D3D11_SHADER_RESOURCE_VIEW_DESC::default();
            desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
            desc.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
            desc.u.Texture2D_mut().MipLevels = 1;
            desc.u.Texture2D_mut().MostDetailedMip = 0;

            let mut font_texture_view = ptr::null_mut();
            let hresult =
                unsafe { device.CreateShaderResourceView(&texture, &desc, &mut font_texture_view) };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(font_texture_view) };
        };

        unimplemented!()
    }
}

impl ops::Drop for D3D11ImGuiRenderer {
    fn drop(&mut self) {
        unsafe { self.factory.Release() };
        unsafe { self.device_context.Release() };
        unsafe { self.device.Release() };
    }
}

#[repr(C)]
struct VertexConstantBuffer {
    mvp: [[f32; 4]; 4],
}
