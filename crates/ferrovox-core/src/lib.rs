//! Core types shared across FerroVox: identifiers, config, runtime setup.

pub mod config;
pub mod runtime;

pub use config::ServerConfig;
pub use runtime::build_runtime;

/// Protocol/version pins for the two editions FerroVox targets.
///
/// Update these as new game versions are tracked. Keeping them in one
/// place makes it trivial to see (and bump) what the server currently
/// speaks.
pub mod versions {
    /// Java Edition release name this build targets.
    pub const JAVA_EDITION_NAME: &str = "26.3";
    /// Java Edition protocol version number for 26.3.
    pub const JAVA_PROTOCOL_VERSION: i32 = 777;
    /// Java Edition data version for 26.3.
    pub const JAVA_DATA_VERSION: i32 = 5023;

    /// Bedrock Edition release name this build targets.
    pub const BEDROCK_EDITION_NAME: &str = "26.52.3";
    /// Bedrock Edition network protocol version.
    pub const BEDROCK_PROTOCOL_VERSION: i32 = 2193;
}
