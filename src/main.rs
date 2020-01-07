use std::{convert::TryInto, ffi::c_void, mem, ops, ptr};

use imgui::{DrawData, DrawIdx, DrawVert, TextureId};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use smallvec::SmallVec;
use winapi::{
    shared::{
        dxgi::{
            IDXGIAdapter, IDXGIDevice, IDXGIFactory, IDXGISwapChain, DXGI_SWAP_CHAIN_DESC,
            DXGI_SWAP_EFFECT_DISCARD,
        },
        dxgiformat::{
            DXGI_FORMAT, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32_UINT,
            DXGI_FORMAT_R8G8B8A8_UNORM,
        },
        dxgitype::DXGI_USAGE_RENDER_TARGET_OUTPUT,
        minwindef::{BOOL, HINSTANCE},
        windef::HWND,
        winerror::{FAILED, HRESULT, SUCCEEDED, S_OK},
    },
    um::{
        d3d11::{
            D3D11CreateDeviceAndSwapChain, ID3D11BlendState, ID3D11Buffer, ID3D11ClassInstance,
            ID3D11DepthStencilState, ID3D11Device, ID3D11DeviceContext, ID3D11GeometryShader,
            ID3D11InputLayout, ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView,
            ID3D11Resource, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D,
            ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_INDEX_BUFFER,
            D3D11_BIND_SHADER_RESOURCE, D3D11_BIND_VERTEX_BUFFER, D3D11_BLEND_DESC,
            D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_OP_ADD, D3D11_BLEND_SRC_ALPHA, D3D11_BLEND_ZERO,
            D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS,
            D3D11_CPU_ACCESS_WRITE, D3D11_CULL_NONE, D3D11_DEPTH_STENCIL_DESC,
            D3D11_DEPTH_WRITE_MASK_ALL, D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_MAPPED_SUBRESOURCE,
            D3D11_PRIMITIVE_TOPOLOGY, D3D11_RASTERIZER_DESC, D3D11_RECT, D3D11_SAMPLER_DESC,
            D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_STENCIL_OP_KEEP,
            D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_TEXTURE_ADDRESS_WRAP,
            D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC, D3D11_VIEWPORT,
            D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE,
        },
        d3dcommon::{
            D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_SRV_DIMENSION_TEXTURE2D,
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

    // let mut imgui = imgui::Context::create();
    // let mut platform = WinitPlatform::init(&mut imgui);
    // platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);
    // imgui.set_ini_filename(None);

    let mut renderer = D3D11Renderer::create(&window, width, height).unwrap();
    // let mut imgui_renderer = D3D11ImGuiRenderer::init(
    //     unsafe { renderer.device.as_mut().unwrap() },
    //     unsafe { renderer.device_context.as_mut().unwrap() },
    //     &mut imgui,
    // )
    // .unwrap();

    window.set_visible(true);

    event_loop.run(move |event, _, control_flow| {
        // platform.handle_event(imgui.io_mut(), &window, &event);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                // let (width, height) = window.inner_size().to_physical(hidpi_factor).into();
                // renderer.resize(width, height);
            }
            Event::EventsCleared => {
                // platform
                //     .prepare_frame(imgui.io_mut(), &window)
                //     .expect("Failed to prepare frame");
                // let ui = imgui.frame();
                renderer.render();
                // imgui_renderer.render_ui(ui);
                renderer.present();
            }
            _ => {}
        }
    });
}

macro_rules! release_com_object {
    ($var:expr) => {
        if let Some(com_object) = unsafe { $var.as_mut() } {
            unsafe { com_object.Release() };
            $var = ptr::null_mut();
        }
    };
}

/// Call COM object method where the last argument is the out variable
macro_rules! cclom {
    ($obj:expr, $method:ident$(,)? $($arg:expr),*) => {
        {
            let mut out = ptr::null_mut();
            unsafe { $obj.$method($($arg, )* &mut out) };
            if out == ptr::null_mut() {
                Err(concat!(stringify!($method), " returned a null-pointer"))
            } else {
                Ok(unsafe { ComPtr::from_raw(out) })
            }
        }
    };
}

