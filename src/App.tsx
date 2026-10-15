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

  // Initialize
  useEffect(() => {
    loadConfig();
    checkAuth();
    refreshServer();

    // Listen to download and sync progress from Rust
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
      // If error indicates missing client ID, prompt to open settings
      if (typeof err === "string" && err.includes("Azure Client ID")) {
        setIsSettingsOpen(true);
      }
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

    // If not signed in, prompt login
    if (!profile) {
      handleLogin();
      return;
    }

    setIsSyncing(true);
    try {
      // 1. Sync modpack & ensure Java 17
      await invoke("sync_modpack", { forceRepair: false });

      // 2. Launch Minecraft
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
    <div className="flex flex-col h-screen w-screen bg-[#0e1115] text-slate-100 overflow-hidden font-sans select-none">
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
        {/* Background Atmosphere */}
        <div className="absolute inset-0 bg-radial from-amber-950/20 via-[#0e1115]/90 to-[#0e1115] pointer-events-none" />
        <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-amber-500/5 rounded-full blur-3xl pointer-events-none" />
        <div className="absolute bottom-0 left-0 w-[400px] h-[400px] bg-amber-600/5 rounded-full blur-3xl pointer-events-none" />

        {/* Top Floating Row: Server Status & Quick Actions */}
        <div className="relative z-10 flex items-start justify-between">
          <div className="space-y-1">
            <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-amber-500/10 border border-amber-500/30 text-amber-400 text-xs font-semibold">
              <Flame className="w-3.5 h-3.5" />
              <span>Dedicated Multiplayer Modpack</span>
            </div>
            <h2 className="text-3xl font-extrabold text-white tracking-tight drop-shadow-md">
              Create Modpack
            </h2>
            <p className="text-sm text-slate-400">
              Minecraft 1.20.1 Forge • Steam, Cogs & Industry
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
          <div className="relative z-10 my-4 p-3.5 rounded-xl bg-red-950/60 border border-red-500/40 text-red-200 text-xs flex items-center justify-between shadow-lg">
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
            <div className="bg-[#141920]/90 border border-[#273241] rounded-xl p-3.5 backdrop-blur-md shadow-lg space-y-2">
              <div className="flex items-center justify-between text-xs">
                <span className="font-semibold text-amber-400">
                  {progress?.task || "Preparing modpack..."}
                </span>
                <span className="font-mono text-slate-400">
                  {progress && progress.total_bytes > 0
                    ? `${formatBytes(progress.current_bytes)} / ${formatBytes(
                        progress.total_bytes
                      )} (${progress.percentage.toFixed(0)}%) ${formatSpeed(
                        progress.speed_bytes_sec
                      )}`
                    : ""}
                </span>
              </div>
              <div className="w-full h-2.5 bg-slate-900 rounded-full overflow-hidden border border-[#2b3747]">
                <div
                  className="h-full bg-gradient-to-r from-amber-600 via-amber-500 to-amber-400 transition-all duration-150 rounded-full shadow-[0_0_12px_rgba(245,158,11,0.5)]"
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
              className="px-4 py-3 rounded-xl bg-[#141920]/80 hover:bg-[#1a222c] border border-[#263140] text-slate-300 hover:text-amber-400 flex items-center gap-2 text-xs font-semibold transition-all cursor-pointer shadow-sm"
            >
              <FileText className="w-4 h-4 text-amber-400" />
              <span>Patch Notes & Updates</span>
            </button>

            {/* Launch / Play Button */}
            <button
              onClick={handlePlay}
              disabled={isSyncing || isLaunching}
              className={`h-14 px-10 rounded-2xl font-extrabold text-base tracking-wider uppercase transition-all duration-200 flex items-center justify-center gap-3 cursor-pointer shadow-xl ${
                launchSuccess
                  ? "bg-emerald-600 text-white shadow-emerald-950/50"
                  : isSyncing || isLaunching
                  ? "bg-slate-800 text-slate-400 border border-slate-700 cursor-not-allowed opacity-80"
                  : !profile
                  ? "bg-gradient-to-r from-amber-600 to-amber-700 hover:from-amber-500 hover:to-amber-600 text-slate-950 shadow-amber-950/60 hover:scale-[1.02]"
                  : "bg-gradient-to-r from-amber-500 via-amber-600 to-amber-700 hover:from-amber-400 hover:via-amber-500 hover:to-amber-600 text-slate-950 shadow-amber-950/60 hover:scale-[1.02] active:scale-[0.98]"
              }`}
            >
              {launchSuccess ? (
                <>
                  <CheckCircle2 className="w-5 h-5 text-white animate-bounce" />
                  <span>Minecraft Launched!</span>
                </>
              ) : isLaunching ? (
                <>
                  <div className="w-5 h-5 border-2 border-slate-400 border-t-amber-400 rounded-full animate-spin" />
                  <span>Launching Game...</span>
                </>
              ) : isSyncing ? (
                <>
                  <div className="w-5 h-5 border-2 border-slate-400 border-t-amber-400 rounded-full animate-spin" />
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
        version="v1.0.0"
      />
    </div>
  );
};

export default App;
