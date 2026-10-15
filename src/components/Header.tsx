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
    <header className="h-16 px-6 bg-[#12161c]/90 border-b border-[#252c37] flex items-center justify-between backdrop-blur-md z-20">
      {/* Brand & Logo */}
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-amber-600 via-amber-700 to-amber-900 border border-amber-500/40 flex items-center justify-center shadow-lg shadow-amber-950/40">
          <Cog className="w-6 h-6 text-amber-200 animate-spin-slow" />
        </div>
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-lg font-bold tracking-wider text-slate-100 font-mono">
              CREATE<span className="text-amber-500">CLIENT</span>
            </h1>
            <span className="text-[10px] font-semibold tracking-wider uppercase px-1.5 py-0.5 rounded bg-amber-500/10 border border-amber-500/30 text-amber-400">
              Forge 1.20.1
            </span>
          </div>
          <p className="text-xs text-slate-400 tracking-tight">Dedicated Modpack Launcher</p>
        </div>
      </div>

      {/* Right controls: User Profile & Settings */}
      <div className="flex items-center gap-4">
        {profile ? (
          <div className="flex items-center gap-3 bg-[#181e26] border border-[#2b3542] rounded-full pl-1.5 pr-3 py-1">
            <img
              src={`https://mc-heads.net/avatar/${profile.username}/32`}
              alt={profile.username}
              className="w-7 h-7 rounded-full bg-slate-800 border border-amber-500/40 shadow-sm"
              onError={(e) => {
                // Fallback to default avatar
                (e.target as HTMLImageElement).src =
                  "https://mc-heads.net/avatar/MHF_Steve/32";
              }}
            />
            <div className="flex flex-col">
              <span className="text-xs font-semibold text-slate-200 leading-none">
                {profile.username}
              </span>
              <span className="text-[10px] text-emerald-400 leading-tight">Signed in</span>
            </div>
            <button
              onClick={onLogout}
              title="Sign Out"
              className="ml-2 text-slate-400 hover:text-red-400 transition-colors p-1 rounded hover:bg-white/5"
            >
              <LogOut className="w-3.5 h-3.5" />
            </button>
          </div>
        ) : (
          <button
            onClick={onLogin}
            disabled={isLoggingIn}
            className="flex items-center gap-2 bg-gradient-to-r from-amber-600 to-amber-700 hover:from-amber-500 hover:to-amber-600 text-slate-900 font-semibold text-xs px-4 py-2 rounded-lg border border-amber-400/30 shadow-md shadow-amber-950/30 transition-all cursor-pointer disabled:opacity-50"
          >
            <LogIn className="w-4 h-4 text-slate-950" />
            <span>{isLoggingIn ? "Waiting for Browser..." : "Sign In with Microsoft"}</span>
          </button>
        )}

        {/* Settings button */}
        <button
          onClick={onOpenSettings}
          title="Launcher Settings"
          className="p-2.5 rounded-lg bg-[#181e26] border border-[#2b3542] text-slate-300 hover:text-amber-400 hover:border-amber-500/50 transition-all cursor-pointer shadow-sm"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