/// Call COM object method where the first argument is the out variable
macro_rules! ccfom {
    ($obj:expr, $method:ident$(,)? $($arg:expr),*) => {
        {
            let mut out = ptr::null_mut();
            unsafe { $obj.$method(&mut out $(, $arg)* ) };
            if out == ptr::null_mut() {
                Err(concat!(stringify!($method), " returned a null-pointer"))
            } else {
                Ok(unsafe { ComPtr::from_raw(out) })
            }
        }
    };
}

#[allow(dead_code)]
struct D3D11Renderer {
    pub device: *mut ID3D11Device,
    pub device_context: *mut ID3D11DeviceContext,
    swap_chain: *mut IDXGISwapChain,
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
            if SUCCEEDED(result) {
                break;
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
        release_com_object!(self.render_target_view);
        release_com_object!(self.swap_chain);
        release_com_object!(self.device_context);
        release_com_object!(self.device);
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
    vertex_buffer_size: u32,
    index_buffer_size: u32,
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

        let vertex_constant_buffer = {
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

            unsafe { ComPtr::from_raw(vertex_constant_buffer) }
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

            unsafe { ComPtr::from_raw(rasterizer_state) }
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
            let mut font_atlas = imgui.fonts();

            let font_texture = {
                let text_data = font_atlas.build_rgba32_texture();

                let mut desc = D3D11_TEXTURE2D_DESC::default();
                desc.Width = text_data.width;
                desc.Height = text_data.height;
                desc.MipLevels = 1;
                desc.ArraySize = 1;
                desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
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
            unsafe { desc.u.Texture2D_mut().MipLevels = 1 };
            unsafe { desc.u.Texture2D_mut().MostDetailedMip = 0 };

            let mut font_texture_view = ptr::null_mut();
            let hresult = unsafe {
                device.CreateShaderResourceView(
                    font_texture.as_raw() as *mut ID3D11Resource,
                    &desc,
                    &mut font_texture_view,
                )
            };
            if hresult != S_OK {
                return Err(hresult);
            }

            font_atlas.tex_id = TextureId::from(font_texture_view);

            unsafe { ComPtr::from_raw(font_texture_view) }
        };

        let font_sampler = {
            let mut desc = D3D11_SAMPLER_DESC::default();
            desc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
            desc.AddressU = D3D11_TEXTURE_ADDRESS_WRAP;
            desc.AddressV = D3D11_TEXTURE_ADDRESS_WRAP;
            desc.AddressW = D3D11_TEXTURE_ADDRESS_WRAP;
            desc.MipLODBias = 0.0;
            desc.ComparisonFunc = D3D11_COMPARISON_ALWAYS;
            desc.MinLOD = 0.0;
            desc.MaxLOD = 0.0;

            let mut font_sampler = ptr::null_mut();
            let hresult = unsafe { device.CreateSamplerState(&desc, &mut font_sampler) };
            if hresult != S_OK {
                return Err(hresult);
            }

            unsafe { ComPtr::from_raw(font_sampler) }
        };

        let (vertex_buffer, vertex_buffer_size) = {
            let vertex_buffer_size = 5_000;

            let mut desc = D3D11_BUFFER_DESC::default();
            desc.Usage = D3D11_USAGE_DYNAMIC;
            desc.ByteWidth = vertex_buffer_size * mem::size_of::<DrawVert>() as u32;
            desc.BindFlags = D3D11_BIND_VERTEX_BUFFER;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
            desc.MiscFlags = 0;

            let mut vertex_buffer = ptr::null_mut();
            let hresult =
                unsafe { device.CreateBuffer(&desc, ptr::null_mut(), &mut vertex_buffer) };
            if hresult != S_OK {
                return Err(hresult);
            }
            let vertex_buffer = unsafe { ComPtr::from_raw(vertex_buffer) };

            (vertex_buffer, vertex_buffer_size)
        };

        let (index_buffer, index_buffer_size) = {
            let index_buffer_size = 10_000;

            let mut desc = D3D11_BUFFER_DESC::default();
            desc.Usage = D3D11_USAGE_DYNAMIC;
            desc.ByteWidth = vertex_buffer_size * mem::size_of::<DrawVert>() as u32;
            desc.BindFlags = D3D11_BIND_INDEX_BUFFER;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
            desc.MiscFlags = 0;

            let mut index_buffer = ptr::null_mut();
            let hresult = unsafe { device.CreateBuffer(&desc, ptr::null_mut(), &mut index_buffer) };
            if hresult != S_OK {
                return Err(hresult);
            }
            let index_buffer = unsafe { ComPtr::from_raw(index_buffer) };

            (index_buffer, index_buffer_size)
        };

        Ok(Self {
            device,
            device_context,
            factory,
            vertex_buffer,
            index_buffer,
            vertex_shader,
            input_layout,
            vertex_constant_buffer,
            pixel_shader,
            font_sampler,
            font_texture_view,
            rasterizer_state,
            blend_state,
            depth_stencil_state,
            vertex_buffer_size,
            index_buffer_size,
        })
    }

    pub fn render_ui(&mut self, ui: imgui::Ui) {
        let draw_data = ui.render();

        if draw_data.display_size[0] <= 0.0 || draw_data.display_size[1] <= 0.0 {
            return;
        }

        if self.vertex_buffer_size < draw_data.total_vtx_count as u32 {
            // `self.vertex_buffer` might be a dangling pointer after this
            unsafe { self.vertex_buffer.Release() };
            self.vertex_buffer_size = draw_data.total_vtx_count as u32 + 5_000;
            let mut desc = D3D11_BUFFER_DESC::default();
            desc.Usage = D3D11_USAGE_DYNAMIC;
            desc.ByteWidth = self.vertex_buffer_size * mem::size_of::<DrawVert>() as u32;
            desc.BindFlags = D3D11_BIND_VERTEX_BUFFER;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
            desc.MiscFlags = 0;
            self.vertex_buffer = {
                let mut vertex_buffer = ptr::null_mut();
                let hresult = unsafe {
                    self.device
                        .CreateBuffer(&desc, ptr::null_mut(), &mut vertex_buffer)
                };
                if hresult != S_OK {
                    panic!("Could not create new vertex buffer: {:X}", hresult)
                }

                unsafe { ComPtr::from_raw(vertex_buffer) }
            };
        }
        if self.index_buffer_size < draw_data.total_idx_count as u32 {
            // `self.index_buffer` might be a dangling pointer after this
            unsafe { self.index_buffer.Release() };
            self.index_buffer_size = draw_data.total_vtx_count as u32 + 5_000;
            let mut desc = D3D11_BUFFER_DESC::default();
            desc.Usage = D3D11_USAGE_DYNAMIC;
            desc.ByteWidth = self.index_buffer_size * mem::size_of::<DrawVert>() as u32;
            desc.BindFlags = D3D11_BIND_INDEX_BUFFER;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
            desc.MiscFlags = 0;
            self.index_buffer = {
                let mut index_buffer = ptr::null_mut();
                let hresult = unsafe {
                    self.device
                        .CreateBuffer(&desc, ptr::null_mut(), &mut index_buffer)
                };
                if hresult != S_OK {
                    panic!("Could not create new index buffer: {:X}", hresult)
                }

                unsafe { ComPtr::from_raw(index_buffer) }
            };
        }

        // let vertex_resource = {
        //     let vertex_resource = D3D11_MAPPED_SUBRESOURCE::default();
        //     unsafe { self.device_context.map() }
        // };
        // let index_resource = D3D11_MAPPED_SUBRESOURCE::default();
    }

    fn setup_render_state(&self, draw_data: &DrawData) {
        let mut viewport = D3D11_VIEWPORT::default();
        viewport.Width = draw_data.display_size[0];
        viewport.Height = draw_data.display_size[1];
        viewport.MinDepth = 0.0;
        viewport.MaxDepth = 1.0;
        viewport.TopLeftX = 0.0;
        viewport.TopLeftY = 0.0;
        unsafe { self.device_context.RSSetViewports(1, &viewport) };

        let stride = mem::size_of::<DrawVert>() as u32;
        let offset = 0;
        unsafe {
            self.device_context
                .IASetInputLayout(self.input_layout.as_raw())
        };
        unsafe {
            self.device_context.IASetVertexBuffers(
                0,
                1,
                &self.vertex_buffer.as_raw(),
                &stride,
                &offset,
            )
        };
        unsafe {
            self.device_context.IASetIndexBuffer(
                self.index_buffer.as_raw(),
                if mem::size_of::<DrawIdx>() == 2 {
                    DXGI_FORMAT_R16_UINT
                } else {
                    DXGI_FORMAT_R32_UINT
                },
                0,
            )
        };
        unsafe {
            self.device_context
                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST)
        };
        unsafe {
            self.device_context
                .VSSetShader(self.vertex_shader.as_raw(), ptr::null_mut(), 0);
        };
        unsafe {
            self.device_context
                .VSSetConstantBuffers(0, 1, &self.vertex_constant_buffer.as_raw())
        };
        unsafe {
            self.device_context
                .PSSetShader(self.pixel_shader.as_raw(), ptr::null_mut(), 0);
        };
        unsafe {
            self.device_context
                .PSSetSamplers(0, 1, &self.font_sampler.as_raw());
        };
        unsafe {
            self.device_context
                .GSSetShader(ptr::null_mut(), ptr::null_mut(), 0)
        };
        unsafe {
            self.device_context
                .HSSetShader(ptr::null_mut(), ptr::null_mut(), 0)
        };
        unsafe {
            self.device_context
                .DSSetShader(ptr::null_mut(), ptr::null_mut(), 0)
        };
        unsafe {
            self.device_context
                .CSSetShader(ptr::null_mut(), ptr::null_mut(), 0)
        };

        let blend_factor = [0.0, 0.0, 0.0, 0.0];
        unsafe {
            self.device_context.OMSetBlendState(
                self.blend_state.as_raw(),
                &blend_factor,
                0xffffffff,
            )
        };
        unsafe {
            self.device_context
                .OMSetDepthStencilState(self.depth_stencil_state.as_raw(), 0)
        };
        unsafe {
            self.device_context
                .RSSetState(self.rasterizer_state.as_raw())
        };
    }
}

