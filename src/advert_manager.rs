use retour::static_detour;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::{fs, ptr, thread};

use windows::Win32::Graphics::Direct3D9::*;

use crate::dx_tools::{d3d9_load_texture_from_memory_ex_new, get_d3d9_device};
use crate::file_utils::{
	download_ads_zip, get_local_checksum, get_remote_checksum, remove_ads_dir, unpack_ads,
	write_ads_checksum,
};

#[repr(C)]
struct AdvertManager {
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
struct AdvertTexture {
	pub unk1: [u8; 0xC],
	pub size: u32,
	pub zero: u32,
	pub ptr_to_dx_texture: IDirect3DTexture9,
	pub unk_id: u32,
	pub size_x10: u16,   //always 0x10
	pub texture_id: u16, //advert[10]
	pub mode: u32, //Not sure what it exactly represents. To make textures to show up in game a value of 2 is required. It can be altered up to 4, then cycle back to 0. When in game, can goes as high as 6.
}

struct PLevelResource {
	_header1: [u8; 0x10],
	_byte1: u8,
	level_name: [c_char; 0x107],
}

type GetAdPositionOnLevel = extern "thiscall" fn(
	pLevelResource: *mut c_void,
	out_position: *mut u32,
	texture_name: *const i8,
);

type GetLevelAdsData = extern "fastcall" fn(pLevelResource: *mut c_void) -> u32;

//0085de70
static_detour! {
	static EnterZone_PostLoad: unsafe extern "thiscall" fn(*mut AdvertManager,*mut PLevelResource,*mut c_void);
}

//0085e530
static_detour! {
	static AdvertManager_InitializeSystem: unsafe extern "thiscall" fn(*mut AdvertManager);
}

pub fn install_hook_zone_postload(ptr_base: *mut c_void) {
	type FnEnterZonePostLoad =
		unsafe extern "thiscall" fn(*mut AdvertManager, *mut PLevelResource, *mut c_void);
	const ORG_FN_ADDRESS_OFFSET_ENTER_ZONE_POST_LOAD: isize = 0x45DE70;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_ENTER_ZONE_POST_LOAD);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnEnterZonePostLoad>(ptr);
		EnterZone_PostLoad
			.initialize(ptr, enter_zone_post_load)
			.unwrap()
			.enable()
			.unwrap()
	};
}

pub fn install_hook_advert_manager_initialize_system(ptr_base: *mut c_void) {
	type FnAdvertManagerInitializeSystem = unsafe extern "thiscall" fn(*mut AdvertManager);
	const ORG_FN_ADDRESS_OFFSET_ADVERT_MANAGER_INITIALIZE_SYSTEM: isize = 0x45E530;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_ADVERT_MANAGER_INITIALIZE_SYSTEM);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnAdvertManagerInitializeSystem>(ptr);
		AdvertManager_InitializeSystem
			.initialize(ptr, advert_manager_initialize)
			.unwrap()
			.enable()
			.unwrap()
	};
}

fn advert_manager_initialize(advert_manager: *mut AdvertManager) {
	thread::spawn(|| {
		if let Some(appdata_amax_path) = get_appdata_amax_path() {
			let local_checksum = get_local_checksum(&appdata_amax_path).unwrap_or_default();
			let remote_checksum_opt = get_remote_checksum();

			match remote_checksum_opt {
				Some(remote_checksum) => match remote_checksum == local_checksum {
					true => {
						log::info!("Local and Remote checksums are the same. Skipping download.");
					}
					false => {
						log::info!("Downloading latest ads files...");
						remove_ads_dir(&appdata_amax_path);
						match download_ads_zip(&appdata_amax_path) {
							Some(path_to_zip) => match unpack_ads(&path_to_zip) {
								Ok(_) => {
									write_ads_checksum(&appdata_amax_path, remote_checksum);
								}
								Err(e) => {
									log::error!("Failed to unpack ads archive - {e}")
								}
							},
							None => {
								log::error!("Failed to download ads archive!")
							}
						};
					}
				},
				None => {}
			}
		}
	});

	unsafe { AdvertManager_InitializeSystem.call(advert_manager) }
}

