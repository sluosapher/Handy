import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";

import { commands, type FoundryStatus } from "@/bindings";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { useSettingsStore } from "@/stores/settingsStore";

const FOUNDRY_INSTALL_URL =
  "https://learn.microsoft.com/en-us/azure/ai-foundry/foundry-local/get-started?view=foundry-classic";

export const FoundryNotification: React.FC = () => {
  const { t } = useTranslation();
  const refreshSettings = useSettingsStore((state) => state.refreshSettings);
  const [status, setStatus] = useState<FoundryStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isConfiguring, setIsConfiguring] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  const loadStatus = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await commands.getFoundryStatus();
      if (result.status === "ok") {
        setStatus(result.data);
      } else {
        console.warn("Failed to load Foundry status:", result.error);
      }
    } catch (error) {
      console.error("Failed to check Foundry status:", error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  const handleStartAndConfigure = async () => {
    setIsConfiguring(true);
    try {
      const startResult = await commands.startFoundryServiceCommand();
      if (startResult.status === "error") {
        throw new Error(startResult.error);
      }

      const configureResult =
        await commands.configureFoundryIntegrationCommand();
      if (configureResult.status === "error") {
        throw new Error(configureResult.error);
      }

      await refreshSettings();
      await loadStatus();
      setDismissed(false);
    } catch (error) {
      console.error("Failed to start and configure Foundry:", error);
      alert(
        t("foundry.error.startAndConfigure", {
          error: error instanceof Error ? error.message : String(error),
        }),
      );
    } finally {
      setIsConfiguring(false);
    }
  };

  const handleInstallClick = () => {
    void openUrl(FOUNDRY_INSTALL_URL);
  };

  if (dismissed || isLoading || !status) return null;

  const needsInstall = !status.installed;
  const needsStart = status.installed && !status.running;

  if (!needsInstall && !needsStart) return null;

  return (
    <Alert variant="info" contained className="w-full">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-[220px]">
          {needsInstall ? (
            <>
              <p className="font-semibold">{t("foundry.notInstalled.title")}</p>
              <p className="text-sm mt-1 text-mid-gray">
                {t("foundry.notInstalled.description")}
              </p>
            </>
          ) : (
            <>
              <p className="font-semibold">
                {t("foundry.installedButNotRunning.title")}
              </p>
              <p className="text-sm mt-1 text-mid-gray">
                {t("foundry.installedButNotRunning.description")}
              </p>
            </>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          {needsInstall ? (
            <Button onClick={handleInstallClick} variant="primary">
              {t("foundry.notInstalled.installButton")}
            </Button>
          ) : (
            <Button
              onClick={handleStartAndConfigure}
              disabled={isConfiguring}
              variant="primary"
            >
              {isConfiguring
                ? t("foundry.installedButNotRunning.configuringButton")
                : t("foundry.installedButNotRunning.startButton")}
            </Button>
          )}
          <Button onClick={() => setDismissed(true)} variant="ghost">
            {t("foundry.dismissButton")}
          </Button>
        </div>
      </div>
    </Alert>
  );
};
