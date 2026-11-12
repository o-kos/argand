//! Package and source identity of the running binary.

pub const NAME: &str = "Argand";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const HASH: &str = env!("ARGAND_GIT_HASH");
pub const DATE: &str = env!("ARGAND_GIT_DATE");
pub const TAG: &str = env!("ARGAND_GIT_TAG");

pub fn display_date(date: &str) -> String {
    // Git supplies RFC 3339; keep the offset and unavailable labels intact.
    date.replacen('T', " ", 1)
}

#[cfg(test)]
#[path = "build_metadata_test.rs"]
mod tests;
