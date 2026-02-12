# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Prerequisites:** [Rust](https://rustup.rs/) (latest stable), [Bun](https://bun.sh/)

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev
# If cmake error on macOS:
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev

# Build for production
bun run tauri build

# Linting and formatting (run before committing)
bun run lint              # ESLint for frontend
bun run lint:fix          # ESLint with auto-fix
bun run format            # Prettier + cargo fmt
bun run format:check      # Check formatting without changes
```

**Model Setup (Required for Development):**

```bash
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

## Architecture Overview

Handy is a cross-platform desktop speech-to-text app built with Tauri 2.x (Rust backend + React/TypeScript frontend).

### Backend Structure (src-tauri/src/)

- `lib.rs` - Main entry point, Tauri setup, manager initialization
- `managers/` - Core business logic:
  - `audio.rs` - Audio recording and device management
  - `model.rs` - Model downloading and management
  - `transcription.rs` - Speech-to-text processing pipeline
  - `history.rs` - Transcription history storage
  - `foundry.rs` - **NEW:** Microsoft Foundry Local integration (detection, service management, config update)
- `audio_toolkit/` - Low-level audio processing:
  - `audio/` - Device enumeration, recording, resampling
  - `vad/` - Voice Activity Detection (Silero VAD)
- `commands/` - Tauri command handlers for frontend communication
- `shortcut.rs` - Global keyboard shortcut handling
- `settings.rs` - Application settings management

### Frontend Structure (src/)

- `App.tsx` - Main component with onboarding flow. **NEW:** Integrates `FoundryNotification`.
- `components/settings/` - Settings UI (35+ files)
- `components/model-selector/` - Model management interface
- `components/onboarding/` - First-run experience
- `hooks/useSettings.ts`, `useModels.ts` - State management hooks
- `stores/settingsStore.ts` - Zustand store for settings
- `bindings.ts` - Auto-generated Tauri type bindings (via tauri-specta). **NEW:** Includes Foundry-related types and commands.
- `overlay/` - Recording overlay window code
- `components/settings/FoundryNotification.tsx` - **NEW:** Component for displaying Foundry-related notifications and actions.

### Key Patterns

**Manager Pattern:** Core functionality organized into managers (Audio, Model, Transcription, **Foundry**) initialized at startup and managed via Tauri state.

**Command-Event Architecture:** Frontend → Backend via Tauri commands; Backend → Frontend via events.

**Pipeline Processing:** Audio → VAD → Whisper/Parakeet → Text output → Clipboard/Paste. **NEW:** Text output can optionally go through Foundry Local for post-processing.

**State Flow:** Zustand → Tauri Command → Rust State → Persistence (tauri-plugin-store)

---

## Microsoft Foundry Local Integration Design

### Section 1: Overview & Architecture

**Core Concept:** Automate Microsoft Foundry Local discovery and configuration for Handy's post-processing feature, minimizing user setup burden while maintaining existing UI patterns.

**Architecture Overview:**

```
┌─────────────────────────────────────────────────────────────┐
│                    Handy Application                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐         ┌─────────────────────────┐  │
│  │  lib.rs         │         │  Foundry Manager        │  │
│  │  (Startup)      │───────▶│  - Detection             │  │
│  └─────────────────┘         │  - Service Management    │  │
│         │                    │  - Config Update         │  │
│         ▼                    └─────────────────────────┘  │
│  ┌─────────────────┐                  │                    │
│  │  Commands       │                  │                    │
│  │  - Check status │                  │                    │
│  │  - Start        │                  │                    │
│  │  - Get endpoint │                  │                    │
│  └─────────────────┘                  │                    │
│         ▼                             ▼                    │
│  ┌─────────────────┐      Foundry Local (External)      │
│  │  Frontend       │                                      │
│  │  - Notification │                                      │
│  │  - Settings UI  │                                      │
│  └─────────────────┘                                      │
└─────────────────────────────────────────────────────────────┘
```

**Key Components:**
1. **Foundry Manager** (Rust): Handles installation detection, service management, and endpoint discovery
2. **Tauri Commands**: Frontend-backend communication layer
3. **Startup Integration**: Trigger detection when Handy launches
4. **Custom Provider Reuse**: Leverage existing post-processing infrastructure

