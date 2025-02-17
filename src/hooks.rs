use std::ffi::{c_char, c_void, CStr, CString};

use retour::static_detour;

use crate::{
	advert_manager::{AdvertManager, AdvertTexture, PLevelResource},
	dx_tools::get_d3d9_device,
	file_utils::{
		download_ads_zip, get_appdata_amax_path, get_local_checksum, get_remote_checksum,
		remove_ads_dir, unpack_ads, write_ads_checksum,
	},
};

pub fn install(ptr_base: *mut c_void) {
	install_hook_advert_manager_initialize_system(ptr_base);
	install_hook_enter_zone_post_load(ptr_base);
	install_hook_get_level_ads_data(ptr_base);
	install_hook_get_ad_position_on_level(ptr_base);
}

//0x0085de70
static_detour! {
	static EnterZone_PostLoad:
		unsafe extern "thiscall" fn(
			*mut AdvertManager,
			*mut PLevelResource,
			*mut c_void
		);
}

//0x0085e530
static_detour! {
	static AdvertManager_InitializeSystem:
		extern "thiscall" fn(
			*mut AdvertManager
		);
}

//0x0087de10
static_detour! {
	static GetLevelAdsData:
		extern "fastcall" fn(
			*mut PLevelResource
		) -> *mut PLevelResource;
}

//0x00723cf0
static_detour! {
	static GetAdPositionOnLevel:
		extern "thiscall" fn(
			*mut PLevelResource,
			*mut u32,
			*const c_char
		);
}

fn install_hook_get_ad_position_on_level(ptr_base: *mut c_void) {
	type FnGetAdPositionOnLevel = extern "thiscall" fn(
		p_level_resource: *mut PLevelResource,
		out_position: *mut u32,
		texture_name: *const c_char,
	);
	const ORG_FN_ADDRESS_OFFSET_GET_AD_POSITION_ON_LEVEL: isize = 0x323CF0;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_GET_AD_POSITION_ON_LEVEL);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnGetAdPositionOnLevel>(ptr);
		GetAdPositionOnLevel
			.initialize(ptr, hook_get_ad_position_on_level)
			.unwrap()
			.enable()
			.unwrap()
	}
}

fn install_hook_enter_zone_post_load(ptr_base: *mut c_void) {
	type FnEnterZonePostLoad =
		unsafe extern "thiscall" fn(*mut AdvertManager, *mut PLevelResource, *mut c_void);
	const ORG_FN_ADDRESS_OFFSET_ENTER_ZONE_POST_LOAD: isize = 0x45DE70;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_ENTER_ZONE_POST_LOAD);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnEnterZonePostLoad>(ptr);
		EnterZone_PostLoad
			.initialize(ptr, hook_enter_zone_post_load)
			.unwrap()
			.enable()
			.unwrap()
	};
}

fn install_hook_advert_manager_initialize_system(ptr_base: *mut c_void) {
	type FnAdvertManagerInitializeSystem = extern "thiscall" fn(*mut AdvertManager);
	const ORG_FN_ADDRESS_OFFSET_ADVERT_MANAGER_INITIALIZE_SYSTEM: isize = 0x45E530;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_ADVERT_MANAGER_INITIALIZE_SYSTEM);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnAdvertManagerInitializeSystem>(ptr);
		AdvertManager_InitializeSystem
			.initialize(ptr, hook_advert_manager_initialize_system)
			.unwrap()
			.enable()
			.unwrap()
	};
}

fn install_hook_get_level_ads_data(ptr_base: *mut c_void) {
	type FnGetLevelAdsData = extern "fastcall" fn(*mut PLevelResource) -> *mut PLevelResource;
	const ORG_FN_ADDRESS_OFFSET_GET_LEVEL_ADS_DATA: isize = 0x47DE10;
	let ptr = ptr_base.wrapping_byte_offset(ORG_FN_ADDRESS_OFFSET_GET_LEVEL_ADS_DATA);
	unsafe {
		let ptr = std::mem::transmute::<*mut c_void, FnGetLevelAdsData>(ptr);
		GetLevelAdsData
			.initialize(ptr, hook_get_level_ads_data)
			.unwrap()
			.enable()
			.unwrap()
	};
}

