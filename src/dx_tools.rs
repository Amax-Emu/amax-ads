use std::ffi::{c_void, CString};
use windows::Win32::Graphics::Direct3D9::{IDirect3DDevice9, IDirect3DTexture9, D3DFMT_R8G8B8, D3DFORMAT, D3DPOOL, D3DPOOL_MANAGED};
use windows::{
	core::{HRESULT, PCSTR, PCWSTR},
	Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

pub fn get_d3d9_device() -> Option<*mut IDirect3DDevice9> {
	let dev_ptr = crate::MyPlugin::get_api().get_d3d9dev() as *mut IDirect3DDevice9;
	Some(dev_ptr)
}

pub fn d3d9_load_texture_from_memory_ex_new(
	d3d9_device: *mut IDirect3DDevice9,
	mut tex_buffer: Vec<u8>,
) -> Result<Option<IDirect3DTexture9>, ()> {
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
	let func_addr =
		get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileInMemoryEx")
			.expect("could not find 'D3DXCreateTextureFromFileInMemoryEx' address");

	let mut texture: Option<IDirect3DTexture9> = None;

	let d3d9_func: D3DXCreateTextureFromFileInMemoryEx = unsafe { std::mem::transmute(func_addr) };

	unsafe {
		// https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dxcreatetexturefromfileinmemoryexHRESULT D3DXCreateTextureFromFileInMemoryEx(
		let result = d3d9_func(
			&*d3d9_device,                             // pDevice
			tex_buffer.as_mut_ptr(),                   // pSrcData
			tex_buffer.len(),                          // SrcDataSize
			u32::MAX,                                  // Width
			u32::MAX,                                  // Height
			1,                                         // MipLevels
			0,                                         // Usage
			D3DFMT_R8G8B8,                             // D3DFORMAT format
			D3DPOOL_MANAGED,                           // D3DPOOL
			3 << 0,                                    // Filter
			3 << 0,                                    // MipFilter
			0,                                         // ColorKey
			std::ptr::null_mut(),                      // *pSrcInfo
			std::ptr::null_mut(),                      // *pPalette
			std::ptr::addr_of_mut!(texture) as *mut _, // *ppTexture
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
		.chain(std::iter::once(0))
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
