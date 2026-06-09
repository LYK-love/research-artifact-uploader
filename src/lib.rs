pub mod archive;
pub mod cli;
pub mod collect;
pub mod gitinfo;
pub mod manifest;
pub mod metadata;
pub mod oss;
pub mod paths;
pub mod records;
pub mod subprocess_utils;

pub type AppResult<T> = Result<T, String>;
