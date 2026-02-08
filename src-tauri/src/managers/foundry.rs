use std::process::Command;
use std::path::PathBuf;
use serde_json::Value;
use crate::error::Result;

pub struct FoundryManager;

impl FoundryManager {
    /// Check if Foundry is installed by looking for the executable and checking it's in PATH
    pub fn is_installed() -> bool {
        Command::new("foundry")
            .arg("--help")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get the path to Foundry executable (Placeholder, not actively used in current design but useful)
    pub fn get_executable_path() -> Option<PathBuf> {
        // Check common installation locations on Windows primarily.
        #[cfg(target_os = "windows")]
        let locations = vec![
            "C:\\Program Files\\FoundryLocal\\foundry.exe",
            "C:\\Program Files (x86)\\FoundryLocal\\foundry.exe",
            // Add other common paths if known
        ];
        #[cfg(not(target_os = "windows"))]
        let locations: Vec<&str> = vec![]; // Foundry Local is primarily Windows

        locations.iter()
            .map(PathBuf::from)
            .find(|path| path.exists())
    }

    /// Check if Foundry service is running based on `foundry service list` output
    pub fn is_service_running() -> Result<bool> {
        let output = Command::new("foundry")
            .args(&["service", "list"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Consider specific error messages for "not running" vs "command failed"
            if stderr.contains("service is not running") || stderr.contains("not responding") {
                return Ok(false);
            }
            return Err(format!("Foundry 'service list' command failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // If output contains any active service, assume it's running
        Ok(stdout.contains("localhost:") || stdout.contains("Service running"))
    }

    /// Start Foundry service using `foundry service start`
    pub fn start_service() -> Result<()> {
        log::info!("Attempting to start Foundry service...");
        let output = Command::new("foundry")
            .args(&["service", "start"])
            // Detach the process or run in background if necessary
            // For now, let's assume it waits or is quick
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to start Foundry service: {}", stderr).into());
        }

        // Wait a moment for service to initialize and models to load
        std::thread::sleep(std::time::Duration::from_secs(5)); // Increased sleep for robustness
        log::info!("Foundry service start command executed. Output: {}", String::from_utf8_lossy(&output.stdout));

        Ok(())
    }

    /// Get endpoint URL and model ID from `foundry service list` output
    pub fn get_endpoint_info() -> Result<(String, String)> {
        let output = Command::new("foundry")
            .args(&["service", "list"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Foundry 'service list' command failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        log::debug!("Foundry service list output: {}", stdout);

        let endpoint = extract_endpoint_url(&stdout)?;
        let model_id = extract_model_id(&stdout)?;

        Ok((endpoint, model_id))
    }

    /// Run a specific model with Foundry, e.g., `foundry model run phi-3.5-mini --retain`
    pub fn run_model(model_name: &str) -> Result<()> {
        log::info!("Attempting to run Foundry model '{}'...", model_name);
        // The '--retain' flag is important to keep the model loaded after the command exits.
        let output = Command::new("foundry")
            .args(&["model", "run", model_name, "--retain"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to run Foundry model '{}': {}", model_name, stderr).into());
        }
        log::info!("Foundry model '{}' run command executed. Output: {}", model_name, String::from_utf8_lossy(&output.stdout));
        Ok(())
    }

    /// Get available models from Foundry
    pub fn get_available_models() -> Result<Vec<String>> {
        let output = Command::new("foundry")
            .args(&["model", "list"]) // Assuming 'foundry model list' shows available models
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Foundry 'model list' command failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        log::debug!("Foundry model list output: {}", stdout);

        // This parsing might need adjustment based on actual output.
        // Assuming output lists model names line by line or in a simple parseable format.
        let models: Vec<String> = stdout.lines()
            .filter(|line| !line.trim().is_empty() && !line.contains("NAME") && !line.contains("--------")) // Filter header/separator
            .map(|line| line.split_whitespace().next().unwrap_or("").to_string()) // Take first word as model name
            .filter(|s| !s.is_empty())
            .collect();

        Ok(models)
    }
}


/// Helper: extract URL from service list output
fn extract_endpoint_url(output: &str) -> Result<String> {
    for line in output.lines() {
        if line.contains("localhost:") && line.contains("http://") {
            // Updated regex for robustness
            let re = regex::Regex::new(r"(http://localhost:\d+/v\d+)").unwrap();
            if let Some(captures) = re.captures(line) {
                return Ok(captures[1].to_string());
            }
        }
    }
    Err("Could not find endpoint URL in Foundry service list output".into())
}

/// Helper: extract model ID from service list output
fn extract_model_id(output: &str) -> Result<String> {
    // Look for lines that contain model information, e.g., "Model Name: phi-3.5-mini" or directly "phi-3.5-mini"
    for line in output.lines() {
        if line.to_lowercase().contains("phi") || line.to_lowercase().contains("model") {
            // This regex will try to find a pattern like "model_name: some-model" or just "model_name"
            let re = regex::Regex::new(r"([a-zA-Z0-9_\-\.]+:\d+)|(phi-[0-9\.]+-[a-z]+)").unwrap();
            if let Some(captures) = re.captures(line) {
                if let Some(model_id_match) = captures.get(0) { // Get the entire matched string
                    return Ok(model_id_match.as_str().to_string());
                }
            } else if line.contains("Model: ") {
                if let Some(model_part) = line.split("Model: ").nth(1) {
                    return Ok(model_part.trim().to_string());
                }
            }
        }
    }
    Err("Could not find model ID in Foundry service list output. Ensure a model is loaded and 'foundry service list' returns its ID.".into())
}
