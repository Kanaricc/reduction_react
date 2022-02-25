use std::cmp::Ordering;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageTag{
    pub version:VersionTag,
    pub hash: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
}

impl PackageTag{
    pub fn new(version: VersionTag, hash: String, download_url: String) -> Self{
        PackageTag{
            version,
            hash,
            download_url,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq,Eq)]
pub struct VersionTag{
    major: u32,
    minor: u32,
    patch: u32,
}

impl PartialOrd for VersionTag{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.major!=other.major{
            return Some(self.major.cmp(&other.major));
        }
        if self.minor!=other.minor{
            return Some(self.minor.cmp(&other.minor));
        }
        if self.patch!=other.patch{
            return Some(self.patch.cmp(&other.patch));
        }
        Some(Ordering::Equal)
    }
}

impl Ord for VersionTag{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::max_by(self, other, Ord::cmp)
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::min_by(self, other, Ord::cmp)
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
    {
        assert!(min <= max);
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

impl VersionTag{
    pub fn new(major: u32, minor: u32, patch: u32) -> Self{
        VersionTag{
            major,
            minor,
            patch,
        }
    }

    pub fn as_string(&self)->String{
        format!("{}.{}.{}",self.major,self.minor,self.patch)
    }
}

impl From<String> for VersionTag{
    fn from(s: String) -> Self{
        let mut parts = s.split(".");
        let major = parts.next().unwrap().parse::<u32>().unwrap();
        let minor = parts.next().unwrap().parse::<u32>().unwrap();
        let patch = parts.next().unwrap().parse::<u32>().unwrap();
        VersionTag::new(major, minor, patch)
    }
}

impl TryFrom<&str> for VersionTag{
    type Error = std::num::ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split(".");
        if parts.clone().count()<3{
            "3q".parse::<u32>()?;
        }
        let major = parts.next().unwrap().parse::<u32>()?;
        let minor = parts.next().unwrap().parse::<u32>()?;
        let patch = parts.next().unwrap().parse::<u32>()?;
        Ok(VersionTag::new(major, minor, patch))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn test_cmp(){
        let v1 = VersionTag::new(1,2,3);
        let v2 = VersionTag::new(1,2,3);
        let v3 = VersionTag::new(1,2,4);
        let v4 = VersionTag::new(1,3,3);
        let v5 = VersionTag::new(2,2,3);

        assert!(v1==v2);
        assert!(v1<=v2);
        assert!(v1>=v2);
        assert!(v1<v3);
        assert!(v1<v4);
        assert!(v1<v5);
    }
}