**Key Features:**
- Automatic detection at application startup (only)
- **NEW:** One-time notification for installation with option to download/install Foundry.
- **NEW:** Auto-start Foundry service if installed but inactive.
- **NEW:** Automatically load `phi-3.5-mini` model for post-processing if Foundry is running.
- Extract endpoint URL and model ID via command-line tools.
- Transparent configuration via existing "Custom" provider in Handy's settings (i.e. `settings.post_process_provider_id` will be set to `"custom"` and `settings.post_process_models.custom` and `settings.post_process_providers` will be updated).
- Minimal UI changes - reuse existing patterns.

### Section 2: Implementation Strategy

**Discovery & Configuration Flow (Rust Startup):**

```rust
// Pseudocode for the startup flow
const DEFAULT_FOUNDRY_MODEL = "phi-3.5-mini"; // Define default model

fn startup() {
    // 1. Check if Foundry is installed
    if foundry_installed() {
        // 2. Check if Foundry service is running
        if !foundry_service_running() {
            // 3. Auto-start Foundry service
            start_foundry_service();
        }

        // NEW: Load default Foundry model
        run_foundry_model(DEFAULT_FOUNDRY_MODEL);

        // 4. Get current endpoint and model from Foundry CLI
        let (endpoint, model_id) = foundry_get_endpoint();

        // 5. Update settings_store.json with new values
        update_settings_store(endpoint, model_id);
    } else {
        // 6. Prompt user for installation (one-time) - Handled by frontend notification
    }
}
```

**Key Implementation Details:**

1.  **Foundry Installation Detection:**
    -   Check common installation paths (e.g., `C:\Program Files\FoundryLocal` on Windows, or rely on `foundry --help` command being available in PATH).
    -   Verify `foundry.exe` or similar executable exists.
    -   Check for required CLI tools.

2.  **Service Discovery Commands:**
    -   `foundry service start`: Start the Foundry service.
    -   `foundry service list`: Get information about loaded models, including the endpoint URL and model ID.
    -   `foundry model run <model_name> --retain`: Command to run a specific model.

3.  **Settings File Update:**
    -   Parse existing `settings_store.json` located at `~\AppData\Roaming\com.pais.handy\settings_store.json`.
    -   Update only the `custom` provider's `base_url` and `model` within the `settings.post_process_providers` and `settings.post_process_models` JSON structures.
    -   Set `settings.post_process_provider_id` to `"custom"`.
    -   Preserve all other settings.
    -   Write back atomically to avoid corruption.

4.  **Error Handling:**
    -   Gracefully handle cases where Foundry CLI tools aren't found.
    -   Skip update if Foundry is running but returns errors for endpoint/model info.
    -   Log warnings rather than crash the app.

### Section 3: Foundry Integration Manager (Rust Backend)

**Component: `src-tauri/src/managers/foundry.rs`**

```rust
use std::process::Command;
use std::path::PathBuf;
use serde_json::Value; // Added use for Value
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
```

### Section 4: Integration Points

**1. Startup Integration (`src-tauri/src/lib.rs` - Modifications within `setup` callback)**

