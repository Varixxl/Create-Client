import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Header } from "./components/Header";
import { ServerCard } from "./components/ServerCard";
import { SettingsModal } from "./components/SettingsModal";
import { ChangelogDrawer } from "./components/ChangelogDrawer";
import { Play, Flame, AlertCircle, FileText, CheckCircle2 } from "lucide-react";
import {
  AppConfig,
  AuthStatus,
  MinecraftProfile,
  ProgressPayload,
  ServerStatus,
} from "./types";

export const App: React.FC = () => {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [profile, setProfile] = useState<MinecraftProfile | null>(null);
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);

  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isSyncing, setIsSyncing] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);
  const [isRefreshingServer, setIsRefreshingServer] = useState(false);
  const [launchSuccess, setLaunchSuccess] = useState(false);

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isChangelogOpen, setIsChangelogOpen] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    loadConfig();
    checkAuth();
    refreshServer();

    const unlistenPromise = listen<ProgressPayload>("download-progress", (event) => {
      setProgress(event.payload);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
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
      const newProfile = await invoke<MinecraftProfile>("login", { clientId: null });
      setProfile(newProfile);
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsLoggingIn(false);
    }
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

      setLaunchSuccess(true);
      setTimeout(() => {
        setIsLaunching(false);
        setLaunchSuccess(false);
        setProgress(null);
      }, 4000);
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
            <p className="text-xs text-[#8fa0bc] font-medium">
              Minecraft 1.20.1 Forge 47.4.20 • Steam, Cogs & Industry
            </p>
          </div>

          <ServerCard
            status={serverStatus}
            onRefresh={refreshServer}
            isLoading={isRefreshingServer}
          />
        </div>

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
            {/* Changelog / Notes Button */}
            <button
              onClick={() => setIsChangelogOpen(true)}
              className="px-4 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
            >
              <FileText className="w-4 h-4 text-[#12c3ca]" />
              <span>Server Update Log & Notes</span>
            </button>

            {/* Launch / Play Button */}
            <button
              onClick={handlePlay}
              disabled={isSyncing || isLaunching}
              className={`h-14 px-10 rounded-2xl font-black text-sm tracking-wider uppercase transition-all duration-200 flex items-center justify-center gap-3 cursor-pointer shadow-xl ${
                launchSuccess
                  ? "bg-emerald-600 text-white shadow-emerald-950/60"
                  : isSyncing || isLaunching
                  ? "bg-[#182232] text-[#8fa0bc] border border-[#263042] cursor-not-allowed opacity-80"
                  : !profile
                  ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 shadow-cyan-950/60 hover:scale-[1.02]"
                  : "bg-gradient-to-r from-[#0d949b] via-[#12c3ca] to-[#5ce1e6] hover:brightness-110 text-slate-950 shadow-cyan-950/70 hover:scale-[1.02] active:scale-[0.98]"
              }`}
            >
              {launchSuccess ? (
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
    </div>
  );
};

export default App;
