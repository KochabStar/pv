use crate::error::{PvError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactPackageSpec {
    pub name: String,
    pub version: String,
}

impl PackageSpec {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(PvError::InvalidPackageSpec(input.to_string()));
        }

        let parts: Vec<&str> = trimmed.split('@').collect();
        match parts.as_slice() {
            [name] if !name.is_empty() => Ok(Self {
                name: (*name).to_string(),
                version: None,
            }),
            [name, version] if !name.is_empty() && !version.is_empty() => Ok(Self {
                name: (*name).to_string(),
                version: Some((*version).to_string()),
            }),
            _ => Err(PvError::InvalidPackageSpec(input.to_string())),
        }
    }
}

impl ExactPackageSpec {
    pub fn parse(input: &str) -> Result<Self> {
        let spec = PackageSpec::parse(input)?;
        let version = spec
            .version
            .ok_or_else(|| PvError::InvalidPackageSpec(input.to_string()))?;

        Ok(Self {
            name: spec.name,
            version,
        })
    }
}