```rust
mod managers;
mod commands;
mod settings; // Assuming settings module is used for store interaction

use managers::foundry::FoundryManager;
use tauri::{Manager, App};
use serde_json::json; // For constructing JSON to update settings store
use tauri_plugin_store::StoreBuilder; // For direct store access from main process

/// Main function remains largely the same, only the setup closure is modified.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = tauri::Builder::default()
        // ... existing setup
        .setup(|app| {
            // ... existing initializations

            // Spawn an async task for Foundry integration to not block the main UI thread
            let app_handle = app.handle(); // Clone handle for the async task
            tokio::spawn(async move {
                if let Err(e) = initialize_foundry_integration(app_handle).await {
                    log::warn!("Foundry integration failed during startup: {}", e);
                }
            });

            Ok(())
        });

    // ... rest of main function
    builder.run(tauri::generate_context!())?;
    Ok(())
}

/// Initialize Foundry integration during startup
async fn initialize_foundry_integration(app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error.Error>> {
    if !FoundryManager::is_installed() {
        log::info!("Microsoft Foundry Local not installed, skipping automatic integration.");
        // Frontend will be responsible for prompting installation if needed.
        return Ok(());
    }

    log::info!("Foundry detected. Checking service status...");

    let is_running = FoundryManager::is_service_running()
        .unwrap_or_else(|e| {
            log::warn!("Failed to determine Foundry service status: {}", e);
            false
        });

    if !is_running {
        log::info!("Foundry service not running, attempting to start...");
        if let Err(e) = FoundryManager::start_service() {
            log::warn!("Failed to auto-start Foundry service: {}", e);
            // If startup fails, proceed without updating settings, frontend can notify.
            return Ok(());
        }
        log::info!("Foundry service started successfully.");
    } else {
        log::info!("Foundry service is already running.");
    }

    // NEW: Attempt to run the default model after ensuring the service is up.
    const DEFAULT_FOUNDRY_MODEL: &str = "phi-3.5-mini";
    if let Err(e) = FoundryManager::run_model(DEFAULT_FOUNDRY_MODEL) {
        log::warn!("Failed to run default Foundry model \'{}\' during startup: {}", DEFAULT_FOUNDRY_MODEL, e);
        // Continue, as some models might be pre-loaded or custom
    } else {
        log::info!("Successfully instructed Foundry to run default model \'{}\' during startup.", DEFAULT_FOUNDRY_MODEL);
    }

    // After ensuring service is running, attempt to get endpoint and model info.
    match FoundryManager::get_endpoint_info() {
        Ok((endpoint_url, model_id)) => {
            log::info!("Discovered Foundry endpoint: {}", endpoint_url);
            log::info!("Discovered Foundry model: {}", model_id);

            // Update Handy's settings with the discovered Foundry info
            if let Err(e) = update_foundry_settings(&app_handle, endpoint_url, model_id).await {
                log::warn!("Failed to update Handy settings with Foundry info: {}", e);
            } else {
                log::info!("Handy settings updated with Foundry Local configuration.");
            }
        }
        Err(e) => {
            log::warn!("Failed to get Foundry endpoint info: {}. Frontend may prompt manual configuration.", e);
        }
    }

    Ok(())
}

/// Update Handy's settings_store.json with Foundry endpoint and model
/// This function is called from the main process during application startup.
async fn update_foundry_settings(
    app_handle: &tauri::AppHandle,
    endpoint_url: String,
    model_id: String,
) -> Result<(), Box<dyn std::error.Error>> {
    // Get the path to settings_store.json
    let settings_path = app_handle.path_resolver()
        .app_config_dir()
        .ok_or("Could not get app config directory")?
        .join("settings_store.json");

    // Load existing settings
    let mut store = StoreBuilder::new(settings_path).build();
    store.load().map_err(|e| format!("Failed to load settings: {}", e))?;

    // Get current settings data, if exists
    let mut settings_data: Value = store.get("settings").cloned().unwrap_or_else(|| json!({}));

    // Ensure post_process_providers is an array
    let providers = settings_data["post_process_providers"]
        .as_array_mut()
        .ok_or("post_process_providers is not an array or missing")?;

    // Find and update the "custom" provider, or add it if not found
    let custom_provider_index = providers
        .iter()
        .position(|p| p["id"].as_str() == Some("custom"));

    let new_custom_provider = json!({
        "id": "custom",
        "label": "Custom",
        "base_url": endpoint_url,
        "allow_base_url_edit": true,
        "models_endpoint": "/models"
    });

    if let Some(index) = custom_provider_index {
        providers[index] = new_custom_provider;
    } else {
        providers.push(new_custom_provider);
    }

    // Update post_process_provider_id to "custom"
    settings_data["post_process_provider_id"] = json!("custom");

    // Update post_process_models.custom
    let models_map = settings_data["post_process_models"]
        .as_object_mut()
        .ok_or("post_process_models is not an object or missing")?;

    models_map.insert("custom".to_string(), json!(model_id));

    // Save updated settings object back to the store
    store.set("settings", settings_data.clone()).map_err(|e| format!("Failed to update settings object in store: {}", e))?;

    // Save settings to disk
    store.save().map_err(|e| format!("Failed to save settings: {}", e))?;

    // Notify frontend of settings change (optional, but good for reactivity)
    app_handle.emit_and_wait("settings-changed", {}).await?;

    Ok(())
}
```

