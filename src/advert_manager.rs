use std::ffi::{c_void};
use std::os::raw::c_char;

use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

#[repr(C)]
pub struct AdvertManager {
	pub unk1: u32,
	pub unk2: u32,
	pub unk3: u32,
	pub unk4: [u8; 0x28],
	pub platform_name: [u8; 0xC],
	pub unk5: u32,
	pub platform_version: [u8; 0x4],
	pub unk6: [u8; 0xC],
	pub level_instance_ptr: *mut c_void,
	pub ptr_to_textures: *mut AdvertTexture,
	pub unk7: u32, //num of ads + 1
	pub num_of_ads: u32,
}

#[repr(C)]
pub struct AdvertTexture {
	pub unk1: [u8; 0xC],
	pub size: u32,
	pub zero: u32,
	pub ptr_to_dx_texture: *mut IDirect3DTexture9,
	pub unk_id: u32,
	pub size_x10: u16,   //always 0x10
	pub texture_id: u16, //advert[10]
	pub mode: u32, //Not sure what it exactly represents. To make textures to show up in game a value of 2 is required. It can be altered up to 4, then cycle back to 0. When in game, can goes as high as 6.
}

#[repr(C)]
pub struct PLevelResource {
	_header1: [u8; 0x10],
	_byte1: u8,
	pub level_name: [c_char; 0x107],
}

