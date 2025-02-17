use std::{
	ffi::{c_void, CString},
	iter, ptr,
};
use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;
use windows::Win32::Graphics::Direct3D9::*;
use windows::{
	core::{HRESULT, PCSTR, PCWSTR},
	Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

type D3DXCreateTextureFromFileInMemoryEx = extern "stdcall" fn(
	device: &IDirect3DDevice9,
	pSrcData: *mut u8,
	SrcDataSize: usize,
	Width: u32,
	Height: u32,
	MipLevels: u32,
	Usage: u32,
	Format: D3DFORMAT,
	Pool: D3DPOOL,
	Filter: u32,
	MipFilter: u32,
	ColorKey: u32, //D3DCOLOR
	pSrcInfo: *mut c_void,
	pPalette: *mut c_void,
	ppTexture: *mut IDirect3DTexture9,
) -> HRESULT;

pub unsafe fn get_d3d9_device() -> Option<*mut IDirect3DDevice9> {
	let dev_ptr = crate::MyPlugin::get_api().get_d3d9dev() as *mut IDirect3DDevice9;
	Some(dev_ptr)
}

pub fn d3d9_load_texture_from_memory_ex_new(
	d3d9_device: *mut IDirect3DDevice9,
	mut tex_buffer: Vec<u8>,
) -> Result<Option<IDirect3DTexture9>, ()> {
	let func_addr =
		get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileInMemoryEx")
			.expect("could not find 'D3DXCreateTextureFromFileInMemoryEx' address");

	let mut texture: Option<IDirect3DTexture9> = None;

	let d3d9_func: D3DXCreateTextureFromFileInMemoryEx = unsafe { std::mem::transmute(func_addr) };

	unsafe {
		let result = d3d9_func(
			&*d3d9_device,
			tex_buffer.as_mut_ptr(),
			tex_buffer.len(),
			u32::MAX,
			u32::MAX,
			1,
			0,
			D3DFMT_R8G8B8,
			D3DPOOL_MANAGED,
			3 << 0,
			3 << 0,
			0,
			ptr::null_mut(),
			ptr::null_mut(),
			ptr::addr_of_mut!(texture) as *mut _,
		);

		log::debug!(
			"Result of D3DXCreateTextureFromFileInMemoryEx: {:?}",
			&result
		);

		if result.is_ok() {
			Ok(texture)
		} else {
			Err(())
		}
	}
}

pub fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
	let module = module
		.encode_utf16()
		.chain(iter::once(0))
		.collect::<Vec<u16>>();
	let symbol = CString::new(symbol).unwrap_or_default();
	unsafe {
		let handle = GetModuleHandleW(PCWSTR(module.as_ptr() as _)).unwrap_or_default();
		match GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)) {
			Some(func) => Some(func as usize),
			None => None,
		}
	}
}
