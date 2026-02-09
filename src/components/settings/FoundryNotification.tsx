import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ask } from "@tauri-apps/plugin-dialog";

import { commands, type FoundryStatus } from "@/bindings";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { useSettingsStore } from "@/stores/settingsStore";

export const FoundryNotification: React.FC = () => {
  const { t } = useTranslation();
  const refreshSettings = useSettingsStore((state) => state.refreshSettings);
  const [status, setStatus] = useState<FoundryStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isConfiguring, setIsConfiguring] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);
  const [installElapsedSeconds, setInstallElapsedSeconds] = useState(0);
  const [configureElapsedSeconds, setConfigureElapsedSeconds] = useState(0);
  const [dismissed, setDismissed] = useState(false);

  const loadStatus = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await commands.getFoundryStatus();
      if (result.status === "ok") {
        setStatus(result.data);
        setStatusError(null);
      } else {
        console.warn("Failed to load Foundry status:", result.error);
        setStatusError(result.error);
        setStatus({
          installed: false,
          running: false,
          endpoint_url: null,
          model_id: null,
          model_cached: false,
        });
      }
    } catch (error) {
      console.error("Failed to check Foundry status:", error);
      setStatusError(error instanceof Error ? error.message : String(error));
      setStatus({
        installed: false,
        running: false,
        endpoint_url: null,
        model_id: null,
        model_cached: false,
      });
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  useEffect(() => {
    if (!isInstalling) {
      setInstallElapsedSeconds(0);
      return;
    }

    const startedAt = Date.now();
    setInstallElapsedSeconds(0);
    const timer = window.setInterval(() => {
      setInstallElapsedSeconds(
        Math.floor((Date.now() - startedAt) / 1000),
      );
    }, 1000);

    return () => window.clearInterval(timer);
  }, [isInstalling]);

  useEffect(() => {
    if (!isConfiguring) {
      setConfigureElapsedSeconds(0);
      return;
    }

    const startedAt = Date.now();
    setConfigureElapsedSeconds(0);
    const timer = window.setInterval(() => {
      setConfigureElapsedSeconds(
        Math.floor((Date.now() - startedAt) / 1000),
      );
    }, 1000);

    return () => window.clearInterval(timer);
  }, [isConfiguring]);

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
      toast.success(t("foundry.installedButNotRunning.success"));
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

  const handleInstallClick = async () => {
    const confirmed = await ask(t("foundry.notInstalled.installConfirm"), {
      title: t("foundry.notInstalled.installConfirmTitle"),
      kind: "warning",
    });

    if (!confirmed) return;

    setIsInstalling(true);
    try {
      const installResult = await commands.installFoundryLocalCommand();
      if (installResult.status === "error") {
        throw new Error(installResult.error);
      }

      await loadStatus();
      setDismissed(false);
      alert(
        t("foundry.notInstalled.installSuccess", {
          version: installResult.data,
        }),
      );
    } catch (error) {
      console.error("Failed to install Foundry Local:", error);
      alert(
        t("foundry.error.install", {
          error: error instanceof Error ? error.message : String(error),
        }),
      );
    } finally {
      setIsInstalling(false);
    }
  };

  if (dismissed || isLoading || !status) return null;

  const needsInstall = !status.installed;
  const needsDownload = status.installed && !status.model_cached;
  const needsStart = status.installed && !status.running;

  if (!needsInstall && !needsDownload && !needsStart) return null;

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
              {isInstalling && (
                <div className="mt-3 space-y-2">
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-mid-gray/20">
                    <div className="h-full w-1/3 bg-logo-primary/80 foundry-install-bar" />
                  </div>
                  <p className="text-xs text-mid-gray">
                    {t("foundry.notInstalled.installingDetail", {
                      seconds: installElapsedSeconds,
                    })}
                  </p>
                </div>
              )}
              {statusError && (
                <p className="text-xs mt-2 text-mid-gray">
                  {t("foundry.error.status", { error: statusError })}
                </p>
              )}
            </>
          ) : needsDownload ? (
            <>
              <p className="font-semibold">
                {t("foundry.modelNotCached.title")}
              </p>
              <p className="text-sm mt-1 text-mid-gray">
                {t("foundry.modelNotCached.description")}
              </p>
              {isConfiguring && (
                <div className="mt-3 space-y-2">
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-mid-gray/20">
                    <div className="h-full w-1/3 bg-logo-primary/80 foundry-install-bar" />
                  </div>
                  <p className="text-xs text-mid-gray">
                    {t("foundry.installedButNotRunning.downloadingDetail", {
                      seconds: configureElapsedSeconds,
                    })}
                  </p>
                </div>
              )}
            </>
          ) : (
            <>
              <p className="font-semibold">
                {t("foundry.installedButNotRunning.title")}
              </p>
              <p className="text-sm mt-1 text-mid-gray">
                {t("foundry.installedButNotRunning.description")}
              </p>
              {isConfiguring && (
                <div className="mt-3 space-y-2">
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-mid-gray/20">
                    <div className="h-full w-1/3 bg-logo-primary/80 foundry-install-bar" />
                  </div>
                  <p className="text-xs text-mid-gray">
                    {t("foundry.installedButNotRunning.downloadingDetail", {
                      seconds: configureElapsedSeconds,
                    })}
                  </p>
                </div>
              )}
            </>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          {needsInstall ? (
            <Button
              onClick={handleInstallClick}
              disabled={isInstalling}
              variant="primary"
            >
              {isInstalling
                ? t("foundry.notInstalled.installingButton")
                : t("foundry.notInstalled.installButton")}
            </Button>
          ) : (
            <Button
              onClick={handleStartAndConfigure}
              disabled={isConfiguring}
              variant="primary"
            >
              {isConfiguring
                ? t("foundry.installedButNotRunning.downloadingButton")
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
