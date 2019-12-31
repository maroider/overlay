use std::{
    env,
    ffi::c_void,
    fs,
    path::{Path, PathBuf},
    ptr, slice, str,
};

use winapi::{shared::winerror::S_OK, um::d3dcompiler::D3DCompile};
use wio::com::ComPtr;

fn main() {
    let path = compile_shader("src/shaders/vertex.hlsl".as_ref(), b"vs_4_0\0");
    cargo_emit::rustc_env!("IMGUI_VERTEX_SHADER_BLOB", "{}", path.display());
    let path = compile_shader("src/shaders/pixel.hlsl".as_ref(), b"ps_4_0\0");
    cargo_emit::rustc_env!("IMGUI_PIXEL_SHADER_BLOB", "{}", path.display());
}

/// Compile the shader found at the given path.
///
/// `target` should be a null-terminated [byte literal](https://doc.rust-lang.org/reference/tokens.html#byte-literals).
/// Have a look at [this page](https://docs.microsoft.com/windows/desktop/direct3dhlsl/specifying-compiler-targets)
/// to figure out which target string to use.
fn compile_shader(path: &Path, target: &[u8]) -> PathBuf {
    if !target.ends_with(&[0]) {
        panic!("`taret` should be a null-terminated string");
    }

    let shader_src = fs::read_to_string(path).unwrap();

    let blob = {
        let mut blob = ptr::null_mut();
        let mut err = ptr::null_mut();
        let hresult = unsafe {
            D3DCompile(
                shader_src.as_ptr() as *const c_void,
                shader_src.len(),
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
                b"main\0".as_ptr() as *const i8,
                target.as_ptr() as *const i8,
                0,
                0,
                &mut blob,
                &mut err,
            )
        };
        if hresult != S_OK {
            if err != ptr::null_mut() {
                let err = unsafe { ComPtr::from_raw(err) };
                let err_msg_ptr = unsafe { err.GetBufferPointer() };
                let err_msg_buffer_size = unsafe { err.GetBufferSize() };
                let err_str = str::from_utf8(unsafe {
                    slice::from_raw_parts(err_msg_ptr as *const u8, err_msg_buffer_size)
                })
                .unwrap();
                panic!("{}", err_str);
            }
        }
        eprintln!("hresult: {:X}", hresult);
        if blob == ptr::null_mut() {
            panic!("Could not complile shader at: {}", path.display());
        }
        unsafe { ComPtr::from_raw(blob) }
    };

    let blob = {
        let blob_buffer_ptr = unsafe { blob.GetBufferPointer() as *const u8 };
        let blob_buffer_size = unsafe { blob.GetBufferSize() };
        if blob_buffer_ptr == ptr::null() {
            panic!(
                "Could not retrieve pointer to compiled shader at: {}",
                path.display()
            );
        }
        unsafe { slice::from_raw_parts(blob_buffer_ptr, blob_buffer_size) }
    };

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap())
        .join(path.file_name().unwrap())
        .with_extension("dx10blob");

    fs::write(&out_path, blob).expect(&format!(
        "Could not write compiled shader blob for shader at {} to {}",
        path.display(),
        out_path.display()
    ));

    dunce::canonicalize(&out_path).expect(&format!(
        "Could not cannonicalize path: {}",
        out_path.display()
    ))
}
