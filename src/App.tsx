import React, { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Header } from "./components/Header";
import { ServerCard } from "./components/ServerCard";
import { SettingsModal } from "./components/SettingsModal";
import { ChangelogDrawer } from "./components/ChangelogDrawer";
import { LoginModal } from "./components/LoginModal";
import { Play, Flame, AlertCircle, FileText, CheckCircle2, FolderOpen, Check } from "lucide-react";
import {
  AppConfig,
  AuthStatus,
  MinecraftProfile,
  ProgressPayload,
  ServerStatus,
  DeviceCodeInfo,
} from "./types";

export const App: React.FC = () => {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [profile, setProfile] = useState<MinecraftProfile | null>(null);
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);

  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [deviceInfo, setDeviceInfo] = useState<DeviceCodeInfo | null>(null);
  const [isLoginModalOpen, setIsLoginModalOpen] = useState(false);
  const pollingRef = useRef<number | null>(null);

  const [isSyncing, setIsSyncing] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [isRefreshingServer, setIsRefreshingServer] = useState(false);
  const [launchSuccess, setLaunchSuccess] = useState(false);

  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [updateToast, setUpdateToast] = useState<string | null>(null);

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isChangelogOpen, setIsChangelogOpen] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
    checkAuth();
    refreshServer();

    invoke<boolean>("get_game_status")
      .then((running) => setIsRunning(running))
      .catch(() => {});

    const unlistenProgressPromise = listen<ProgressPayload>("download-progress", (event) => {
      setProgress(event.payload);
    });

    const unlistenGamePromise = listen<boolean>("game-status", (event) => {
      setIsRunning(event.payload);
      if (event.payload) {
        setIsLaunching(false);
      }
    });

    const unlistenErrorPromise = listen<string>("game-error", (event) => {
      setErrorMsg(`Minecraft process stopped unexpectedly:\n${event.payload}`);
    });

    return () => {
      unlistenProgressPromise.then((unlisten) => unlisten());
      unlistenGamePromise.then((unlisten) => unlisten());
      unlistenErrorPromise.then((unlisten) => unlisten());
    };
  }, []);

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      setConfig(cfg);
    } catch (err) {
      console.error("Failed to load config:", err);
    }
  };

  const checkAuth = async () => {
    try {
      const auth = await invoke<AuthStatus>("get_auth_status");
      setProfile(auth.profile);
    } catch (err) {
      console.error("Failed to check auth:", err);
    }
  };

  const refreshServer = async () => {
    setIsRefreshingServer(true);
    try {
      const status = await invoke<ServerStatus>("get_server_status");
      setServerStatus(status);
    } catch (err) {
      console.error("Failed to ping server:", err);
    } finally {
      setIsRefreshingServer(false);
    }
  };

  const handleLogin = async () => {
    setErrorMsg(null);
    setIsLoggingIn(true);
    try {
      const info = await invoke<DeviceCodeInfo>("start_device_login");
      setDeviceInfo(info);
      setIsLoginModalOpen(true);
      startPolling(info.device_code, info.interval || 5);
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsLoggingIn(false);
    }
  };

  const startPolling = (deviceCode: string, intervalSec: number) => {
    if (pollingRef.current) clearInterval(pollingRef.current);
    pollingRef.current = setInterval(async () => {
      try {
        const newProfile = await invoke<MinecraftProfile | null>("poll_device_login", { deviceCode });
        if (newProfile) {
          setProfile(newProfile);
          setIsLoginModalOpen(false);
          setDeviceInfo(null);
          if (pollingRef.current) clearInterval(pollingRef.current);
        }
      } catch (err: any) {
        setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
        setIsLoginModalOpen(false);
        if (pollingRef.current) clearInterval(pollingRef.current);
      }
    }, Math.max(3000, intervalSec * 1000));
  };

  const handleCloseLoginModal = () => {
    setIsLoginModalOpen(false);
    setDeviceInfo(null);
    if (pollingRef.current) clearInterval(pollingRef.current);
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      setProfile(null);
    } catch (err) {
      console.error("Logout failed:", err);
    }
  };

  const handleSaveConfig = async (updated: AppConfig) => {
    try {
      await invoke("save_config", { config: updated });
      setConfig(updated);
    } catch (err: any) {
      setErrorMsg(`Failed to save settings: ${err}`);
    }
  };

  const handleOpenFolder = async (path: string) => {
    try {
      await invoke("open_folder", { path });
    } catch (err) {
      console.error("Failed to open folder:", err);
    }
  };

  const handleOpenDiscord = async () => {
    try {
      await invoke("open_url", { url: "https://discord.gg/CFqskvhF8k" });
    } catch (err) {
      console.error("Failed to open Discord:", err);
    }
  };

  const handleCheckClientUpdate = async () => {
    setIsCheckingUpdate(true);
    setUpdateToast(null);
    try {
      const info = await invoke<{
        has_update: boolean;
        current_version: string;
        latest_version: string;
        download_url: string | null;
        release_url: string;
        changelog: string | null;
      }>("check_client_update");

      if (info.has_update) {
        if (info.download_url) {
          setUpdateToast(`Downloading ${info.latest_version}...`);
          const res = await invoke<string>("perform_client_update", {
            downloadUrl: info.download_url,
          });
          setUpdateToast(res);
        } else {
          setUpdateToast(`New version ${info.latest_version} available! Opening releases page...`);
          await invoke("open_url", { url: info.release_url });
        }
      } else {
        setUpdateToast(`Create Client is up to date (${info.current_version})`);
        setTimeout(() => setUpdateToast(null), 4000);
      }
    } catch (err: any) {
      setUpdateToast(`Update check failed: ${err}`);
      setTimeout(() => setUpdateToast(null), 4000);
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handlePlay = async () => {
    setErrorMsg(null);

    if (!profile) {
      handleLogin();
      return;
    }

    setIsSyncing(true);
    try {
      await invoke("sync_modpack", { forceRepair: false });

      setIsSyncing(false);
      setIsLaunching(true);
      await invoke("launch_game");

      setIsRunning(true);
      setIsLaunching(false);
      setLaunchSuccess(true);
      setTimeout(() => {
        setLaunchSuccess(false);
        setProgress(null);
      }, 3000);
    } catch (err: any) {
      setIsSyncing(false);
      setIsLaunching(false);
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    }
  };

  const handleForceRepair = async () => {
    setIsSettingsOpen(false);
    setIsSyncing(true);
    setErrorMsg(null);
    try {
      await invoke("sync_modpack", { forceRepair: true });
    } catch (err: any) {
      setErrorMsg(`Repair failed: ${err}`);
    } finally {
      setIsSyncing(false);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 MB";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  const formatSpeed = (bytesSec: number) => {
    if (bytesSec === 0) return "";
    const mbSec = bytesSec / (1024 * 1024);
    return `• ${mbSec.toFixed(1)} MB/s`;
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-[#0a0d13] text-[#e4e8ee] overflow-hidden font-sans select-none">
      {/* Top Header */}
      <Header
        profile={profile}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onLogin={handleLogin}
        onLogout={handleLogout}
        isLoggingIn={isLoggingIn}
        onCheckUpdate={handleCheckClientUpdate}
        isCheckingUpdate={isCheckingUpdate}
      />

      {/* Main Hero & Dashboard Area */}
      <main className="flex-1 relative flex flex-col justify-between p-8 overflow-hidden">
        {/* Background FancyMenu Atmosphere */}
        <div className="absolute inset-0 bg-radial from-[#12c3ca]/5 via-[#0e121a]/95 to-[#0a0d13] pointer-events-none" />
        <div className="absolute top-0 right-0 w-[550px] h-[550px] bg-[#12c3ca]/5 rounded-full blur-3xl pointer-events-none" />
        <div className="absolute bottom-0 left-0 w-[450px] h-[450px] bg-[#f59e0b]/5 rounded-full blur-3xl pointer-events-none" />

        {/* Top Floating Row: Modpack branding & Server Status */}
        <div className="relative z-10 flex items-start justify-between">
          <div className="space-y-1.5">
            <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-[#12c3ca]/10 border border-[#12c3ca]/40 text-[#5ce1e6] text-xs font-bold font-mono">
              <Flame className="w-3.5 h-3.5 text-[#f59e0b]" />
              <span>OFFICIAL SERVER MODPACK</span>
            </div>
            <h2 className="text-3xl font-extrabold tracking-tight drop-shadow-md">
              <span className="text-[#f59e0b]">Create </span>
              <span className="text-[#e4e8ee]">Modpack</span>
            </h2>
          </div>

          <ServerCard
            status={serverStatus}
            onRefresh={refreshServer}
            isLoading={isRefreshingServer}
          />
        </div>

        {/* Update Toast Banner */}
        {updateToast && (
          <div className="relative z-10 my-2 p-3 rounded-xl bg-[#12c3ca]/20 border border-[#12c3ca]/50 text-[#5ce1e6] text-xs flex items-center justify-between shadow-xl">
            <div className="flex items-center gap-2">
              <Check className="w-4 h-4 text-[#12c3ca] shrink-0" />
              <span>{updateToast}</span>
            </div>
            <button
              onClick={() => setUpdateToast(null)}
              className="text-[#8fa0bc] hover:text-[#e4e8ee] text-xs font-bold px-2 py-0.5 rounded cursor-pointer"
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Error Notification Banner */}
        {errorMsg && (
          <div className="relative z-10 my-4 p-3.5 rounded-xl bg-red-950/70 border border-red-500/50 text-red-200 text-xs flex items-center justify-between shadow-xl">
            <div className="flex items-center gap-2.5">
              <AlertCircle className="w-4 h-4 text-red-400 shrink-0" />
              <span>{errorMsg}</span>
            </div>
            <button
              onClick={() => setErrorMsg(null)}
              className="text-red-400 hover:text-red-200 text-xs font-bold px-2 py-0.5 rounded cursor-pointer"
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Bottom Section: Progress Bar & Big Play Button */}
        <div className="relative z-10 space-y-4 pt-6">
          {/* Progress / Download Status Bar */}
          {(isSyncing || progress) && (
            <div className="bg-[#0e121af5] border border-[#263042] rounded-xl p-3.5 backdrop-blur-md shadow-xl space-y-2">
              <div className="flex items-center justify-between text-xs">
                <span className="font-bold text-[#12c3ca]">
                  {progress?.task || "Preparing modpack..."}
                </span>
                <span className="font-mono text-[#8fa0bc]">
                  {progress && progress.total_bytes > 0
                    ? `${formatBytes(progress.current_bytes)} / ${formatBytes(
                        progress.total_bytes
                      )} (${progress.percentage.toFixed(0)}%) ${formatSpeed(
                        progress.speed_bytes_sec
                      )}`
                    : ""}
                </span>
              </div>
              <div className="w-full h-2.5 bg-[#182232] rounded-full overflow-hidden border border-[#263042]">
                <div
                  className="h-full bg-gradient-to-r from-[#0d949b] via-[#12c3ca] to-[#5ce1e6] transition-all duration-150 rounded-full shadow-[0_0_12px_rgba(18,195,202,0.6)]"
                  style={{
                    width: `${Math.min(100, Math.max(5, progress?.percentage ?? 10))}%`,
                  }}
                />
              </div>
            </div>
          )}

          {/* Controls Bar */}
          <div className="flex items-center justify-between gap-4">
            {/* Left Controls: Modpack Directory & Server Update Log */}
            <div className="flex items-center gap-2.5">
              <button
                onClick={() => handleOpenFolder(config?.game_dir || "")}
                className="px-4 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
                title="Open Modpack Game Directory"
              >
                <FolderOpen className="w-4 h-4 text-[#f59e0b]" />
                <span>Game Folder</span>
              </button>

              <button
                onClick={() => setIsChangelogOpen(true)}
                className="px-4 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
              >
                <FileText className="w-4 h-4 text-[#12c3ca]" />
                <span>Server Update Log & Notes</span>
              </button>
            </div>

            {/* Right Controls: Play Button & Discord */}
            <div className="flex items-center gap-3">
              {/* Launch / Play / Running Button */}
              <button
                onClick={handlePlay}
                disabled={isSyncing || isLaunching || isRunning}
                className={`h-14 px-10 rounded-2xl font-black text-sm tracking-wider uppercase transition-all duration-200 flex items-center justify-center gap-3 shadow-xl ${
                  isRunning
                    ? "bg-emerald-950/80 border border-emerald-500/60 text-emerald-300 cursor-not-allowed shadow-emerald-950/50"
                    : launchSuccess
                    ? "bg-emerald-600 text-white shadow-emerald-950/60"
                    : isSyncing || isLaunching
                    ? "bg-[#182232] text-[#8fa0bc] border border-[#263042] cursor-not-allowed opacity-80"
                    : !profile
                    ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 shadow-cyan-950/60 hover:scale-[1.02] cursor-pointer"
                    : "bg-gradient-to-r from-[#0d949b] via-[#12c3ca] to-[#5ce1e6] hover:brightness-110 text-slate-950 shadow-cyan-950/70 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
                }`}
              >
                {isRunning ? (
                  <>
                    <span className="w-3 h-3 rounded-full bg-emerald-400 animate-pulse shadow-[0_0_8px_#34d399]" />
                    <span>Running</span>
                  </>
                ) : launchSuccess ? (
                  <>
                    <CheckCircle2 className="w-5 h-5 text-white animate-bounce" />
                    <span>Minecraft Launched!</span>
                  </>
                ) : isLaunching ? (
                  <>
                    <div className="w-5 h-5 border-2 border-slate-900 border-t-white rounded-full animate-spin" />
                    <span>Launching Game...</span>
                  </>
                ) : isSyncing ? (
                  <>
                    <div className="w-5 h-5 border-2 border-slate-900 border-t-white rounded-full animate-spin" />
                    <span>Syncing Modpack...</span>
                  </>
                ) : !profile ? (
                  <>
                    <Play className="w-5 h-5 fill-current" />
                    <span>Sign In & Play</span>
                  </>
                ) : (
                  <>
                    <Play className="w-6 h-6 fill-current" />
                    <span>Play Game</span>
                  </>
                )}
              </button>

              {/* Discord Link Button (Bottom Right) */}
              <button
                onClick={handleOpenDiscord}
                className="h-14 px-5 rounded-2xl bg-[#5865F2]/20 hover:bg-[#5865F2]/35 border border-[#5865F2]/40 hover:border-[#5865F2] text-[#9ba5fe] hover:text-white flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-lg hover:scale-[1.02] active:scale-[0.98]"
                title="Join Create Server Discord"
              >
                <svg className="w-5 h-5 fill-current" viewBox="0 0 24 24">
                  <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.893.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                </svg>
                <span>Discord</span>
              </button>
            </div>
          </div>
        </div>
      </main>

      {/* Settings Modal */}
      {config && (
        <SettingsModal
          isOpen={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
          config={config}
          onSaveConfig={handleSaveConfig}
          onOpenFolder={handleOpenFolder}
          onForceRepair={handleForceRepair}
        />
      )}

      {/* Changelog Drawer */}
      <ChangelogDrawer
        isOpen={isChangelogOpen}
        onClose={() => setIsChangelogOpen(false)}
        version="v1.2.6"
      />

      {/* Device Code Login Modal */}
      <LoginModal
        isOpen={isLoginModalOpen}
        onClose={handleCloseLoginModal}
        deviceInfo={deviceInfo}
      />
    </div>
  );
};

export default App;
