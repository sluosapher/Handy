use std::process::Command;
use std::path::PathBuf;
use std::process::Output;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct FoundryManager;

impl FoundryManager {
    fn build_foundry_command() -> Command {
        if let Some(path) = Self::get_executable_path() {
            Command::new(path)
        } else {
            Command::new("foundry")
        }
    }

    fn run_foundry_command(args: &[&str]) -> Result<Output> {
        log::info!("Foundry CLI request: foundry {}", args.join(" "));
        let output = Self::build_foundry_command().args(args).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::info!(
            "Foundry CLI response: status={} stdout={} stderr={}",
            output.status,
            stdout.trim(),
            stderr.trim()
        );
        Ok(output)
    }

    /// Check if Foundry is installed by looking for the executable and checking it's in PATH
    pub fn is_installed() -> bool {
        Self::build_foundry_command()
            .arg("--help")
            .output()
            .map(|output| output.status.success())
            .unwrap_or_else(|_| Self::get_executable_path().is_some())
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

    pub fn get_version() -> Result<String> {
        let output = Self::build_foundry_command()
            .arg("--version")
            .output()
            .or_else(|_| {
                Self::get_executable_path()
                    .ok_or_else(|| {
                        Box::<dyn std::error::Error + Send + Sync>::from(
                            "Foundry executable not found",
                        )
                    })
                    .and_then(|path| {
                        Command::new(path)
                            .arg("--version")
                            .output()
                            .map_err(|e| e.into())
                    })
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Foundry '--version' command failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = stdout.lines().next().unwrap_or("").trim();
        if version.is_empty() {
            return Err("Foundry '--version' output was empty".into());
        }

        Ok(version.to_string())
    }

    pub fn install_foundry_local() -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            if Self::is_installed() {
                return Self::get_version();
            }

            log::info!("Installing Microsoft Foundry Local via winget...");
            let output = Command::new("winget")
                .args(&["install", "Microsoft.FoundryLocal"])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("winget install failed: {}", stderr).into());
            }

            Self::get_version()
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Foundry Local installation is only supported on Windows.".into())
        }
    }

    /// Check if Foundry service is running based on `foundry service list` output
    pub fn is_service_running() -> Result<bool> {
        let output = Self::run_foundry_command(&["service", "status"])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Consider specific error messages for "not running" vs "command failed"
            if stderr.contains("service is not running") || stderr.contains("not responding") {
                return Ok(false);
            }
            return Err(format!("Foundry 'service list' command failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.to_lowercase().contains("not running") {
            return Ok(false);
        }

        Ok(true)
    }

    /// Start Foundry service using `foundry service start`
    pub fn start_service() -> Result<()> {
        log::info!("Attempting to start Foundry service...");
        let output = Self::run_foundry_command(&["service", "start"])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to start Foundry service: {}", stderr).into());
        }

        // Wait a moment for service to initialize and models to load
        std::thread::sleep(std::time::Duration::from_secs(5)); // Increased sleep for robustness
        log::info!("Foundry service start command executed. Output: {}", String::from_utf8_lossy(&output.stdout));

        Ok(())
    }

    /// Start Foundry service, but don't block indefinitely.
    /// Returns Ok(()) if the command exits successfully or times out.
    pub fn start_service_with_timeout(timeout: std::time::Duration) -> Result<()> {
        log::info!("Attempting to start Foundry service (timeout {:?})...", timeout);
        let mut child = Command::new("foundry")
            .args(&["service", "start"])
            .spawn()?;

        let start = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err(format!("Failed to start Foundry service: {}", status).into());
                }
                return Ok(());
            }

            if start.elapsed() >= timeout {
                log::warn!(
                    "Foundry service start did not exit within {:?}; continuing to poll status.",
                    timeout
                );
                return Ok(());
            }

            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }

    /// Get endpoint URL and model ID from `foundry service list` output
    pub fn get_endpoint_info() -> Result<(String, String)> {
        let endpoint = Self::get_endpoint_url()?;
        let model_id = Self::get_model_id_with_retry(10, std::time::Duration::from_secs(2))?;

        Ok((endpoint, model_id))
    }

    /// Load a specific model with Foundry, e.g., `foundry model load phi-4-mini`
    pub fn run_model(model_name: &str) -> Result<()> {
        log::info!("Attempting to load Foundry model '{}'...", model_name);
        let output = Self::run_foundry_command(&["model", "load", model_name])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to run Foundry model '{}': {}", model_name, stderr).into());
        }
        log::info!("Foundry model '{}' run command executed. Output: {}", model_name, String::from_utf8_lossy(&output.stdout));
        Ok(())
    }

    /// Get available models from Foundry
    pub fn get_available_models() -> Result<Vec<String>> {
        let output = Self::run_foundry_command(&["model", "list"])?; // Assuming 'foundry model list' shows available models

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

    /// Download a specific model with Foundry, e.g., `foundry model download phi-4-mini`
    pub fn download_model(model_name: &str) -> Result<()> {
        log::info!("Attempting to download Foundry model '{}'...", model_name);
        let output = Self::run_foundry_command(&["model", "download", model_name])?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to download Foundry model '{}': {}", model_name, stderr).into());
        }

        log::info!(
            "Foundry model '{}' download command executed. Output: {}",
            model_name,
            String::from_utf8_lossy(&output.stdout)
        );
        Ok(())
    }

    /// Ensure model is downloaded before attempting to load it.
    pub fn ensure_model_downloaded(model_name: &str) -> Result<()> {
        if Self::is_model_cached(model_name)? {
            return Ok(());
        }

        Self::download_model(model_name)?;
        Ok(())
    }

    /// Check cache list to see if a model is already downloaded.
    pub fn is_model_cached(model_name: &str) -> Result<bool> {
        let output = Self::run_foundry_command(&["cache", "list"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.to_lowercase().contains("not running")
                || stderr.to_lowercase().contains("not running")
            {
                return Ok(false);
            }
            return Err(format!("Foundry 'cache list' command failed: {}", stderr).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .any(|line| line.to_lowercase().contains(&model_name.to_lowercase())))
    }

    pub fn wait_for_service_ready(
        attempts: usize,
        delay: std::time::Duration,
    ) -> Result<()> {
        for attempt in 1..=attempts {
            let status_output = Self::run_foundry_command(&["service", "status"])?;
            if !status_output.status.success() {
                let stderr = String::from_utf8_lossy(&status_output.stderr);
                return Err(format!(
                    "Foundry 'service status' command failed: {}",
                    stderr
                )
                .into());
            }

            let stdout = String::from_utf8_lossy(&status_output.stdout).to_lowercase();
            if stdout.contains("not running") {
                return Err("Foundry service is not running.".into());
            }

            if !stdout.contains("in progress") && stdout.contains("running on") {
                return Ok(());
            }

            if attempt < attempts {
                std::thread::sleep(delay);
            }
        }

        Err("Foundry service did not become ready in time.".into())
    }

    pub fn ensure_model_loaded(model_name: &str) -> Result<String> {
        // Try loading the model and polling the service list until it appears.
        for attempt in 1..=3 {
            Self::run_model(model_name)?;
            if let Ok(model_id) =
                Self::get_model_id_with_retry(10, std::time::Duration::from_secs(2))
            {
                return Ok(model_id);
            }

            if attempt < 3 {
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }

        Err("Failed to load Foundry model after multiple attempts.".into())
    }

    pub fn get_endpoint_url() -> Result<String> {
        let status_output = Self::run_foundry_command(&["service", "status"])?;

        if !status_output.status.success() {
            let stderr = String::from_utf8_lossy(&status_output.stderr);
            return Err(format!("Foundry 'service status' command failed: {}", stderr).into());
        }

        let status_stdout = String::from_utf8_lossy(&status_output.stdout);
        log::debug!("Foundry service status output: {}", status_stdout);

        extract_endpoint_url(&status_stdout)
    }

    pub fn get_model_id_once() -> Result<Option<String>> {
        let list_output = Self::run_foundry_command(&["service", "list"])?;
        if !list_output.status.success() {
            let stderr = String::from_utf8_lossy(&list_output.stderr);
            return Err(format!("Foundry 'service list' command failed: {}", stderr).into());
        }

        let list_stdout = String::from_utf8_lossy(&list_output.stdout);
        log::debug!("Foundry service list output: {}", list_stdout);

        if list_stdout.to_lowercase().contains("no models are currently loaded") {
            return Ok(None);
        }

        extract_model_id(&list_stdout).map(Some)
    }

    fn get_model_id_with_retry(
        attempts: usize,
        delay: std::time::Duration,
    ) -> Result<String> {
        for attempt in 1..=attempts {
            match Self::get_model_id_once() {
                Ok(Some(model_id)) => return Ok(model_id),
                Ok(None) => {}
                Err(err) => return Err(err),
            }

            if attempt < attempts {
                std::thread::sleep(delay);
            }
        }

        Err("Could not find model ID in Foundry service list output. Ensure a model is loaded and 'foundry service list' returns its ID.".into())
    }
}


/// Helper: extract URL from service list output
fn extract_endpoint_url(output: &str) -> Result<String> {
    let sanitized = strip_ansi(output);
    for line in sanitized.lines() {
        let re = regex::Regex::new(r"(https?://\S+)").unwrap();
        if let Some(captures) = re.captures(line) {
            let raw_url = captures[1].trim_end_matches(|c| c == '.' || c == ',');
            return Ok(normalize_base_url(raw_url));
        }
    }
    Err("Could not find endpoint URL in Foundry service status output".into())
}

fn normalize_base_url(url: &str) -> String {
    // Convert "http://127.0.0.1:49798/openai/status" -> "http://127.0.0.1:49798/v1"
    if let Some(scheme_end) = url.find("://") {
        let scheme = &url[..scheme_end];
        let rest = &url[(scheme_end + 3)..];
        if let Some(host_end) = rest.find('/') {
            let host = &rest[..host_end];
            return format!("{}://{}/v1", scheme, host);
        }
    }
    format!("{}/v1", url.trim_end_matches('/'))
}

fn strip_ansi(input: &str) -> String {
    let re = regex::Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
    re.replace_all(input, "").to_string()
}

/// Helper: extract model ID from service list output
fn extract_model_id(output: &str) -> Result<String> {
    let sanitized = strip_ansi(output);
    // Expected output (example):
    // Models running in service:
    //     Alias                          Model ID
    // 🟢  phi-4-mini                     Phi-4-mini-instruct-openvino-gpu:1
    for line in sanitized.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("🟢")
            || trimmed.starts_with("🟡")
            || trimmed.starts_with("🟠")
            || trimmed.starts_with("🔴")
        {
            let without_status = trimmed
                .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
                .trim();
            let parts: Vec<&str> = without_status
                .split_whitespace()
                .collect();
            if parts.len() >= 2 {
                return Ok(parts[parts.len() - 1].to_string());
            }
        }
    }

    // Fallback: look for a token that looks like a model id with a suffix ":<number>"
    let re = regex::Regex::new(r"([A-Za-z0-9_\-\.]+:[0-9]+)").unwrap();
    if let Some(captures) = re.captures(&sanitized) {
        if let Some(model_id_match) = captures.get(1) {
            return Ok(model_id_match.as_str().to_string());
        }
    }

    Err("Could not find model ID in Foundry service list output. Ensure a model is loaded and 'foundry service list' returns its ID.".into())
}
