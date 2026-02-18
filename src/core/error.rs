use std::path::PathBuf;

/// Result type alias for cargo-image-runner operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for cargo-image-runner.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration-related errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Bootloader-related errors.
    #[error("Bootloader error: {0}")]
    Bootloader(String),

    /// Image building errors.
    #[error("Image build error: {0}")]
    ImageBuild(String),

    /// Runner execution errors.
    #[error("Runner error: {0}")]
    Runner(String),

    /// Firmware-related errors.
    #[error("Firmware error: {0}")]
    Firmware(String),

    /// Template processing errors.
    #[error("Template error: {0}")]
    Template(String),

    /// File not found.
    #[error("File not found: {}", .0.display())]
    FileNotFound(PathBuf),

    /// Missing required configuration field.
    #[error("Missing required configuration field: {0}")]
    MissingConfig(String),

    /// Invalid configuration value.
    #[error("Invalid configuration value for {field}: {value}")]
    InvalidConfig { field: String, value: String },

    /// Feature not enabled.
    #[error("Feature '{0}' is not enabled. Enable it in Cargo.toml features.")]
    FeatureNotEnabled(String),

    /// Unsupported combination of options.
    #[error("Unsupported combination: {0}")]
    UnsupportedCombination(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML deserialization error.
    #[error("TOML parsing error: {0}")]
    TomlDe(#[from] toml::de::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Cargo metadata error.
    #[cfg(feature = "cargo-metadata")]
    #[error("Cargo metadata error: {0}")]
    CargoMetadata(#[from] cargo_metadata::Error),

    /// Git error (when limine feature is enabled).
    #[cfg(feature = "limine")]
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
}

impl Error {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    /// Create a bootloader error.
    pub fn bootloader(msg: impl Into<String>) -> Self {
        Error::Bootloader(msg.into())
    }

    /// Create an image build error.
    pub fn image_build(msg: impl Into<String>) -> Self {
        Error::ImageBuild(msg.into())
    }

    /// Create a runner error.
    pub fn runner(msg: impl Into<String>) -> Self {
        Error::Runner(msg.into())
    }

    /// Create a firmware error.
    pub fn firmware(msg: impl Into<String>) -> Self {
        Error::Firmware(msg.into())
    }

    /// Create a template error.
    pub fn template(msg: impl Into<String>) -> Self {
        Error::Template(msg.into())
    }

    /// Create a feature not enabled error.
    pub fn feature_not_enabled(feature: impl Into<String>) -> Self {
        Error::FeatureNotEnabled(feature.into())
    }

    /// Create an unsupported combination error.
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Error::UnsupportedCombination(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        assert_eq!(
            Error::config("bad value").to_string(),
            "Configuration error: bad value"
        );
        assert_eq!(
            Error::bootloader("fetch failed").to_string(),
            "Bootloader error: fetch failed"
        );
        assert_eq!(
            Error::image_build("write error").to_string(),
            "Image build error: write error"
        );
        assert_eq!(
            Error::runner("not found").to_string(),
            "Runner error: not found"
        );
        assert_eq!(
            Error::firmware("missing file").to_string(),
            "Firmware error: missing file"
        );
        assert_eq!(
            Error::template("bad syntax").to_string(),
            "Template error: bad syntax"
        );
        assert_eq!(
            Error::feature_not_enabled("grub").to_string(),
            "Feature 'grub' is not enabled. Enable it in Cargo.toml features."
        );
        assert_eq!(
            Error::unsupported("bios + fat").to_string(),
            "Unsupported combination: bios + fat"
        );
    }

    #[test]
    fn test_error_file_not_found() {
        let err = Error::FileNotFound(PathBuf::from("/missing/file.txt"));
        assert_eq!(err.to_string(), "File not found: /missing/file.txt");
    }

    #[test]
    fn test_error_missing_config() {
        let err = Error::MissingConfig("boot.type".to_string());
        assert_eq!(
            err.to_string(),
            "Missing required configuration field: boot.type"
        );
    }

    #[test]
    fn test_error_invalid_config() {
        let err = Error::InvalidConfig {
            field: "boot.type".to_string(),
            value: "unknown".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid configuration value for boot.type: unknown"
        );
    }
}
