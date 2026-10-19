import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Server, Sparkles, ShieldCheck, Check } from "lucide-react";
import { ModpackProfile, ServerInstance } from "../types";

interface CreateServerModalProps {
  isOpen: boolean;
  profile: ModpackProfile;
  onClose: () => void;
  onServerCreated: (server: ServerInstance) => void;
  onError: (msg: string) => void;
}

export const CreateServerModal: React.FC<CreateServerModalProps> = React.memo(({
  isOpen,
  profile,
  onClose,
  onServerCreated,
  onError,
}) => {
  const [serverName, setServerName] = useState(`${profile.name} Server`);
  const [isCreating, setIsCreating] = useState(false);

  if (!isOpen) return null;

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!serverName.trim()) return;

    setIsCreating(true);
    try {
      const server = await invoke<ServerInstance>("create_server_for_profile", {
        profileId: profile.id,
        serverName: serverName.trim(),
      });
      onServerCreated(server);
      onClose();
    } catch (err: any) {
      onError(`Failed to create server: ${err}`);
    } finally {
      setIsCreating(false);
    }
  };

  const mcVer =
    profile.mc_version === "latest"
      ? profile.resolved_mc_version || "Latest Stable"
      : profile.mc_version;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
      <div className="w-full max-w-lg bg-[#0e131d] border border-[#2a384c] rounded-2xl shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="px-6 py-4 bg-[#141c2b] border-b border-[#243144] flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/20 border border-emerald-400/40 flex items-center justify-center text-emerald-300">
              <Server className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-extrabold tracking-wide uppercase text-[#e4e8ee]">
                  Create Dedicated Server
                </h2>
                <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-amber-500/20 border border-amber-500/40 text-amber-300">
                  BETA
                </span>
              </div>
              <p className="text-[11px] text-[#8fa0bc]">
                From Profile: {profile.name} ({profile.loader.toUpperCase()} {mcVer})
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={isCreating}
            className="text-[#8fa0bc] hover:text-white p-1 rounded-lg hover:bg-white/5 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleCreate} className="p-6 space-y-4">
          {/* Compatibility & Smart Filtering Notice */}
          <div className="p-3.5 rounded-xl bg-amber-500/10 border border-amber-500/30 space-y-2">
            <div className="flex items-center gap-2 text-xs font-bold text-amber-300">
              <ShieldCheck className="w-4 h-4 text-emerald-400 shrink-0" />
              <span>Smart Client-Only Mod Filtering</span>
            </div>
            <p className="text-[11px] text-[#c6d0df] leading-relaxed">
              Create Client automatically filters out client-only rendering engines, HUDs,
              FancyMenu, minimaps, and shaders so your server starts cleanly without crashing.
            </p>
            <div className="flex items-center gap-3 pt-1 text-[10px] font-mono text-[#8fa0bc]">
              <span className="flex items-center gap-1">
                <Check className="w-3 h-3 text-emerald-400" /> Pre-accepted EULA
              </span>
              <span className="flex items-center gap-1">
                <Check className="w-3 h-3 text-emerald-400" /> Start scripts (run.sh/run.bat)
              </span>
              <span className="flex items-center gap-1">
                <Check className="w-3 h-3 text-emerald-400" /> Port 25565
              </span>
            </div>
          </div>

          {/* Server Instance Name */}
          <div className="space-y-1.5">
            <label className="text-xs font-bold text-[#c6d0df] block">
              Server Instance Name
            </label>
            <input
              type="text"
              value={serverName}
              onChange={(e) => setServerName(e.target.value)}
              disabled={isCreating}
              placeholder="e.g., My Survival World Server"
              className="w-full px-3.5 py-2.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none disabled:opacity-60"
              required
            />
          </div>

          {/* Target Info */}
          <div className="p-3 rounded-xl bg-[#090d14] border border-[#222d3f] flex items-center justify-between text-xs">
            <span className="text-[#8fa0bc]">Loader & Game Version:</span>
            <span className="font-mono font-bold text-[#5ce1e6]">
              {profile.loader.toUpperCase()} {mcVer}
            </span>
          </div>

          {/* Footer Actions */}
          <div className="pt-3 flex items-center justify-end gap-2.5 border-t border-[#1e293b]">
            <button
              type="button"
              onClick={onClose}
              disabled={isCreating}
              className="px-4 py-2 rounded-xl bg-[#182232] hover:bg-[#222f43] text-xs font-bold text-[#8fa0bc] hover:text-white transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isCreating || !serverName.trim()}
              className="px-5 py-2 rounded-xl bg-gradient-to-r from-emerald-500 to-teal-400 hover:from-emerald-400 hover:to-teal-300 text-slate-950 font-extrabold text-xs transition-all cursor-pointer shadow-lg shadow-emerald-950/50 disabled:opacity-50 flex items-center gap-2"
            >
              {isCreating ? (
                <>
                  <Sparkles className="w-3.5 h-3.5 animate-spin" />
                  <span>Generating Server Files...</span>
                </>
              ) : (
                <>
                  <Server className="w-3.5 h-3.5" />
                  <span>Create Server (Beta)</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
});