impl ops::Drop for D3D11ImGuiRenderer {
    fn drop(&mut self) {
        unsafe { self.font_sampler.Release() };
        unsafe { self.font_texture_view.Release() };
        unsafe { self.depth_stencil_state.Release() };
        unsafe { self.rasterizer_state.Release() };
        unsafe { self.blend_state.Release() };
        unsafe { self.pixel_shader.Release() };
        unsafe { self.vertex_constant_buffer.Release() };
        unsafe { self.input_layout.Release() };
        unsafe { self.vertex_shader.Release() };
        unsafe { self.index_buffer.Release() };
        unsafe { self.vertex_buffer.Release() };
        unsafe { self.factory.Release() };
        unsafe { self.device_context.Release() };
        unsafe { self.device.Release() };
    }
}

#[repr(C)]
struct VertexConstantBuffer {
    mvp: [[f32; 4]; 4],
}

/// A backup of the D3D11 renderer state.
///
/// TODO: Backup Hull, Domain and Compute shaders.
///
/// TODO: Remove `SmallVec`
#[must_use]
struct BackupD3D11State {
    scissor_rects_count: u32,
    viewports_count: u32,
    scissor_rects: [D3D11_RECT; D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as usize],
    viewports: [D3D11_VIEWPORT; D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as usize],
    rasterizer_state: ComPtr<ID3D11RasterizerState>,
    blend_state: ComPtr<ID3D11BlendState>,
    blend_factor: [f32; 4],
    sample_mask: u32,
    stencil_ref: u32,
    depth_stencil_state: ComPtr<ID3D11DepthStencilState>,
    pixel_shader_shader_resource: ComPtr<ID3D11ShaderResourceView>,
    pixel_shader_sampler_state: ComPtr<ID3D11SamplerState>,
    pixel_shader: ComPtr<ID3D11PixelShader>,
    vertex_shader: ComPtr<ID3D11VertexShader>,
    geometry_shader: ComPtr<ID3D11GeometryShader>,
    pixel_shader_instances_count: u32,
    vertex_shader_instances_count: u32,
    geometry_shader_instances_count: u32,
    pixel_shader_instances: SmallVec<[ComPtr<ID3D11ClassInstance>; 256]>,
    vertex_shader_instances: SmallVec<[ComPtr<ID3D11ClassInstance>; 256]>,
    geometry_shader_instances: SmallVec<[ComPtr<ID3D11ClassInstance>; 256]>,
    primitive_topology: D3D11_PRIMITIVE_TOPOLOGY,
    index_buffer: ComPtr<ID3D11Buffer>,
    vertex_buffer: ComPtr<ID3D11Buffer>,
    vertex_constant_buffer: ComPtr<ID3D11Buffer>,
    index_buffer_offset: u32,
    vertex_buffer_stride: u32,
    vertex_buffer_offset: u32,
    index_buffer_format: DXGI_FORMAT,
    input_layout: ComPtr<ID3D11InputLayout>,
}

