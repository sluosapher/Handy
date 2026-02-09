use serde::{Deserialize, Serialize};
use specta::Type;

use crate::managers::foundry::FoundryManager;
use crate::{initialize_foundry_integration, update_foundry_settings};

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct FoundryStatus {
    pub installed: bool,
    pub running: bool,
    pub endpoint_url: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct FoundryConfig {
    pub endpoint_url: String,
    pub model_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn get_foundry_status(_app_handle: tauri::AppHandle) -> Result<FoundryStatus, String> {
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
            match FoundryManager::get_endpoint_url() {
                Ok(url) => endpoint_url = Some(url),
                Err(e) => log::warn!("Failed to get Foundry endpoint url: {}", e),
            }

            match FoundryManager::get_model_id_once() {
                Ok(Some(id)) => model_id = Some(id),
                Ok(None) => {}
                Err(e) => log::warn!("Failed to get Foundry model id: {}", e),
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

#[tauri::command]
#[specta::specta]
pub async fn start_foundry_service_command(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if !FoundryManager::is_installed() {
        return Err("Foundry Local is not installed.".to_string());
    }

    FoundryManager::start_service_with_timeout(std::time::Duration::from_secs(8))
        .map_err(|e| format!("Failed to start Foundry service: {}", e))?;
    log::info!("Foundry service started via command.");

    const DEFAULT_FOUNDRY_MODEL: &str = "phi-3.5-mini";
    if let Err(e) = FoundryManager::run_model(DEFAULT_FOUNDRY_MODEL) {
        log::warn!(
            "Failed to run default Foundry model '{}' after service start: {}",
            DEFAULT_FOUNDRY_MODEL,
            e
        );
    } else {
        log::info!(
            "Successfully instructed Foundry to run default model '{}' after service start.",
            DEFAULT_FOUNDRY_MODEL
        );
    }

    if let Err(e) = initialize_foundry_integration(app_handle).await {
        log::warn!("Post-start Foundry integration failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn configure_foundry_integration_command(
    app_handle: tauri::AppHandle,
) -> Result<FoundryConfig, String> {
    if !FoundryManager::is_installed() {
        return Err("Foundry Local is not installed.".to_string());
    }

    if let Err(e) = FoundryManager::start_service() {
        return Err(format!("Failed to ensure Foundry service is running: {}", e));
    }

    let (endpoint_url, model_id) = FoundryManager::get_endpoint_info()
        .map_err(|e| format!("Failed to discover Foundry endpoint and model: {}", e))?;

    update_foundry_settings(&app_handle, endpoint_url.clone(), Some(model_id.clone()))
        .await
        .map_err(|e| format!("Failed to update Handy settings with Foundry configuration: {}", e))?;

    Ok(FoundryConfig {
        endpoint_url,
        model_id,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn run_foundry_model_command(model_name: String) -> Result<(), String> {
    FoundryManager::run_model(&model_name)
        .map_err(|e| format!("Failed to run Foundry model '{}': {}", model_name, e))
}

#[tauri::command]
#[specta::specta]
pub async fn get_foundry_available_models_command() -> Result<Vec<String>, String> {
    FoundryManager::get_available_models()
        .map_err(|e| format!("Failed to get available Foundry models: {}", e))
}