fn enter_zone_post_load(
	advert_manager: *mut AdvertManager,
	p_level_resource: *mut PLevelResource,
	p_level_instance: *mut c_void,
) {
	let appdata_amax_path = match get_appdata_amax_path() {
		Some(path) => path,
		None => {
			log::error!("Failed to get appdata blur dir. Falling back to base function.");
			return unsafe {
				EnterZone_PostLoad.call(advert_manager, p_level_resource, p_level_instance)
			};
		}
	};

	let ads_path = appdata_amax_path.join("ads");

	match ads_path.is_dir() {
		true => {}
		false => {
			log::error!("Failed to get ads dir. Falling back to base function.");
			return unsafe {
				EnterZone_PostLoad.call(advert_manager, p_level_resource, p_level_instance)
			};
		}
	}

	unsafe {
		(*advert_manager).level_instance_ptr = p_level_instance;

		let level_name_full = CStr::from_ptr((*p_level_resource).level_name.as_ptr())
			.to_str()
			.unwrap_or_default();

		log::debug!("Level file path - {}", level_name_full);

		let level_name = level_name_full
			.replace(".\\levels\\", "")
			.replace("\\level.level", "");

		log::debug!("Level name - {}", &level_name);

		let num_of_ads = (*advert_manager).num_of_ads;
		log::debug!("num_of_ads - {}", num_of_ads);
		if num_of_ads < 1 {
			return;
		}

		let get_ads_pos_fn: GetAdPositionOnLevel = std::mem::transmute(0x00723cf0);
		let get_level_ads_data: GetLevelAdsData = std::mem::transmute(0x0087de10);

		let mut j: isize = 0;

		let d3d9_device = match get_d3d9_device() {
			Some(device) => device,
			None => {
				return;
			}
		};

		for i in (1..num_of_ads + 1).rev() {
			let texture_name = format!("advert{}", i);
			log::debug!("Checking texture {}", &texture_name);
			let name_of_texture = CString::new(texture_name).unwrap_or_default();

			let mut adv_pos = 0;
			let level_ads_data = get_level_ads_data(p_level_resource as _);
			get_ads_pos_fn(
				level_ads_data as _,
				ptr::addr_of_mut!(adv_pos),
				name_of_texture.as_ptr(),
			);

			log::debug!("Ads pos - {:x}", adv_pos);

			let full_path = appdata_amax_path
				.join("ads")
				.join(&level_name)
				.join(format!("advert{}.png", i));
			log::debug!("File path {:?}", &full_path);

			let img_data = match std::fs::read(&full_path) {
				Ok(img_data) => img_data,
				Err(e) => {
					log::error!("Failed to read file {:?} - {e}", &full_path);
					continue;
				}
			};

			let size = img_data.len() as u32;

			let new_texture = match d3d9_load_texture_from_memory_ex_new(d3d9_device, img_data) {
				Ok(texture_result) => match texture_result {
					Some(texture) => texture,
					None => {
						continue; //this is safe
					}
				},
				Err(_) => {
					continue; //this is safe
				}
			};

			let temp = AdvertTexture {
				unk1: [0; 0xC],
				size: size as u32,
				zero: 0,
				ptr_to_dx_texture: new_texture,
				unk_id: adv_pos,
				size_x10: 0x10,
				texture_id: i as u16,
				mode: 2,
			};
			let offset_to_write = (*advert_manager).ptr_to_textures.wrapping_offset(j);
			offset_to_write.write(temp);

			j += 1;
		}
	}
}

pub fn get_appdata_amax_path() -> Option<PathBuf> {
	let dir = match known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
	{
		Some(appdata_dir) => appdata_dir
			.join("bizarre creations")
			.join("blur")
			.join("amax"),
		None => return None,
	};

	if !&dir.is_dir() {
		match fs::create_dir_all(&dir) {
			Ok(_) => Some(dir),
			Err(e) => {
				log::error!("Failed to create amax folder in AppData: {e}");
				None
			}
		}
	} else {
		Some(dir)
	}
}