**2. Tauri Commands (`src-tauri/src/commands/mod.rs` and new `foundry.rs` within `commands`)**

First, ensure `src-tauri/src/commands/mod.rs` exports the new commands:

```rust
// src-tauri/src/commands/mod.rs
pub mod foundry; // Add this line

// ... existing `pub use` statements ...
pub use foundry::*; // Export new Foundry commands
```

Then, create and populate `src-tauri/src/commands/foundry.rs`:

```rust
// src-tauri/src/commands/foundry.rs
use tauri::command;
use crate::managers::foundry::FoundryManager;
use crate::error::Result;
use serde::{Serialize, Deserialize};
use crate::initialize_foundry_integration; // Import the startup integration function

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FoundryStatus {
    pub installed: bool,
    pub running: bool,
    pub endpoint_url: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FoundryConfig {
    pub endpoint_url: String,
    pub model_id: String,
}

#[command]
pub async fn get_foundry_status(app_handle: tauri::AppHandle) -> Result<FoundryStatus, String> {
    let installed = FoundryManager::is_installed();
    let mut running = false;
    let mut endpoint_url = None;
    let mut model_id = None;

    if installed {
        running = FoundryManager::is_service_running()
            .unwrap_or_else(|e| {
                log::warn!("Failed to check Foundry service running status: {}", e);
                false
            });

        if running {
            match FoundryManager::get_endpoint_info() {
                Ok((url, id)) => {
                    endpoint_url = Some(url);
                    model_id = Some(id);
                },
                Err(e) => log::warn!("Failed to get Foundry endpoint info: {}", e),
            }
        }
    }

    Ok(FoundryStatus {
        installed,
        running,
        endpoint_url,
        model_id,
    })
}

#[command]
pub async fn start_foundry_service_command(app_handle: tauri::AppHandle) -> Result<(), String> {
    if !FoundryManager::is_installed() {
        return Err("Foundry Local is not installed.".to_string());
    }

    FoundryManager::start_service()
        .map_err(|e| format!("Failed to start Foundry service: {}", e))?;
    log::info!("Foundry service started via command.");

    // NEW: Attempt to run the default model after starting the service.
    const DEFAULT_FOUNDRY_MODEL: &str = "phi-3.5-mini";
    if let Err(e) = FoundryManager::run_model(DEFAULT_FOUNDRY_MODEL) {
        log::warn!("Failed to run default Foundry model \'{}\' after service start: {}", DEFAULT_FOUNDRY_MODEL, e);
    } else {
        log::info!("Successfully instructed Foundry to run default model \'{}\' after service start.", DEFAULT_FOUNDRY_MODEL);
    }

    // After starting, and attempting to run model, re-run the full integration to update settings
    if let Err(e) = initialize_foundry_integration(app_handle).await {
        log::warn!("Post-start Foundry integration failed: {}", e);
    }
    Ok(())
}

#[command]
pub async fn configure_foundry_integration_command(app_handle: tauri::AppHandle) -> Result<FoundryConfig, String> {
    if !FoundryManager::is_installed() {
        return Err("Foundry Local is not installed.".to_string());
    }

    if let Err(e) = FoundryManager::start_service() {
        return Err(format!("Failed to ensure Foundry service is running: {}", e));
    }

    let (endpoint_url, model_id) = FoundryManager::get_endpoint_info()
        .map_err(|e| format!("Failed to discover Foundry endpoint and model: {}", e))?;

    // Update settings in Handy with the discovered Foundry info
    super::update_foundry_settings(&app_handle, endpoint_url.clone(), model_id.clone()).await
        .map_err(|e| format!("Failed to update Handy settings with Foundry configuration: {}", e))?;

    Ok(FoundryConfig {
        endpoint_url,
        model_id,
    })
}

// Add a command to trigger model execution, useful if Foundry doesn't auto-run it.
#[command]
pub async fn run_foundry_model_command(model_name: String) -> Result<(), String> {
    FoundryManager::run_model(&model_name)
        .map_err(|e| format!("Failed to run Foundry model \'{}\'": {}", model_name, e))
}

// Add a command to list available models, for optional future UI enhancements
#[command]
pub async fn get_foundry_available_models_command() -> Result<Vec<String>, String> {
    FoundryManager::get_available_models()
        .map_err(|e| format!("Failed to get available Foundry models: {}", e))
}
```
*Note: Make sure to update `src-tauri/Cargo.toml` to include `regex` crate for the new endpoint extraction. Add `regex = "1.5"` under `[dependencies]`.*

