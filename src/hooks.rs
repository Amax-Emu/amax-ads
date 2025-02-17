use std::{
	ffi::{c_char, c_void, CStr, CString},
	str::FromStr,
};

use retour::static_detour;

use crate::{
	advert_manager::{AdvertManager, AdvertTexture, PLevelResource},
	cache::AdCache,
	download::AdsDownloader,
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
	std::thread::Builder::new()
		.name("AMAX-ADS-Downloader".to_string())
		.spawn(|| {
			match AdsDownloader::resolve() {
				Ok(_) => log::info!("AdsDownloader::resolve() OK."),
				Err(_) => log::error!("AdsDownloader::resolve() failed!"),
			}
			AdCache::g();
		})
		.expect("failed to created amax ads downloader thread");
	AdvertManager_InitializeSystem.call(advert_manager);
}

pub fn hook_enter_zone_post_load(
	advert_manager: *mut AdvertManager,
	p_level_resource: *mut PLevelResource,
	p_level_instance: *mut c_void,
) {
	// return unsafe { EnterZone_PostLoad.call(advert_manager, p_level_resource, p_level_instance) };
	unsafe {
		(*advert_manager).level_instance_ptr = p_level_instance;

		let level_name_full = CStr::from_ptr((*p_level_resource).level_name.as_ptr())
			.to_str()
			.unwrap_or_default();

		log::trace!("level_name_full: \"{level_name_full}\"");

		let level_name = level_name_full
			.trim_start_matches(".\\levels\\")
			.trim_end_matches("\\level.level");
		log::trace!("level_name: {level_name}");

		let ad_count = (*advert_manager).ad_count;
		log::trace!("ad_count: {ad_count}");
		if ad_count < 1 {
			return;
		}

		let cache = AdCache::g();
		let cache_read = cache.read().unwrap();

		let mut j: isize = 0;
		for idx in (1..=ad_count).rev() {
			let texture_name = format!("advert{}", idx);
			let name_of_texture = CString::from_str(&texture_name).unwrap();
			log::trace!("texture_name: {texture_name}");

			let Some((tex_ptr, size)) = cache_read.get_tex_data(level_name, &texture_name) else {
				log::warn!("{level_name}/{texture_name} not found in cache?");
				continue;
			};

			let mut adv_pos = 0;
			let level_ads_data = hook_get_level_ads_data(p_level_resource);
			hook_get_ad_position_on_level(level_ads_data, &mut adv_pos, name_of_texture.as_ptr());

			log::debug!("adv_pos = {adv_pos}");

			let temp = AdvertTexture {
				unk1: [0; 0xC],
				size: size,
				zero: 0,
				ptr_to_dx_texture: tex_ptr,
				unk_id: adv_pos,
				size_x10: 0x10,
				texture_id: idx as u16,
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
