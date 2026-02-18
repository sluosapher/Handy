use serde::{Deserialize, Serialize};
use specta::Type;

use crate::managers::foundry::FoundryManager;
use crate::{initialize_foundry_integration, update_foundry_settings};

const DEFAULT_FOUNDRY_MODEL: &str = "phi-4-mini";

async fn wait_for_model_cached(
    model_name: &str,
    attempts: usize,
    delay: std::time::Duration,
) -> Result<(), String> {
    for attempt in 1..=attempts {
        let model = model_name.to_string();
        let cached = tokio::task::spawn_blocking(move || {
            FoundryManager::is_model_cached(&model)
        })
            .await
            .map_err(|e| format!("Failed to check Foundry cache: {}", e))?
            .map_err(|e| format!("Failed to check Foundry cache: {}", e))?;

        if cached {
            return Ok(());
        }

        if attempt < attempts {
            tokio::time::sleep(delay).await;
        }
    }

    Err("Foundry model was not cached in time.".to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone, Type)]
pub struct FoundryStatus {
    pub installed: bool,
    pub running: bool,
    pub endpoint_url: Option<String>,
    pub model_id: Option<String>,
    pub model_cached: bool,
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
    let mut model_cached = false;

    if installed {
        running = FoundryManager::is_service_running()
            .unwrap_or_else(|e| {
                log::warn!("Failed to check Foundry service running status: {}", e);
                false
            });

        if running {
            model_cached = FoundryManager::is_model_cached(DEFAULT_FOUNDRY_MODEL)
                .unwrap_or_else(|e| {
                    log::warn!("Failed to check Foundry model cache: {}", e);
                    false
                });
        }

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
        model_cached,
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

    // Kick off model load in the background. The first load may download the model.
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        let load_result = tokio::task::spawn_blocking(|| {
            FoundryManager::ensure_model_downloaded(DEFAULT_FOUNDRY_MODEL)?;
            FoundryManager::run_model(DEFAULT_FOUNDRY_MODEL)
        })
        .await;

        match load_result {
            Ok(Ok(())) => {
                log::info!(
                    "Successfully instructed Foundry to run default model '{}'.",
                    DEFAULT_FOUNDRY_MODEL
                );
            }
            Ok(Err(e)) => {
                log::warn!(
                    "Failed to run default Foundry model '{}' after service start: {}",
                    DEFAULT_FOUNDRY_MODEL,
                    e
                );
            }
            Err(e) => {
                log::warn!(
                    "Foundry model load task failed for '{}': {}",
                    DEFAULT_FOUNDRY_MODEL,
                    e
                );
            }
        }

        if let Err(e) = wait_for_model_cached(
            DEFAULT_FOUNDRY_MODEL,
            120,
            std::time::Duration::from_secs(3),
        )
        .await
        {
            log::warn!("Foundry model did not appear in cache: {}", e);
        }

        if let Err(e) = initialize_foundry_integration(app_handle_clone).await {
            log::warn!("Post-start Foundry integration failed: {}", e);
        }
    });

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

    let running = FoundryManager::is_service_running()
        .map_err(|e| format!("Failed to check Foundry service status: {}", e))?;
    if !running {
        FoundryManager::start_service_with_timeout(std::time::Duration::from_secs(8))
            .map_err(|e| format!("Failed to ensure Foundry service is running: {}", e))?;
    }

    tokio::task::spawn_blocking(|| {
        FoundryManager::ensure_model_downloaded(DEFAULT_FOUNDRY_MODEL)
    })
    .await
    .map_err(|e| format!("Failed to download Foundry model: {}", e))?
    .map_err(|e| format!("Failed to download Foundry model: {}", e))?;

    if let Err(e) = wait_for_model_cached(
        DEFAULT_FOUNDRY_MODEL,
        120,
        std::time::Duration::from_secs(3),
    )
    .await
    {
        log::warn!("Foundry model cache check timed out: {}", e);
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

#[tauri::command]
#[specta::specta]
pub async fn install_foundry_local_command() -> Result<String, String> {
    let version = tokio::task::spawn_blocking(FoundryManager::install_foundry_local)
        .await
        .map_err(|e| format!("Foundry install task failed: {}", e))?
        .map_err(|e| format!("Failed to install Foundry Local: {}", e))?;

    Ok(version)
}