pub fn hook_advert_manager_initialize_system(advert_manager: *mut AdvertManager) {
	{
		let p = crate::MyPlugin::get_api().get_d3d9dev();
		log::warn!("AdvertManager_InitializeSystem called! get_d3d9dev() is {p:?}");
	}
	std::thread::Builder::new()
		.name("AMAX-ADS-Downloader".to_string())
		.spawn(|| {
			if let Some(appdata_amax_path) = get_appdata_amax_path() {
				let local_checksum = get_local_checksum(&appdata_amax_path).unwrap_or_default();
				let remote_checksum_opt = get_remote_checksum();

				match remote_checksum_opt {
					Some(remote_checksum) => match remote_checksum == local_checksum {
						true => {
							log::info!(
								"Local and Remote checksums are the same. Skipping download."
							);
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
				let cache = crate::cache::AdCache::g();
				log::debug!("{cache:?}")
			}
		})
		.expect("failed to created amax ads downloader thread");

	AdvertManager_InitializeSystem.call(advert_manager);
}

pub fn hook_enter_zone_post_load(
	advert_manager: *mut AdvertManager,
	p_level_resource: *mut PLevelResource,
	p_level_instance: *mut c_void,
) {
	let Some(appdata_amax_path) = get_appdata_amax_path() else {
		log::error!("Failed to get appdata blur dir. Falling back to base function.");
		return unsafe {
			EnterZone_PostLoad.call(advert_manager, p_level_resource, p_level_instance)
		};
	};
	let ads_path = appdata_amax_path.join("ads");
	if !ads_path.is_dir() {
		log::error!("Failed to get ads dir. Falling back to base function.");
		return unsafe {
			EnterZone_PostLoad.call(advert_manager, p_level_resource, p_level_instance)
		};
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

		// let get_ads_pos_fn: FnGetAdPositionOnLevel = std::mem::transmute(0x00723cf0);
		// let get_level_ads_data: GetLevelAdsData = std::mem::transmute(0x0087de10);

		let mut j: isize = 0;

		let Some(d3d9_device) = get_d3d9_device() else {
			return;
		};

		for i in (1..num_of_ads + 1).rev() {
			let texture_name = format!("advert{}", i);
			log::debug!("Checking texture {}", &texture_name);
			let name_of_texture = CString::new(texture_name).unwrap_or_default();

			let mut adv_pos = 0;
			let level_ads_data = hook_get_level_ads_data(p_level_resource);
			hook_get_ad_position_on_level(
				level_ads_data as _,
				std::ptr::addr_of_mut!(adv_pos),
				name_of_texture.as_ptr(),
			);

			log::debug!("Ads pos - {:x}", adv_pos);

			let full_path = appdata_amax_path
				.join("ads")
				.join(&level_name)
				.join(format!("advert{}.png", i));
			log::debug!("File path {:?}", &full_path);

			let mut img_data = match std::fs::read(&full_path) {
				Ok(img_data) => img_data,
				Err(e) => {
					log::error!("Failed to read file {:?} - {e}", &full_path);
					continue;
				}
			};

			let size = img_data.len() as u32;

			#[allow(deprecated)]
			let new_texture = crate::dx_tools::d3d9_create_tex_from_mem_ex_v1(d3d9_device, &mut img_data);

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

pub fn hook_get_level_ads_data(level_ptr: *mut PLevelResource) -> *mut PLevelResource {
	GetLevelAdsData.call(level_ptr)
}

pub fn hook_get_ad_position_on_level(
	p_level_resource: *mut PLevelResource,
	out_position: *mut u32,
	texture_name: *const c_char,
) {
	GetAdPositionOnLevel.call(p_level_resource, out_position, texture_name)
}
