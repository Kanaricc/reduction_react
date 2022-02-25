use std::{
    env,
    fs::{self, Permissions},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use data::{PackageTag, VersionTag};
use thiserror::Error;

mod data;
mod utils;

#[derive(Error, Debug)]
pub enum Error {
    #[error("this os `{0}` is not supported")]
    UnsupportedOS(String),
    #[error("failed to execute IO operation: {msg}")]
    CommonFileError {
        msg: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to fetch data from the publisher")]
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
    pub fn new(
        name: impl Into<String>,
        version: impl TryInto<VersionTag>,
        pulishing_url: impl Into<String>,
    ) -> Self {
        let version = version.try_into();
        if version.is_err() {
            panic!("invalid version, please contact the author");
        }
        let version = version.unwrap_or_else(|_| VersionTag::new(0, 0, 0));
        let self_name = env::current_exe().unwrap();
        let mut self_version = self_name.file_name().unwrap().to_str().unwrap().split("-");
        if let Some(name_version) = self_version.nth(1) {
            let name_version: VersionTag = name_version.try_into().unwrap();
            if version != name_version {
                panic!("invalid version, please contact the author");
            }
        }
        Reactor {
            name: name.into(),
            version: version,
            pulishing_url: pulishing_url.into(),
        }
    }

    pub fn oneclick(&self) -> Result<(), Error> {
        self.self_update_if_available()?;
        self.check_update_and_update()?;
        self.self_update_if_available()?;

        Ok(())
    }

    fn check_update_and_update(&self) -> Result<(), Error> {
        let latest_version = self.check_update()?;
        if let CheckUpdateResult::UpdateAvailable(latest_version) = latest_version {
            self.update(&latest_version)?;
        } else {
            println!("{} is up to date", self.name);
        }

        Ok(())
    }

    fn check_update(&self) -> Result<CheckUpdateResult, Error> {
        let resp = reqwest::blocking::get(&self.pulishing_url)?.text()?;
        let package_tag = serde_json::from_str::<data::PackageTag>(&resp)?;
        if package_tag.version > self.version {
            return Ok(CheckUpdateResult::UpdateAvailable(package_tag));
        } else {
            return Ok(CheckUpdateResult::UpToDate);
        }
    }

    fn self_update_if_available(&self) -> Result<(), Error> {
        thread::sleep(Duration::from_secs(1));

        let mut other_version = self.find_other_available_versions()?;
        other_version.sort();
        // remove old versions
        for i in other_version.iter() {
            if i.0 <= self.version {
                fs::remove_file(&i.1).map_err(|err| Error::CommonFileError {
                    msg: format!("failed to remove old version"),
                    source: err,
                })?;
                println!("removed old version: {:?}", i.0);
            }
        }

        // start latest version
        other_version.reverse();
        if let Some(new_version) = other_version.get(0) {
            if new_version.0 > self.version {
                #[cfg(unix)]
                {
                    use std::os::unix::prelude::PermissionsExt;
                    fs::set_permissions(&new_version.1, Permissions::from_mode(0o755)).map_err(|err| Error::CommonFileError {
                        msg: format!("failed to set permission of new version"),
                        source: err,
                    })?;
                }
                println!(
                    "found new local version: {:?}. restarting...",
                    new_version.0
                );
                run_executable_and_quit(new_version.1.canonicalize().unwrap());
            }
        }

        // make self as default executable
        let cur_path = std::env::current_exe().map_err(|err| Error::CommonFileError {
            msg: format!("failed to locate current executable"),
            source: err,
        })?;
        if cur_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .find("-")
            .is_some()
        {
            let new_path = cur_path
                .parent()
                .unwrap()
                .join(utils::get_executable_file_name(&self.name)?);
            fs::copy(&cur_path, &new_path).map_err(|err| Error::CommonFileError {
                msg: format!("failed to set default executable"),
                source: err,
            })?;
            println!("replaced default version. restarting...");
            run_executable_and_quit(new_path.canonicalize().unwrap());
        }

        Ok(())
    }

    fn find_other_available_versions(&self) -> Result<Vec<(VersionTag, PathBuf)>, Error> {
        let paths = fs::read_dir(".").map_err(|err| Error::CommonFileError {
            msg: format!("failed to read current directory"),
            source: err,
        })?;

        let mut result = vec![];
        for path in paths {
            let path = path.map_err(|err| Error::CommonFileError {
                msg: format!("failed to read dir"),
                source: err,
            })?;
            let name = path.file_name();
            let name = name.to_str().unwrap();
            if name.starts_with(&self.name) {
                let file_version = name.split("-").nth(1);
                if let Some(file_version) = file_version {
                    let file_version = file_version.try_into();
                    if let Ok(version_tag) = file_version {
                        result.push((version_tag, path.path()));
                    }
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
        println!("extracted remote package");
        utils::copy(&temp_dir, ".").map_err(|err| Error::CommonFileError {
            msg: format!("failed to copy directories"),
            source: err,
        })?;
        println!("replaced old data with new data");
        std::fs::remove_dir_all(temp_dir).map_err(|err| Error::CommonFileError {
            msg: format!("failed to remove temp directory"),
            source: err,
        })?;
        println!("finish updates");

        Ok(())
    }
}

#[cfg(unix)]
fn run_executable_and_quit(path: impl AsRef<Path>) {
    use std::os::unix::prelude::CommandExt;
    std::process::Command::new(path.as_ref().to_str().unwrap()).exec();
}

#[cfg(windows)]
fn run_executable_and_quit(path: impl AsRef<Path>) {
    std::process::Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path.as_ref().to_str().unwrap())
        .output()
        .unwrap();
    std::process::exit(0);
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
