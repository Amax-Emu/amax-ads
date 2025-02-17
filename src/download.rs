use std::path::Path;

use crate::file_utils::{
	get_ads_path, get_local_checksum, remove_ads_dir, unzip, write_ads_checksum,
};

pub struct AdsDownloader {}

impl AdsDownloader {
	pub fn resolve() -> Result<(), ()> {
		let ads_dir = get_ads_path().unwrap();
		log::trace!("Resolving if we need to download AMAX Ads from server...");
		let remote_checksum = get_remote_checksum().map_err(|e| {
			log::trace!("Failed to get remote checksum: {e}");
			log::trace!("Can't download because we can't get a remote checksum...");
		})?;

		let Ok(local_checksum) = get_local_checksum(&ads_dir).map_err(|e| {
			log::trace!("Failed to get local checksum: {e}");
			log::trace!("Need download because we don't have a local checksum!");
		}) else {
			return Self::download(&ads_dir, &remote_checksum);
		};
		if local_checksum == remote_checksum {
			log::info!("Server checksum matches local - skipping download.");
			return Ok(());
		}
		log::info!("Server checksum doesn't match local, need download.");
		Self::download(&ads_dir, &remote_checksum)
	}

	pub fn download(ads_dir: &Path, remote_checksum: &str) -> Result<(), ()> {
		let displ = ads_dir.display();
		log::trace!("Removing local ads files...");
		remove_ads_dir(ads_dir).map_err(|e| {
			log::error!("Failed to remove local ads dir [{displ}]: {e}");
		})?;
		log::info!("Downloading latest ads files...");
		let zip_data = download_zip_file_data().map_err(|e| {
			log::error!("Failed download zip for ads: {e}");
		})?;

		unzip(zip_data, ads_dir).map_err(|e| {
			log::error!("Failed unzip downloaded ads: {e}");
		})?;
		write_ads_checksum(ads_dir, remote_checksum).map_err(|e| {
			log::error!("Failed write downloaded ads checksum to disk: {e}");
		})?;
		Ok(())
	}
}

pub fn get_remote_checksum() -> Result<String, reqwest::Error> {
	let url = url_gen("/checksum.file");
	let response = reqwest::blocking::get(url)?;
	let text = response.text()?;
	Ok(text.trim().to_string())
}

pub fn download_zip_file_data() -> Result<Vec<u8>, reqwest::Error> {
	let url = url_gen("/ads.zip");
	let response = reqwest::blocking::get(url)?;
	let file_data = response.bytes()?;
	Ok(file_data.to_vec())
}

fn url_gen(url: &str) -> String {
	const DL_URL_BASE: &str = "https://amax-ads.fra1.cdn.digitaloceanspaces.com";
	std::format!("{DL_URL_BASE}{url}")
}
