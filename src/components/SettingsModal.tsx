import React, { useState } from "react";
import { X, FolderOpen, Wrench, Check, RotateCcw } from "lucide-react";
import { AppConfig } from "../types";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  config: AppConfig;
  onSaveConfig: (updated: AppConfig) => Promise<void>;
  onOpenFolder: (path: string) => void;
  onForceRepair: () => void;
}

const DEFAULT_JVM_FLAGS =
  "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch";

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  config,
  onSaveConfig,
  onOpenFolder,
  onForceRepair,
}) => {
  const [ram, setRam] = useState(config.ram_gb);
  const [jvmFlags, setJvmFlags] = useState(config.jvm_flags);
  const [gameDir, setGameDir] = useState(config.game_dir);
  const [savedSuccess, setSavedSuccess] = useState(false);

  if (!isOpen) return null;

  const handleSave = async () => {
    const updated: AppConfig = {
      ...config,
      ram_gb: ram,
      jvm_flags: jvmFlags,
      game_dir: gameDir,
    };
    await onSaveConfig(updated);
    setSavedSuccess(true);
    setTimeout(() => {
      setSavedSuccess(false);
      onClose();
    }, 500);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
      <div className="bg-[#0e121af5] border border-[#263042] w-full max-w-lg rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
        {/* FancyMenu Modal Header */}
        <div className="px-5 py-3.5 bg-[#182232] border-b border-[#263042] flex items-center justify-between">
          <div className="flex items-center gap-1.5 font-mono text-sm">
            <span className="font-extrabold text-[#f59e0b]">CLIENT SETTINGS</span>
            <span className="text-[#3a465a]">|</span>
            <span className="font-semibold text-[#12c3ca]">Configuration</span>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded text-[#8fa0bc] hover:text-[#12c3ca] hover:bg-white/5 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Modal Form Body */}
        <div className="p-6 overflow-y-auto space-y-6 text-xs text-[#e4e8ee]">
          {/* RAM Allocation Slider */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="font-bold text-[#e4e8ee] uppercase tracking-wider text-[11px]">
                Allocated Memory (RAM)
              </label>
              <span className="font-mono text-xs px-2.5 py-0.5 rounded bg-[#12c3ca]/15 border border-[#12c3ca]/50 text-[#5ce1e6] font-bold">
                {ram} GB {ram >= 6 && ram <= 8 ? "• Recommended" : ""}
              </span>
            </div>
            <input
              type="range"
              min="4"
              max="16"
              step="1"
              value={ram}
              onChange={(e) => setRam(Number(e.target.value))}
              className="w-full h-2 bg-[#1c2432] rounded-lg appearance-none cursor-pointer accent-[#12c3ca]"
            />
            <div className="flex justify-between text-[10px] text-[#8fa0bc] font-mono">
              <span>4 GB</span>
              <span>6 GB</span>
              <span>8 GB (Optimal)</span>
              <span>12 GB</span>
              <span>16 GB</span>
            </div>
          </div>

          {/* Java Runtime Flags */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="font-bold text-[#e4e8ee] uppercase tracking-wider text-[11px]">
                Java Virtual Machine (JVM) Flags
              </label>
              <button
                type="button"
                onClick={() => setJvmFlags(DEFAULT_JVM_FLAGS)}
                className="text-[11px] text-[#12c3ca] hover:text-[#5ce1e6] flex items-center gap-1 cursor-pointer transition-colors"
              >
                <RotateCcw className="w-3 h-3" />
                Reset Defaults
              </button>
            </div>
            <textarea
              rows={3}
              value={jvmFlags}
              onChange={(e) => setJvmFlags(e.target.value)}
              className="w-full bg-[#1c2432] border border-[#263042] focus:border-[#12c3ca] rounded-lg p-2.5 text-xs font-mono text-[#e4e8ee] focus:outline-none transition-colors resize-none"
              placeholder="-XX:+UseG1GC..."
            />
          </div>

          {/* Game Directory */}
          <div className="space-y-2">
            <label className="block font-bold text-[#e4e8ee] uppercase tracking-wider text-[11px]">
              Game Installation Directory
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={gameDir}
                onChange={(e) => setGameDir(e.target.value)}
                className="flex-1 bg-[#1c2432] border border-[#263042] focus:border-[#12c3ca] rounded-lg px-3 py-2 text-xs font-mono text-[#e4e8ee] focus:outline-none"
              />
              <button
                onClick={() => onOpenFolder(gameDir)}
                title="Open Folder in File Manager"
                className="px-3 py-2 bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 rounded-lg text-[#e4e8ee] flex items-center gap-1.5 text-xs font-medium transition-colors cursor-pointer"
              >
                <FolderOpen className="w-3.5 h-3.5 text-[#12c3ca]" />
                Browse
              </button>
            </div>
          </div>

          {/* Troubleshooting */}
          <div className="pt-3 border-t border-[#1c2432]">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-bold text-[#e4e8ee] text-xs">Troubleshooting & Integrity</h3>
                <p className="text-[11px] text-[#8fa0bc]">Re-verify and repair corrupted modpack files</p>
              </div>
              <button
                type="button"
                onClick={onForceRepair}
                className="px-3 py-1.5 bg-red-500/10 hover:bg-red-500/20 text-red-300 border border-red-500/30 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <Wrench className="w-3.5 h-3.5" />
                Repair Files
              </button>
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="px-5 py-3.5 bg-[#182232] border-t border-[#263042] flex items-center justify-end gap-3">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-lg text-xs font-medium text-[#8fa0bc] hover:text-[#e4e8ee] hover:bg-white/5 transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="px-5 py-2 rounded-lg bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 font-extrabold text-xs flex items-center gap-1.5 transition-all cursor-pointer shadow-md shadow-cyan-950/40"
          >
            {savedSuccess ? (
              <>
                <Check className="w-4 h-4" />
                Saved!
              </>
            ) : (
              "Save Changes"
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
