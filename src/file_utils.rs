use std::io::{Cursor, Write as io_write};
use std::path::{Path, PathBuf};

pub fn get_ads_path() -> Result<PathBuf, std::io::Error> {
	let dir = known_folders::get_known_folder_path(known_folders::KnownFolder::RoamingAppData)
		.ok_or_else(|| std::io::Error::other("Couldn't get FOLDERID_RoamingAppData (defaut: %USERPROFILE%\\AppData\\Roaming [%APPDATA%]) from system"))?
		.join("bizarre creations")
		.join("blur")
		.join("amax")
		.join("amax_ads");
	if !&dir.is_dir() {
		std::fs::create_dir_all(&dir)?;
	};
	Ok(dir)
}

pub fn get_local_checksum(ads_dir: &Path) -> Result<String, std::io::Error> {
	let checksum_path = ads_dir.join("checksum.file");
	std::fs::read_to_string(checksum_path)
}

pub fn remove_ads_dir(ads_dir: &Path) -> Result<(), std::io::Error> {
	std::fs::remove_dir_all(ads_dir)
}

pub fn write_ads_checksum(
	ads_dir: &Path,
	checksum: &str,
) -> Result<(), std::io::Error> {
	let src = ads_dir.join("checksum.file");
	let mut file = std::fs::File::create(src)?;
	let _ = file.write(checksum.as_bytes())?;
	Ok(())
}

pub fn unzip(data: Vec<u8>, dst: &Path) -> Result<(), std::io::Error> {
	let mut data = Cursor::new(data);
	let mut archive = zip::ZipArchive::new(&mut data)?;

	for idx in 0..archive.len() {
		let mut file = archive.by_index(idx)?;
		let Some(extracted_file) = file.enclosed_name() else {
			continue;
		};
		let extracted_file_dst = dst.join(extracted_file);
		let extracted_displ = extracted_file_dst.display();
		if file.is_dir() {
			std::fs::create_dir_all(&extracted_file_dst)?;
			log::debug!("Created {extracted_displ} from zip archive[{idx}]");
			continue;
		}
		if let Some(p) = extracted_file.parent() {
			if !p.exists() {
				std::fs::create_dir_all(p)?;
			}
		}
		let size = file.size();
		let mut outfile = std::fs::File::create(&extracted_file_dst)?;
		std::io::copy(&mut file, &mut outfile)?;
		log::debug!("Created file {extracted_displ} ({size} bytes) from zip archive[{idx}]");
	}
	Ok(())
}