impl BackupD3D11State {
    fn new(device_context: &mut ID3D11DeviceContext) -> Self {
        let mut scissor_rects_count = D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE;
        let mut viewports_count = D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE;
        let mut scissor_rects = [D3D11_RECT::default();
            D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as usize];
        let mut viewports = [D3D11_VIEWPORT::default();
            D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as usize];
        unsafe {
            device_context.RSGetScissorRects(&mut scissor_rects_count, scissor_rects.as_mut_ptr())
        };
        unsafe { device_context.RSGetViewports(&mut viewports_count, viewports.as_mut_ptr()) };
        let rasterizer_state = cclom!(device_context, RSGetState).unwrap();
        let mut blend_factor = [0.0; 4];
        let mut sample_mask = 0;
        let blend_state = ccfom!(
            device_context,
            OMGetBlendState,
            &mut blend_factor,
            &mut sample_mask
        )
        .unwrap();
        let mut stencil_ref = 0;
        let depth_stencil_state =
            ccfom!(device_context, OMGetDepthStencilState, &mut stencil_ref).unwrap();
        let pixel_shader_shader_resource =
            cclom!(device_context, PSGetShaderResources, 0, 1).unwrap();
        let pixel_shader_sampler_state = cclom!(device_context, PSGetSamplers, 0, 1).unwrap();
        let mut pixel_shader_instances_count = 256;
        let mut vertex_shader_instances_count = 256;
        let mut geometry_shader_instances_count = 256;
        let mut pixel_shader_instances = [ptr::null_mut(); 256];
        let mut vertex_shader_instances = [ptr::null_mut(); 256];
        let mut geometry_shader_instances = [ptr::null_mut(); 256];
        let pixel_shader = ccfom!(device_context, PSGetShader
            pixel_shader_instances.as_mut_ptr(),
            &mut pixel_shader_instances_count
        )
        .unwrap();
        let vertex_shader = ccfom!(device_context, VSGetShader
            vertex_shader_instances.as_mut_ptr(),
            &mut vertex_shader_instances_count
        )
        .unwrap();
        let vertex_constant_buffer = cclom!(device_context, VSGetConstantBuffers, 0, 1).unwrap();
        let geometry_shader = ccfom!(device_context, GSGetShader
            geometry_shader_instances.as_mut_ptr(),
            &mut geometry_shader_instances_count
        )
        .unwrap();
        let pixel_shader_instances = {
            let mut wrapped_pixel_shader_instances = SmallVec::new();
            for i in 0..pixel_shader_instances_count as usize {
                wrapped_pixel_shader_instances
                    .push(wrap_com_ptr(pixel_shader_instances[i]).unwrap());
            }
            wrapped_pixel_shader_instances
        };
        let vertex_shader_instances = {
            let mut wrapped_vertex_shader_instances = SmallVec::new();
            for i in 0..vertex_shader_instances_count as usize {
                wrapped_vertex_shader_instances
                    .push(wrap_com_ptr(vertex_shader_instances[i]).unwrap());
            }
            wrapped_vertex_shader_instances
        };
        let geometry_shader_instances = {
            let mut wrapped_geometry_shader_instances = SmallVec::new();
            for i in 0..geometry_shader_instances_count as usize {
                wrapped_geometry_shader_instances
                    .push(wrap_com_ptr(geometry_shader_instances[i]).unwrap());
            }
            wrapped_geometry_shader_instances
        };

        let mut primitive_topology = D3D11_PRIMITIVE_TOPOLOGY::default();
        unsafe { device_context.IAGetPrimitiveTopology(&mut primitive_topology) };
        let mut index_buffer_offset = 0;
        let mut vertex_buffer_stride = 0;
        let mut vertex_buffer_offset = 0;
        let mut index_buffer_format = DXGI_FORMAT::default();
        let index_buffer = ccfom!(
            device_context,
            IAGetIndexBuffer,
            &mut index_buffer_format,
            &mut index_buffer_offset
        )
        .unwrap();
        let vertex_buffer = {
            let mut vertex_buffer = ptr::null_mut();
            unsafe {
                device_context.IAGetVertexBuffers(
                    0,
                    1,
                    &mut vertex_buffer,
                    &mut vertex_buffer_stride,
                    &mut vertex_buffer_offset,
                )
            };
            wrap_com_ptr(vertex_buffer).unwrap()
        };
        let input_layout = ccfom!(device_context, IAGetInputLayout).unwrap();

        Self {
            scissor_rects_count,
            viewports_count,
            scissor_rects,
            viewports,
            rasterizer_state,
            blend_state,
            blend_factor,
            sample_mask,
            stencil_ref,
            depth_stencil_state,
            pixel_shader_shader_resource,
            pixel_shader_sampler_state,
            pixel_shader,
            vertex_shader,
            geometry_shader,
            pixel_shader_instances_count,
            vertex_shader_instances_count,
            geometry_shader_instances_count,
            pixel_shader_instances,
            vertex_shader_instances,
            geometry_shader_instances,
            primitive_topology,
            index_buffer,
            vertex_buffer,
            vertex_constant_buffer,
            index_buffer_offset,
            vertex_buffer_stride,
            vertex_buffer_offset,
            index_buffer_format,
            input_layout,
        }
    }

