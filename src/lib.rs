use std::{fs, path::PathBuf};

use data::{PackageTag, VersionTag};
use thiserror::Error;

mod data;
mod utils;

#[derive(Error, Debug)]
pub enum Error {
    #[error("this os `{0}` is not supported")]
    UnsupportedOS(String),
    #[error("failed to execute IO operation")]
    FileError {
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
    name: String,
    version: data::VersionTag,
    pulishing_url: String,
}

pub enum CheckUpdateResult {
    UpToDate,
    UpdateAvailable(data::PackageTag),
}

impl Reactor {
    pub fn new(name: String, version: data::VersionTag, pulishing_url: String) -> Self {
        Reactor {
            name,
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

    pub fn self_update_if_available(&self) -> Result<(), Error> {
        let mut other_version = self.find_other_available_versions()?;
        other_version.sort();
        // remove old versions
        for i in other_version.iter() {
            if i.0 <= self.version {
                println!("remove old version: {:?}", i.0);
                fs::remove_file(&i.1)?;
            }
        }

        // start latest version
        other_version.reverse();
        if let Some(new_version) = other_version.get(0) {
            if new_version.0 > self.version {
                println!("find new local version: {:?}. restarting...", new_version.0);
                std::process::Command::new(&new_version.1).spawn().unwrap();
                std::process::exit(0);
            }
        } else {
            // make self as default executable
            let cur_path = std::env::current_exe()?;
            if cur_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .find("-")
                .is_some()
            {
                println!("replace default version. restarting...");
                let new_path = cur_path
                    .parent()
                    .unwrap()
                    .join(utils::get_executable_file_name(&self.name)?);
                fs::copy(&cur_path, &new_path)?;
                std::process::Command::new(new_path).spawn().unwrap();
                std::process::exit(0);
            }
        }

        Ok(())
    }

    fn find_other_available_versions(&self) -> Result<Vec<(VersionTag, PathBuf)>, Error> {
        let paths = fs::read_dir(".")?;

        let mut result = vec![];
        for path in paths {
            let path = path?;
            let name = path.file_name();
            let name = name.to_str().unwrap();
            if name.starts_with(&self.name) {
                let file_version = name.split(".").nth(0).unwrap().split("-").nth(1);
                if let Some(file_version) = file_version {
                    result.push((file_version.into(), path.path()));
                }
            }
        }
        Ok(result)
    }

    pub fn update(&self, package_tag: &PackageTag) -> Result<(), Error> {
        // update lib
        utils::download_file(&package_tag.download_url, "temp.zip")?;
        // TODO: check hash
        let temp_dir = PathBuf::from("./temp");
        utils::extract_zip("temp.zip", &temp_dir)?;
        utils::copy(&temp_dir, ".")?;
        std::fs::remove_dir_all(temp_dir)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_unzip() {
        let path = PathBuf::from("test.zip");
        let dest = PathBuf::from("./test");
        utils::extract_zip(&path, &dest).unwrap();
        std::fs::remove_dir_all("./test").unwrap();
    }
}
