use log::{debug, error, warn};
use std::fs::File;
use std::io::Write as io_write;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub fn get_remote_checksum() -> Option<String> {
    let url = url_gen("/checksum.file");

    let response = match reqwest::blocking::get(url) {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to get checksum.file - {e}");
            return None;
        }
    };

    match response.text() {
        Ok(version_string) => {
            let temp = version_string.replace("\n", "");
            Some(temp)
        }
        Err(e) => {
            error!("Failed to convert checksum to text - {e}");
            return None;
        }
    }
}

pub fn download_ads_zip(path: PathBuf) -> Option<PathBuf> {
    let url = url_gen("/ads.zip");

    let ads_zip_path = path.join("ads.zip");

    download_file(&url, &ads_zip_path)
}

pub fn download_file(url: &str, path: &Path) -> Option<PathBuf> {
    // Reqwest setup
    let response = match reqwest::blocking::get(url) {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to download file - {e}");
            return None;
        }
    };

    let file_data = match response.bytes() {
        Ok(data) => data.to_vec(),
        Err(e) => {
            error!("Failed to convert response to bytes - {e}");
            return None;
        }
    };

    let mut file = match File::create(path) {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to create file - {e}");
            return None;
        }
    };

    let _ = file.write(&file_data);

    return Some(path.to_path_buf());
}

fn url_gen(url: &str) -> String {
    format!("https://amax-ads.fra1.cdn.digitaloceanspaces.com{url}")
}

pub fn remove_ads_dir(appdata_amax_path: &Path) {
    let ads_path = appdata_amax_path.join("ads");
    let _ = fs::remove_dir(ads_path);
}

pub fn write_ads_checksum(appdata_amax_path: &Path, checksum: String) {
    let ads_checksum_path = appdata_amax_path.join("ads").join("checksum.file");
    if let Ok(mut file) = fs::File::create(ads_checksum_path) {
        let _ = file.write(checksum.as_bytes());
    }
}

pub fn unpack_ads(path: PathBuf) -> bool {
    let fname = path;

    let base_path = match fname.parent() {
        Some(path) => path.to_owned(),
        None => PathBuf::new(),
    };

    let file = fs::File::open(fname).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => {
                let temp_path = base_path.join(path);
                temp_path
            }
            None => continue,
        };

        debug!("{}", outpath.display());

        if (*file.name()).ends_with('/') {
            debug!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            debug!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );

            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    true
}

pub fn get_local_checksum(appdata_amax_path: &Path) -> Option<String> {
    let checksum_path = appdata_amax_path.join("ads").join("checksum.file");

    match fs::read_to_string(checksum_path) {
        Ok(local_version) => Some(local_version),
        Err(e) => {
            warn!("Failed to get local checksum - {e}");
            return None;
        }
    }
}

// pub fn apply_update(update_path: PathBuf) {
//     info!("Applying an update...");
//     for file_path in &self.base_updated_files {

//         let temp = &self.temp_path.join(file_path);

//         match fs::metadata(temp){
//             Ok(metadata) => {
//                 if metadata.is_dir() {
//                     continue;
//                 }
//             },
//             Err(_) => continue,
//         };

//         info!("{}",&self.game_path.join(file_path).display());

//         let _ = fs::copy(
//             &self.temp_path.join(file_path),
//             &self.game_path.join(file_path),
//         );
//     }

//     let mut version_file =
//         fs::File::create(&self.game_path.join("version")).unwrap();
//     io::copy(&mut self.remote_version.as_bytes(), &mut version_file).unwrap();
//     info!("Done!");
// }
