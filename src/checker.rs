use crate::{data, Error};

pub struct UpdateChecker<'a> {
    version: data::VersionTag,
    pulishing_url: &'a str,
}

pub enum CheckUpdateResult {
    UpToDate,
    UpdateAvailable(data::PackageTag),
}


impl<'a> UpdateChecker<'a> {
    pub fn new(current_version: data::VersionTag, publishing_url: &'a str) -> Self {
        Self {
            version: current_version,
            pulishing_url: publishing_url,
        }
    }

    pub fn get_latest_package_tag(&self)->Result<data::PackageTag,Error>{
        let resp = reqwest::blocking::get(self.pulishing_url)?.text()?;
        Ok(serde_yaml::from_str::<data::PackageTag>(&resp)?)
    }

    pub fn get_latest_version(&self)->Result<data::VersionTag,Error>{
        let package_tag=self.get_latest_package_tag()?;
        Ok(package_tag.version)
    }

    pub fn check_update(&self)->Result<CheckUpdateResult,Error>{
        let package_tag = self.get_latest_package_tag()?;
        if package_tag.version > self.version {
            return Ok(CheckUpdateResult::UpdateAvailable(package_tag));
        } else {
            return Ok(CheckUpdateResult::UpToDate);
        }
    }
}