**3. Frontend Integration Points (`src/App.tsx` and `src/components/settings/FoundryNotification.tsx`)**

**Component: `src/components/settings/FoundryNotification.tsx`**

```typescript
// src/components/settings/FoundryNotification.tsx
import { useEffect, useState } from "react";
import { commands } from "@/bindings";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { useTranslation } from "react-i18next";
import { useSettingsStore } from "@/stores/settingsStore"; // To trigger UI refresh

export const FoundryNotification: React.FC = () => {
  const { t } = useTranslation();
  const [showPrompt, setShowPrompt] = useState(false);
  const [isConfiguring, setIsConfiguring] = useState(false);
  const [installChecked, setInstallChecked] = useState(false); // Track if install status has been checked

  const refreshSettings = useSettingsStore(state => state.refreshSettings);

  useEffect(() => {
    // Check Foundry status on component mount
    checkFoundryStatus();
  }, []);

  const checkFoundryStatus = async () => {
    try {
      const status = await commands.getFoundryStatus();
      setInstallChecked(true); // Mark that we've checked install status

      if (!status.installed) {
        // Foundry not installed, show prompt to install
        setShowPrompt(true);
      } else if (!status.running) {
        // Foundry installed but not running, show prompt to start/configure
        setShowPrompt(true); // Still show prompt but with different message/action
      }
      // If installed and running, no prompt is needed.
    } catch (error) {
      console.error("Failed to check Foundry status:", error);
      // Decide if an error specific prompt should be shown
    }
  };

  const handleStartAndConfigure = async () => {
    setIsConfiguring(true);
    try {
      // Calls start_foundry_service_command which also triggers the backend
      // initialize_foundry_integration to update settings.
      await commands.startFoundryServiceCommand();
      await commands.configureFoundryIntegrationCommand(); // This command also includes running the default model
      await refreshSettings(); // Force frontend settings refresh
      setShowPrompt(false); // Hide notification on success
    } catch (error) {
      console.error("Failed to start and configure Foundry:", error);
      alert(t("foundry.error.startAndConfigure", { error: error })); // Show user-friendly error
    } finally {
      setIsConfiguring(false);
    }
  };

  const handleInstallClick = () => {
    // Open Microsoft Foundry Local get started page
    window.open("https://learn.microsoft.com/en-us/azure/ai-foundry/foundry-local/get-started?view=foundry-classic", "_blank");
  };

  if (!showPrompt || !installChecked) return null; // Only show if prompt is needed and we've checked installation

  return (
    <Alert variant="info" contained>
      <div className="flex items-center justify-between">
        <div>
          {/* Determine message based on status */}
          {(async () => { // IIFE to use await inside JSX
            const status = await commands.getFoundryStatus();
            if (!status.installed) {
              return (
                <>
                  <p className="font-semibold">{t("foundry.notInstalled.title")}</p>
                  <p className="text-sm mt-1">
                    {t("foundry.notInstalled.description")}
                  </p>
                </>
              );
            } else if (!status.running) {
              return (
                <>
                  <p className="font-semibold">{t("foundry.installedButNotRunning.title")}</p>
                  <p className="text-sm mt-1">
                    {t("foundry.installedButNotRunning.description")}
                  </p>
                </>
              );
            }
            return null; // Should not happen if showPrompt is true and installed/not running checks are correct
          })()}
        </div>
        <div className="flex gap-2 ml-4">
          {(async () => {
            const status = await commands.getFoundryStatus();
            if (!status.installed) {
              return (
                <Button
                  onClick={handleInstallClick}
                  variant="primary"
                >
                  {t("foundry.notInstalled.installButton")}
                </Button>
              );
            } else if (!status.running) {
              return (
                <Button
                  onClick={handleStartAndConfigure}
                  disabled={isConfiguring}
                  variant="primary"
                >
                  {isConfiguring ? t("foundry.installedButNotRunning.configuringButton") : t("foundry.installedButNotRunning.startButton")}
                </Button>
              );
            }
            return null;
          })()}
          <Button
            onClick={() => setShowPrompt(false)}
            variant="ghost"
          >
            {t("foundry.dismissButton")}
          </Button>
        </div>
      </div>
    </Alert>
  );
};
```
*Note: You would need to add translation keys (e.g., `foundry.notInstalled.title`, `foundry.error.startAndConfigure`) to your `src/i18n/locales/en/translation.json` file for the above frontend code.*

