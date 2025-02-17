use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::str::FromStr;

use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

use crate::hooks::{hook_get_ad_position_on_level, hook_get_level_ads_data};

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
	pub ptr_to_textures: *mut AdvertTexture, //FIXME: If this is an pointer to an array or something, specify how it should look
	pub unk7: u32,                           //num of ads + 1
	pub ad_count: u32,
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

impl PLevelResource {
	pub fn get_ad_pos(lvl: *mut PLevelResource, ad_name: &str) -> u32 {
		let ads_data = hook_get_level_ads_data(lvl);
		let mut ad_pos = 0;
		let ad_name = CString::from_str(ad_name).unwrap();
		hook_get_ad_position_on_level(ads_data, &mut ad_pos, ad_name.as_ptr());
		ad_pos
	}
}
