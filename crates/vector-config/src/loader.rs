//! TOML loader + profile resolver. Task 2 fills the bodies; Task 1 ships only surface.

use crate::{error::ConfigError, schema::ConfigFile};

#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub name: String,
    pub block: crate::schema::ProfileBlock,
}

pub fn parse(_source: &str) -> Result<ConfigFile, ConfigError> {
    unimplemented!("Plan 05-02 Task 2 lands this")
}

pub fn resolve_profile(_cfg: &ConfigFile, _name: &str) -> ResolvedProfile {
    unimplemented!("Plan 05-02 Task 2 lands this")
}