**App.tsx Integration:**

```typescript
// src/App.tsx
import { FoundryNotification } from "@/components/settings/FoundryNotification"; // Import the new component

function App() {
  return (
    <div className="app">
      {/* Show Foundry notification at the top */}
      <FoundryNotification />

      {/* Rest of your main application components */}
      {/* ... */}
    </div>
  );
}
```

**4. Bindings Update (`src/bindings.ts`)**

```typescript
// src/bindings.ts (Add to the existing auto-generated file or ensure your generation process includes these)

export interface FoundryStatus {
  installed: boolean;
  running: boolean;
  endpoint_url: string | null;
  model_id: string | null;
}

export interface FoundryConfig {
  endpoint_url: string;
  model_id: string;
}

export const commands = {
  // ... existing commands ...

  // Foundry integration commands
  getFoundryStatus: (): Promise<FoundryStatus> => window.__TAURI_INTERNALS__.invoke("get_foundry_status"),
  startFoundryServiceCommand: (): Promise<void> => window.__TAURI_INTERNALS__.invoke("start_foundry_service_command"),
  configureFoundryIntegrationCommand: (): Promise<FoundryConfig> => window.__TAURI_INTERNALS__.invoke("configure_foundry_integration_command"),
  runFoundryModelCommand: (modelName: string): Promise<void> => window.__TAURI_INTERNALS__.invoke("run_foundry_model_command", { modelName }),
  getFoundryAvailableModelsCommand: (): Promise<string[]> => window.__TAURI_INTERNALS__.invoke("get_foundry_available_models_command"),
};

// Also ensure AppSettings type (from settingsStore.ts) can handle new provider changes
// (This is usually inferred by tauri-specta, but if manual, verify structure)
// Example:
// export interface AppSettings {
//   // ... existing fields ...
//   post_process_providers: PostProcessProvider[];
//   post_process_models: Record<string, string>; // Maps providerId to modelId
//   post_process_provider_id: string; // The currently selected provider
// }
//
// export interface PostProcessProvider {
//    id: string;
//    label: string;
//    base_url: string;
//    allow_base_url_edit: boolean;
//    models_endpoint: string;
// }
```

---

## Code Style

**Rust:**

- Run `cargo fmt` and `cargo clippy` before committing.
- Handle errors explicitly (avoid unwrap in production).
- Use descriptive names, add doc comments for public APIs.
- Prefer `anyhow::Result` for error propagation.

**TypeScript/React:**

- Strict TypeScript, avoid `any` types.
- Functional components with hooks.
- Tailwind CSS for styling.
- Path aliases: `@/` → `./src/`.

## Commit Guidelines

Use conventional commits:

- `feat:` new features
- `fix:` bug fixes
- `docs:` documentation
- `refactor:` code refactoring
- `chore:` maintenance

## Debug Mode

Access debug features: `Cmd+Shift+D` (macOS) or `Ctrl+Shift+D` (Windows/Linux)

## Platform Notes

- **macOS**: Metal acceleration, accessibility permissions required
- **Windows**: Vulkan acceleration, code signing
- **Linux**: OpenBLAS + Vulkan, limited Wayland support, overlay disabled by default
