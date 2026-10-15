import React from "react";
import { Settings, LogIn, LogOut, Cog } from "lucide-react";
import { MinecraftProfile } from "../types";

interface HeaderProps {
  profile: MinecraftProfile | null;
  onOpenSettings: () => void;
  onLogin: () => void;
  onLogout: () => void;
  isLoggingIn: boolean;
}

export const Header: React.FC<HeaderProps> = ({
  profile,
  onOpenSettings,
  onLogin,
  onLogout,
  isLoggingIn,
}) => {
  return (
    <header className="h-16 px-6 bg-[#182232]/95 border-b border-[#2a384c] flex items-center justify-between backdrop-blur-md z-20 shadow-md">
      {/* Brand & Logo */}
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
              Forge 1.20.1
            </span>
          </div>
          <p className="text-[11px] text-[#8fa0bc] tracking-tight">Dedicated Multiplayer Edition</p>
        </div>
      </div>

      {/* Right controls: User Profile & Settings */}
      <div className="flex items-center gap-3">
        {profile ? (
          <div className="flex items-center gap-3 bg-[#0e121a] border border-[#2a384c] hover:border-[#12c3ca]/60 transition-colors rounded-full pl-1.5 pr-3 py-1 shadow-inner">
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
              <span className="text-[10px] text-[#12c3ca] font-medium leading-tight">Ready to Play</span>
            </div>
            <button
              onClick={onLogout}
              title="Sign Out"
              className="ml-2 text-[#8fa0bc] hover:text-red-400 transition-colors p-1 rounded hover:bg-white/5 cursor-pointer"
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
            <span>{isLoggingIn ? "Check Your Browser..." : "Sign In with Microsoft"}</span>
          </button>
        )}

        {/* Settings button */}
        <button
          onClick={onOpenSettings}
          title="Launcher Settings"
          className="p-2.5 rounded-lg bg-[#0e121a] border border-[#2a384c] text-[#8fa0bc] hover:text-[#12c3ca] hover:border-[#12c3ca]/60 transition-all cursor-pointer shadow-sm hover:shadow-cyan-950/30"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
