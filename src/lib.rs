use std::{
    io::{self, Write},
    os::unix::prelude::AsRawFd,
    path::{Path, PathBuf},
};

use data::PackageTag;
use thiserror::Error;

mod data;
mod self_opt;

#[derive(Error, Debug)]
pub enum Error {
    #[error("this os `{0}` is not supported")]
    UnsupportedOS(String),
    #[error("failed to create script")]
    ScriptIo {
        #[from]
        source: std::io::Error,
    },
    #[error("failed to fetch data from the Internet")]
    NetError {
        #[from]
        source: reqwest::Error,
    },
    #[error("failed to parse data from the publisher")]
    ParsingError {
        #[from]
        source: serde_json::Error,
    },
    #[error("failed to extract data")]
    ZipError {
        #[from]
        source: zip::result::ZipError,
    },
}

pub struct Reactor {
    version: data::VersionTag,
    pulishing_url: String,
}

pub enum CheckUpdateResult {
    UpToDate,
    UpdateAvailable(data::PackageTag),
}

impl Reactor {
    pub fn new(version: data::VersionTag, pulishing_url: String) -> Self {
        Reactor {
            version,
            pulishing_url,
        }
    }

    pub fn check_update(&self) -> Result<CheckUpdateResult, Error> {
        let resp = reqwest::blocking::get(&self.pulishing_url)?.text()?;
        let package_tag = serde_json::from_str::<data::PackageTag>(&resp)?;
        if package_tag.version > self.version {
            return Ok(CheckUpdateResult::UpdateAvailable(package_tag));
        } else {
            return Ok(CheckUpdateResult::UpToDate);
        }
    }

    pub fn update(&self, package_tag: &PackageTag) -> Result<(), Error> {
        let resp = reqwest::blocking::get(&package_tag.download_url)?.bytes()?;
        let path = PathBuf::from(format!("temp-{}.zip", package_tag.version.as_string()));
        let mut file = std::fs::File::create(&path)?;
        file.write_all(&resp)?;
        drop(file);
        extract_zip(&path, PathBuf::from("./temp"))?;

        Ok(())
    }
}

fn extract_zip(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<(), Error> {
    let src = src.as_ref();
    let dest= dest.as_ref();
    if !dest.exists() {
        std::fs::create_dir_all(&dest)?;
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&src)?)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if let Some(relative_path) = file.enclosed_name() {
            let out_path = dest.join(relative_path);
            println!("{}", out_path.display());
            let out_dir = out_path.parent().unwrap();
            if !out_dir.exists() {
                std::fs::create_dir_all(out_dir)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            io::copy(&mut file, &mut out_file)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_unzip() {
        let path = PathBuf::from("test.zip");
        let dest = PathBuf::from("./test");
        extract_zip(&path, &dest).unwrap();
    }
}
