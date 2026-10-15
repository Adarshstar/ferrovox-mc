use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level server configuration, loaded from `ferrovox.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub java: JavaSection,
    #[serde(default)]
    pub bedrock: BedrockSection,
    #[serde(default)]
    pub performance: PerformanceSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default = "default_true")]
    pub online_mode: bool,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            motd: default_motd(),
            max_players: default_max_players(),
            online_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_java_bind")]
    pub bind: String,
    #[serde(default = "default_java_port")]
    pub port: u16,
}

impl Default for JavaSection {
    fn default() -> Self {
        Self {
            enabled: true,
            bind: default_java_bind(),
            port: default_java_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_bedrock_bind")]
    pub bind: String,
    #[serde(default = "default_bedrock_port")]
    pub port: u16,
}

impl Default for BedrockSection {
    fn default() -> Self {
        Self {
            enabled: true,
            bind: default_bedrock_bind(),
            port: default_bedrock_port(),
        }
    }
}

/// Hardware / scheduling tuning knobs. Defaults auto-detect at runtime
/// (0 means "use all available cores").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSection {
    #[serde(default)]
    pub worker_threads: usize,
    #[serde(default = "default_true")]
    pub pin_threads: bool,
}

impl Default for PerformanceSection {
    fn default() -> Self {
        Self {
            worker_threads: 0,
            pin_threads: true,
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_motd() -> String {
    "A FerroVox Server".to_string()
}
fn default_max_players() -> u32 {
    100
}
fn default_java_bind() -> String {
    "0.0.0.0".to_string()
}
fn default_java_port() -> u16 {
    25565
}
fn default_bedrock_bind() -> String {
    "0.0.0.0".to_string()
}
fn default_bedrock_port() -> u16 {
    19132
}

impl ServerConfig {
    /// Load configuration from a TOML file, falling back to defaults for
    /// any missing fields. Creates the file with defaults if it does not
    /// exist yet.
    pub fn load_or_create(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let cfg = Self::default();
            let toml = toml::to_string_pretty(&cfg)?;
            std::fs::write(path, toml)?;
            return Ok(cfg);
        }
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }
}
