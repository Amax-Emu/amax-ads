#![allow(unused)]
#![allow(deprecated)]

use std::path::Path;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;

use windows::Win32::Graphics::Direct3D9::IDirect3DDevice9;
use windows::Win32::Graphics::Direct3D9::IDirect3DTexture9;

use crate::dx_tools::d3d9_create_tex_from_mem_ex_v1;
use crate::dx_tools::get_d3d9_device;
use crate::file_utils::get_ads_path;

// I don't like this solution, it has unwrap and fs stuff inside LazyLock:
static G_CACHE: LazyLock<Arc<RwLock<AdCache>>> = LazyLock::new(|| {
	let dev = get_d3d9_device();
	let dir = get_ads_path().unwrap().join("ads");
	let my_cache = AdCache::new(dev, dir);
	Arc::new(RwLock::new(my_cache))
});

#[derive(Debug)]
pub struct AdCache {
	levels: Vec<LevelCache>,
}

impl AdCache {
	fn new<P: AsRef<Path>>(dev: *mut IDirect3DDevice9, dir: P) -> Self {
		let subdirs = dir
			.as_ref()
			.read_dir()
			.unwrap()
			.filter(|dir_entry| dir_entry.as_ref().unwrap().file_type().unwrap().is_dir())
			.map(|subdir| subdir.as_ref().unwrap().path());
		let levels: Vec<LevelCache> = subdirs.map(|subdir| LevelCache::new(dev, subdir)).collect();
		Self { levels }
	}

	fn find_level<'a>(&'a self, level_name: &str) -> Option<&'a LevelCache> {
		self.levels.iter().find(|lvl| lvl.level_name == level_name)
	}

	pub fn g() -> Arc<RwLock<Self>> {
		G_CACHE.clone()
	}

	pub fn get_tex_data(
		&self,
		level_name: &str,
		ad_name: &str,
	) -> Option<(*mut IDirect3DTexture9, u32)> {
		let Some(lvl) = self.find_level(level_name) else {
			return None;
		};
		lvl.find_tex_data(ad_name)
	}
}

#[derive(Debug)]
pub struct LevelCache {
	level_name: String,
	ads: Vec<Ad>,
}

impl LevelCache {
	pub fn new<P: AsRef<Path>>(dev: *mut IDirect3DDevice9, dir: P) -> Self {
		let level_name = dir
			.as_ref()
			.file_name()
			.unwrap()
			.to_str()
			.unwrap()
			.to_string();

		let pngs_in_dir = dir
			.as_ref()
			.read_dir()
			.unwrap()
			.map(|dir_entry| dir_entry.as_ref().unwrap().path())
			.filter(|p| p.extension().is_some_and(|ext| ext == "png"));

		let ads: Vec<Ad> = pngs_in_dir.map(|png| Ad::new(dev, png)).collect();

		Self { level_name, ads }
	}

	fn find_ad<'a>(&'a self, ad_name: &str) -> Option<&'a Ad> {
		self.ads.iter().find(|ad| ad.ad_name == ad_name)
	}

	fn find_tex_data(&self, ad_name: &str) -> Option<(*mut IDirect3DTexture9, u32)> {
		self.find_ad(ad_name).map(|ad| (ad.tex, ad.img_data_len))
	}
}

#[derive(Debug)]
pub struct Ad {
	ad_name: String,
	tex: *mut IDirect3DTexture9,
	img_data_len: u32,
}
unsafe impl Send for Ad {}
unsafe impl Sync for Ad {}

impl Ad {
	pub fn new<P: AsRef<Path>>(dev: *mut IDirect3DDevice9, png_path: P) -> Self {
		let ad_name = png_path
			.as_ref()
			.file_stem()
			.unwrap()
			.to_str()
			.unwrap()
			.to_string();
		let mut data = std::fs::read(&png_path).unwrap();
		let img_data_len = data.len() as u32;
		let tex = d3d9_create_tex_from_mem_ex_v1(dev, &mut data);
		{
			// FIXME
			let pp = png_path.as_ref().display();
			log::trace!("Adding Ad to cache: {pp}");
		}
		Self {
			ad_name,
			tex,
			img_data_len,
		}
	}
}
