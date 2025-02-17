use std::{
	ffi::{c_void, CString},
	sync::LazyLock,
};

use windows::{
	core::HRESULT,
	Win32::Graphics::Direct3D9::{
		IDirect3DDevice9, IDirect3DTexture9, D3DFMT_R8G8B8, D3DFORMAT, D3DPOOL, D3DPOOL_MANAGED,
	},
};

pub fn get_d3d9_device() -> *mut IDirect3DDevice9 {
	crate::MyPlugin::get_api().get_d3d9dev() as *mut IDirect3DDevice9
}

/// Strong independent plugin, shouldn't don't need no "d3dx9_42.dll!D3DXCreateTextureFromFileInMemoryEx(..)"
// #[deprecated]
pub fn d3d9_create_tex_from_mem_ex_v1(
	dev: *mut IDirect3DDevice9,
	tex_buffer: &mut [u8],
) -> *mut IDirect3DTexture9 {
	//FIXME: Let's get rid of this "importing DLL business" and use device.CreateTexture(..) like the holy spirit intended.
	fn get_module_symbol_address(module: &str, symbol: &str) -> Option<usize> {
		use windows::{
			core::{PCSTR, PCWSTR},
			Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
		};
		let module = module
			.encode_utf16()
			.chain(std::iter::once(0))
			.collect::<Vec<u16>>();
		let symbol = CString::new(symbol).unwrap();
		unsafe {
			let handle = GetModuleHandleW(PCWSTR(module.as_ptr() as *const _)).unwrap();
			GetProcAddress(handle, PCSTR(symbol.as_ptr() as _)).map(|addr| addr as usize)
		}
	}

	type D3DXCreateTextureFromFileInMemoryEx = extern "stdcall" fn(
		device: *mut IDirect3DDevice9,
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
		ColorKey: u32,
		pSrcInfo: *mut c_void,
		pPalette: *mut c_void,
		ppTexture: *mut *mut IDirect3DTexture9,
	) -> HRESULT;

	static ONCE_FN_D3DX_CREATE_TEXTURE_FROM_FILE_IN_MEMORY_EX: LazyLock<
		D3DXCreateTextureFromFileInMemoryEx,
	> = LazyLock::new(|| {
		let func_addr =
			get_module_symbol_address("d3dx9_42.dll", "D3DXCreateTextureFromFileInMemoryEx")
				.expect("could not get_module_symbol_address() for 'D3DXCreateTextureFromFileInMemoryEx(..)' function in 'd3dx9_42.dll'");
		unsafe { std::mem::transmute::<usize, D3DXCreateTextureFromFileInMemoryEx>(func_addr) }
	});

	let d3d9_func = *ONCE_FN_D3DX_CREATE_TEXTURE_FROM_FILE_IN_MEMORY_EX;

	let mut tex_ptr: *mut IDirect3DTexture9 = std::ptr::null_mut();

	log::trace!("D3DXCreateTextureFromFileInMemoryEx(");
	d3d9_func(
		// ptr to IDirect3DDevice9
		dev,
		// ptr to bytes img data
		tex_buffer.as_mut_ptr(),
		// size of file in mem
		tex_buffer.len(),
		// image width
		u32::MAX,
		// image height
		u32::MAX,
		// mipLevels
		1,
		// (default?) 0  usage. idk what 0 means and I'm too scared to look it up
		0u32,
		// D3DFMT_R8G8B8 | https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dformat
		D3DFMT_R8G8B8,
		// D3DPOOL_MANAGED | https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dpool
		D3DPOOL_MANAGED, //D3DPOOL_MANAGED keeps it safe from Resets, the device handles all of the restoration for us!
		// .filter = D3DX_FILTER_NONE  | https://learn.microsoft.com/en-us/windows/win32/direct3d9/d3dx-filter
		3,
		// .MipFilter = D3DX_FILTER_NONE
		3,
		// .ColorKey = D3DCOLOR, 32bit ARGB, opaque black
		0xFF000000,
		// pSrcInfo D3DXIMAGE_INFO structure to be filled with a description of the data in the source image file, or NULL
		std::ptr::null_mut(),
		// pPalette Pointer to a PALETTEENTRY structure, representing a 256-color palette to fill in, or NULL
		std::ptr::null_mut(),
		// ppTexture | Address of a pointer to an IDirect3DTexture9 interface, representing the created texture object.
		&mut tex_ptr,
	)
	.unwrap();
	log::trace!(")");
	tex_ptr
}
