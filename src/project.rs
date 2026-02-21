use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path; // убрали PathBuf

#[derive(Debug, Serialize, Deserialize)]
pub struct SpackConfig {
    pub name: String,
    pub version: String,
    pub main: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub output: Option<String>,
}

impl SpackConfig {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let config: SpackConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        Ok(config)
    }

    pub fn output_name(&self) -> String {
        self.output.clone().unwrap_or_else(|| format!("{}.exe", self.name))
    }
}