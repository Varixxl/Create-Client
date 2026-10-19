import React from "react";
import {
  Settings,
  LogIn,
  LogOut,
  Cog,
  Sparkles,
  RefreshCw,
  Play,
  Package,
  Terminal,
} from "lucide-react";
import {
  MinecraftProfile,
  ClientUpdateInfo,
  ModpackProfile,
  WorkspaceTab,
} from "../types";

interface HeaderProps {
  profile: MinecraftProfile | null;
  activeModpack: ModpackProfile | null;
  activeTab: WorkspaceTab;
  onChangeTab: (tab: WorkspaceTab) => void;
  onOpenSettings: () => void;
  onLogin: () => void;
  onLogout: () => void;
  isLoggingIn: boolean;
  clientUpdate: ClientUpdateInfo | null;
  onCheckUpdates: () => void;
  isCheckingUpdates: boolean;
  onUpdateClient: () => void;
  isUpdatingClient: boolean;
}

export const Header: React.FC<HeaderProps> = ({
  profile,
  activeModpack,
  activeTab,
  onChangeTab,
  onOpenSettings,
  onLogin,
  onLogout,
  isLoggingIn,
  clientUpdate,
  onCheckUpdates,
  isCheckingUpdates,
  onUpdateClient,
  isUpdatingClient,
}) => {
  const loaderLabel = activeModpack
    ? `${activeModpack.loader.toUpperCase()} ${
        activeModpack.mc_version === "latest"
          ? activeModpack.resolved_mc_version || "LATEST"
          : activeModpack.mc_version
      }`
    : "FORGE 1.20.1";

  return (
    <header className="h-16 px-6 bg-[#182232]/95 border-b border-[#2a384c] flex items-center justify-between backdrop-blur-md z-30 shadow-md shrink-0">
      {/* Left: Brand & Logo */}
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-[#1b2536] to-[#0e121a] border border-[#12c3ca]/50 flex items-center justify-center shadow-lg shadow-cyan-950/40">
          <Cog className="w-6 h-6 text-[#12c3ca] animate-spin-slow" />
        </div>
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-base font-extrabold tracking-wider font-mono">
              <span className="text-[#f59e0b]">CREATE</span>
              <span className="text-[#12c3ca]"> CLIENT</span>
            </h1>
            <span className="text-[10px] font-bold tracking-wider uppercase px-2 py-0.5 rounded bg-[#12c3ca]/10 border border-[#12c3ca]/40 text-[#5ce1e6]">
              {loaderLabel}
            </span>
          </div>
          <p className="text-[11px] text-[#8fa0bc] tracking-tight">
            Multi-Instance Modpack & Server Launcher
          </p>
        </div>
      </div>

      {/* Center: Segmented Workspace Switcher (Play | Mods | Console) */}
      <div className="flex items-center bg-[#0c1018] p-1 rounded-xl border border-[#263449] shadow-inner">
        <button
          onClick={() => onChangeTab("play")}
          className={`px-4 py-1.5 rounded-lg text-xs font-extrabold flex items-center gap-1.5 transition-all cursor-pointer ${
            activeTab === "play"
              ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] text-slate-950 shadow-md"
              : "text-[#8fa0bc] hover:text-[#e4e8ee]"
          }`}
        >
          <Play className="w-3.5 h-3.5 fill-current" />
          <span>Dashboard</span>
        </button>

        <button
          onClick={() => onChangeTab("mods")}
          className={`px-4 py-1.5 rounded-lg text-xs font-extrabold flex items-center gap-1.5 transition-all cursor-pointer ${
            activeTab === "mods"
              ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] text-slate-950 shadow-md"
              : "text-[#8fa0bc] hover:text-[#e4e8ee]"
          }`}
        >
          <Package className="w-3.5 h-3.5" />
          <span>Mods</span>
        </button>

        <button
          onClick={() => onChangeTab("console")}
          className={`px-4 py-1.5 rounded-lg text-xs font-extrabold flex items-center gap-1.5 transition-all cursor-pointer ${
            activeTab === "console"
              ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] text-slate-950 shadow-md"
              : "text-[#8fa0bc] hover:text-[#e4e8ee]"
          }`}
        >
          <Terminal className="w-3.5 h-3.5" />
          <span>Console</span>
        </button>
      </div>

      {/* Right controls: User Profile & Actions */}
      <div className="flex items-center gap-2.5">
        {profile ? (
          <div className="flex items-center gap-2.5 bg-[#0e121a] border border-[#2a384c] hover:border-[#12c3ca]/60 transition-colors rounded-full pl-1.5 pr-3 py-1 shadow-inner">
            <img
              src={`https://mc-heads.net/avatar/${profile.username}/32`}
              alt={profile.username}
              className="w-7 h-7 rounded-full bg-slate-800 border border-[#12c3ca]/60 shadow-sm"
              onError={(e) => {
                (e.target as HTMLImageElement).src =
                  "https://mc-heads.net/avatar/MHF_Steve/32";
              }}
            />
            <div className="flex flex-col">
              <span className="text-xs font-bold text-[#e4e8ee] leading-none">
                {profile.username}
              </span>
              <span className="text-[10px] text-[#12c3ca] font-medium leading-tight">
                Ready to Play
              </span>
            </div>
            <button
              onClick={onLogout}
              title="Sign Out"
              className="ml-1.5 text-[#8fa0bc] hover:text-red-400 transition-colors p-1 rounded hover:bg-white/5 cursor-pointer"
            >
              <LogOut className="w-3.5 h-3.5" />
            </button>
          </div>
        ) : (
          <button
            onClick={onLogin}
            disabled={isLoggingIn}
            className="flex items-center gap-2 bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 font-bold text-xs px-4 py-2 rounded-lg border border-cyan-300/40 shadow-md shadow-cyan-950/40 transition-all cursor-pointer disabled:opacity-50 hover:scale-[1.02]"
          >
            <LogIn className="w-4 h-4 text-slate-950" />
            <span>
              {isLoggingIn ? "Check Your Browser..." : "Sign In with Microsoft"}
            </span>
          </button>
        )}

        {/* Check for Updates button */}
        <button
          onClick={onCheckUpdates}
          disabled={isCheckingUpdates}
          title="Check for Launcher and Modpack Updates"
          className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-[#0e121a] border border-[#2a384c] text-[#8fa0bc] hover:text-[#12c3ca] hover:border-[#12c3ca]/60 transition-all cursor-pointer shadow-sm hover:shadow-cyan-950/30 text-xs font-bold disabled:opacity-60"
        >
          <RefreshCw
            className={`w-3.5 h-3.5 text-[#12c3ca] ${
              isCheckingUpdates ? "animate-spin" : ""
            }`}
          />
          <span className="hidden xl:inline">Check Updates</span>
        </button>

        {/* Update Client / Update Available button */}
        {clientUpdate?.has_update ? (
          <button
            onClick={onUpdateClient}
            disabled={isUpdatingClient}
            title={`Install Create Client ${clientUpdate.latest_version}`}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-gradient-to-r from-amber-500/25 via-amber-400/25 to-amber-500/25 border border-amber-400/80 text-amber-300 hover:brightness-125 transition-all cursor-pointer shadow-lg shadow-amber-950/50 text-xs font-extrabold animate-pulse"
          >
            <Sparkles
              className={`w-3.5 h-3.5 text-amber-300 ${
                isUpdatingClient ? "animate-spin" : ""
              }`}
            />
            <span>
              {isUpdatingClient ? "Updating Client..." : "Update Available"}
            </span>
          </button>
        ) : null}

        {/* Settings button */}
        <button
          onClick={onOpenSettings}
          title="Global Launcher Settings"
          className="p-2.5 rounded-lg bg-[#0e121a] border border-[#2a384c] text-[#8fa0bc] hover:text-[#12c3ca] hover:border-[#12c3ca]/60 transition-all cursor-pointer shadow-sm hover:shadow-cyan-950/30"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