    fn restore(mut self, device_context: &mut ID3D11DeviceContext) {
        unsafe {
            device_context
                .RSSetScissorRects(self.scissor_rects_count, self.scissor_rects.as_mut_ptr())
        };
        unsafe { device_context.RSSetViewports(self.viewports_count, self.viewports.as_mut_ptr()) };
        unsafe { device_context.RSSetState(self.rasterizer_state.as_raw()) };
        unsafe {
            device_context.OMSetBlendState(
                self.blend_state.as_raw(),
                &self.blend_factor,
                self.sample_mask,
            )
        };
        unsafe {
            device_context
                .OMSetDepthStencilState(self.depth_stencil_state.as_raw(), self.stencil_ref)
        };
        unsafe {
            device_context.PSSetShaderResources(0, 1, &self.pixel_shader_shader_resource.as_raw())
        };
        unsafe { device_context.PSSetSamplers(0, 1, &self.pixel_shader_sampler_state.as_raw()) };
        unsafe {
            device_context.PSSetShader(
                self.pixel_shader.as_raw(),
                mem::transmute(self.pixel_shader_instances.as_mut_ptr()),
                self.pixel_shader_instances_count,
            )
        };
        unsafe {
            device_context.VSSetShader(
                self.vertex_shader.as_raw(),
                mem::transmute(self.vertex_shader_instances.as_mut_ptr()),
                self.vertex_shader_instances_count,
            )
        };
        unsafe { device_context.VSSetConstantBuffers(0, 1, &self.vertex_constant_buffer.as_raw()) };
        unsafe {
            device_context.GSSetShader(
                self.geometry_shader.as_raw(),
                mem::transmute(self.geometry_shader_instances.as_mut_ptr()),
                self.geometry_shader_instances_count,
            )
        };
        unsafe { device_context.IASetPrimitiveTopology(self.primitive_topology) };
        unsafe {
            device_context.IASetIndexBuffer(
                self.index_buffer.as_raw(),
                self.index_buffer_format,
                self.index_buffer_offset,
            )
        };
        unsafe {
            device_context.IASetVertexBuffers(
                0,
                1,
                &self.vertex_buffer.as_raw(),
                &self.vertex_buffer_stride,
                &self.vertex_buffer_offset,
            )
        };
        unsafe { device_context.IASetInputLayout(self.input_layout.as_raw()) };
    }
}

fn wrap_com_ptr<T: Interface>(ptr: *mut T) -> Result<ComPtr<T>, NullPointerError> {
    if ptr != ptr::null_mut() {
        Ok(unsafe { ComPtr::from_raw(ptr) })
    } else {
        Err(NullPointerError)
    }
}

#[derive(Debug)]
struct NullPointerError;
