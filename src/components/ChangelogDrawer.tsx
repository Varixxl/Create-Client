import React from "react";
import { X, Sparkles, ShieldCheck, Zap } from "lucide-react";

interface ChangelogDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  version: string;
}

export const ChangelogDrawer: React.FC<ChangelogDrawerProps> = ({
  isOpen,
  onClose,
  version,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-end bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-md h-full bg-[#13171e] border-l border-[#273241] p-6 shadow-2xl flex flex-col animate-in slide-in-from-right duration-200">
        <div className="flex items-center justify-between pb-4 border-b border-[#222b38]">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-amber-400" />
            <h2 className="font-bold text-slate-100 text-base">Modpack Updates</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto py-4 space-y-6 text-xs text-slate-300">
          <div className="p-4 rounded-xl bg-[#181f29] border border-[#2b3747] space-y-3">
            <div className="flex items-center justify-between">
              <span className="font-bold text-amber-400 text-sm">{version}</span>
              <span className="text-[10px] text-slate-400">Current Release</span>
            </div>
            <p className="text-slate-300 leading-relaxed">
              Welcome to the official <b>Create Server</b> modpack client for Forge 1.20.1!
            </p>

            <div className="space-y-2 pt-2 border-t border-[#232c3a]">
              <div className="flex items-start gap-2">
                <Zap className="w-3.5 h-3.5 text-amber-400 shrink-0 mt-0.5" />
                <span>Synchronized with the latest remote server configs and mod dependencies.</span>
              </div>
              <div className="flex items-start gap-2">
                <ShieldCheck className="w-3.5 h-3.5 text-emerald-400 shrink-0 mt-0.5" />
                <span>Player settings, server favorites, and keybindings are preserved across updates.</span>
              </div>
              <div className="flex items-start gap-2">
                <Sparkles className="w-3.5 h-3.5 text-amber-300 shrink-0 mt-0.5" />
                <span>Integrated FancyMenu layouts and performance optimizations pre-configured.</span>
              </div>
            </div>
          </div>
        </div>

        <div className="pt-4 border-t border-[#222b38] flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-[#1b232e] hover:bg-[#232c3a] border border-[#2e3b4b] rounded-lg text-slate-300 font-medium text-xs transition-colors cursor-pointer"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
