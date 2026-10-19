import React, { useState, useEffect } from "react";
import { X, FolderOpen, Wrench, Check, RotateCcw, Trash2, AlertTriangle, Monitor, Sparkles, Key, ExternalLink, RefreshCw } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { AppConfig, GraphicsInfo } from "../types";

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  config: AppConfig;
  onSaveConfig: (updated: AppConfig) => Promise<void>;
  onOpenFolder: (path: string) => void;
  onForceRepair: () => void;
  onDeleteAllFiles: () => Promise<void>;
  onResetDefaults: () => Promise<void>;
  onRefreshAuth?: () => void;
  isRefreshingAuth?: boolean;
}

const DEFAULT_JVM_FLAGS = "-XX:+UseZGC -XX:+ZGenerational -XX:+AlwaysPreTouch";

export const SettingsModal: React.FC<SettingsModalProps> = React.memo(({
  isOpen,
  onClose,
  config,
  onSaveConfig,
  onOpenFolder,
  onForceRepair,
  onDeleteAllFiles,
  onResetDefaults,
  onRefreshAuth,
  isRefreshingAuth,
}) => {
  const [ram, setRam] = useState(config.ram_gb);
  const [jvmFlags, setJvmFlags] = useState(config.jvm_flags);
  const [gameDir, setGameDir] = useState(config.game_dir);
  const [graphicsPreset, setGraphicsPreset] = useState(config.graphics_preset || "auto");
  const [curseforgeApiKey, setCurseforgeApiKey] = useState(config.curseforge_api_key || "");
  const [graphicsInfo, setGraphicsInfo] = useState<GraphicsInfo | null>(null);
  const [savedSuccess, setSavedSuccess] = useState(false);

  const [isConfirmingDelete, setIsConfirmingDelete] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isResettingDefaults, setIsResettingDefaults] = useState(false);
  const [resetSuccess, setResetSuccess] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setRam(config.ram_gb);
      setJvmFlags(config.jvm_flags);
      setGameDir(config.game_dir);
      setGraphicsPreset(config.graphics_preset || "auto");
      setCurseforgeApiKey(config.curseforge_api_key || "");
      setIsConfirmingDelete(false);

      invoke<GraphicsInfo>("get_graphics_info")
        .then((info) => setGraphicsInfo(info))
        .catch((err) => console.warn("Failed to fetch graphics info:", err));
    }
  }, [isOpen, config]);

  if (!isOpen) return null;

  const handleSave = async () => {
    const updated: AppConfig = {
      ...config,
      ram_gb: ram,
      jvm_flags: jvmFlags,
      game_dir: gameDir,
      graphics_preset: graphicsPreset,
      curseforge_api_key: curseforgeApiKey,
    };
    await onSaveConfig(updated);
    setSavedSuccess(true);
    setTimeout(() => {
      setSavedSuccess(false);
      onClose();
    }, 500);
  };

  const handleResetDefaults = async () => {
    setIsResettingDefaults(true);
    try {
      await onResetDefaults();
      setResetSuccess(true);
      setTimeout(() => setResetSuccess(false), 2500);
    } finally {
      setIsResettingDefaults(false);
    }
  };

  const handleDeleteAll = async () => {
    setIsDeleting(true);
    try {
      await onDeleteAllFiles();
      setIsConfirmingDelete(false);
      onClose();
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
      <div className="bg-[#0e121a] border border-[#263042] w-full max-w-lg rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[88vh]">
        {/* Modal Header */}
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
          {/* Graphics Preset Selector */}
          <div className="space-y-2.5 p-3.5 bg-[#141b26] border border-[#263042] rounded-xl">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5 text-[#12c3ca]" />
                <label className="font-bold text-[#e4e8ee] uppercase tracking-wider text-[11px]">
                  Graphics & Performance Preset
                </label>
              </div>
              {graphicsInfo && (
                <div className="flex items-center gap-1 text-[10px] font-mono text-[#5ce1e6] bg-[#12c3ca]/10 px-2 py-0.5 rounded border border-[#12c3ca]/30">
                  <Monitor className="w-3 h-3 text-[#12c3ca]" />
                  <span>{graphicsInfo.detected_refresh_rate}Hz Sync</span>
                </div>
              )}
            </div>

            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                onClick={() => setGraphicsPreset("auto")}
                className={`p-2.5 rounded-lg border text-left transition-all cursor-pointer flex flex-col justify-between ${
                  graphicsPreset === "auto"
                    ? "bg-[#182a3a] border-[#12c3ca] shadow-sm shadow-[#12c3ca]/20"
                    : "bg-[#1c2432] border-[#263042] hover:border-[#12c3ca]/50 text-[#8fa0bc]"
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-[#e4e8ee] text-xs">Auto Tier</span>
                  <span className="text-[9px] px-1.5 py-0.2 bg-[#12c3ca]/20 text-[#5ce1e6] rounded uppercase font-mono">
                    {graphicsInfo?.detected_tier || "DETECTED"}
                  </span>
                </div>
                <p className="text-[10px] text-[#8fa0bc] mt-1">
                  Auto-detects GPU capabilities on launch
                </p>
              </button>

              <button
                type="button"
                onClick={() => setGraphicsPreset("low")}
                className={`p-2.5 rounded-lg border text-left transition-all cursor-pointer flex flex-col justify-between ${
                  graphicsPreset === "low"
                    ? "bg-[#182a3a] border-[#12c3ca] shadow-sm shadow-[#12c3ca]/20"
                    : "bg-[#1c2432] border-[#263042] hover:border-[#12c3ca]/50 text-[#8fa0bc]"
                }`}
              >
                <span className="font-bold text-[#e4e8ee] text-xs">Low (Fast)</span>
                <p className="text-[10px] text-[#8fa0bc] mt-1">
                  DH 32c • POM Off • Laptops & Budget GPUs
                </p>
              </button>

              <button
                type="button"
                onClick={() => setGraphicsPreset("medium")}
                className={`p-2.5 rounded-lg border text-left transition-all cursor-pointer flex flex-col justify-between ${
                  graphicsPreset === "medium"
                    ? "bg-[#182a3a] border-[#12c3ca] shadow-sm shadow-[#12c3ca]/20"
                    : "bg-[#1c2432] border-[#263042] hover:border-[#12c3ca]/50 text-[#8fa0bc]"
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-[#e4e8ee] text-xs">Medium</span>
                  <span className="text-[9px] px-1.5 py-0.2 bg-emerald-500/20 text-emerald-300 rounded font-mono">
                    SWEET SPOT
                  </span>
                </div>
                <p className="text-[10px] text-[#8fa0bc] mt-1">
                  DH 64c + Fog • Patrix POM 3D • 1440p Smooth
                </p>
              </button>

              <button
                type="button"
                onClick={() => setGraphicsPreset("ultra")}
                className={`p-2.5 rounded-lg border text-left transition-all cursor-pointer flex flex-col justify-between ${
                  graphicsPreset === "ultra"
                    ? "bg-[#182a3a] border-[#12c3ca] shadow-sm shadow-[#12c3ca]/20"
                    : "bg-[#1c2432] border-[#263042] hover:border-[#12c3ca]/50 text-[#8fa0bc]"
                }`}
              >
                <span className="font-bold text-[#e4e8ee] text-xs">Ultra (Cinematic)</span>
                <p className="text-[10px] text-[#8fa0bc] mt-1">
                  Full POM Relief • High Shadows • 4K & High-End
                </p>
              </button>
            </div>

            {graphicsInfo?.detected_gpu && (
              <div className="text-[10px] text-[#6b7b95] font-mono flex items-center justify-between pt-1">
                <span className="truncate max-w-[280px]" title={graphicsInfo.detected_gpu}>
                  GPU: {graphicsInfo.detected_gpu}
                </span>
                <span>Framerate locked to {graphicsInfo.detected_refresh_rate} FPS</span>
              </div>
            )}
          </div>

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
              placeholder="-XX:+UseZGC -XX:+AlwaysPreTouch"
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

          {/* CurseForge API Key */}
          <div className="space-y-2.5 p-3.5 bg-[#141b26] border border-[#263042] rounded-xl">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Key className="w-3.5 h-3.5 text-amber-400" />
                <label className="font-bold text-[#e4e8ee] uppercase tracking-wider text-[11px]">
                  CurseForge API Key (User Provided)
                </label>
              </div>
              <a
                href="https://console.curseforge.com/"
                target="_blank"
                rel="noreferrer"
                className="text-[10px] text-[#12c3ca] hover:text-[#5ce1e6] flex items-center gap-1 font-mono transition-colors"
              >
                <span>Get API Key</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            </div>
            <p className="text-[11px] text-[#8fa0bc] leading-relaxed">
              CurseForge requires a personal API key to search and install content from their platform. Get your free key at the CurseForge Console and paste it here to enable CurseForge search.
            </p>
            <input
              type="password"
              value={curseforgeApiKey}
              onChange={(e) => setCurseforgeApiKey(e.target.value)}
              placeholder="$2a$10$..."
              className="w-full bg-[#1c2432] border border-[#263042] focus:border-[#12c3ca] rounded-lg px-3 py-2 text-xs font-mono text-[#e4e8ee] focus:outline-none"
            />
          </div>

          {/* Troubleshooting & Integrity */}
          <div className="pt-3 border-t border-[#1c2432] space-y-3">
            <div>
              <h3 className="font-bold text-[#e4e8ee] text-xs">Troubleshooting & Integrity</h3>
              <p className="text-[11px] text-[#8fa0bc]">Re-verify modpack files or restore modpack video/graphic defaults</p>
            </div>
            <div className="flex flex-wrap gap-2.5">
              <button
                type="button"
                onClick={onForceRepair}
                className="px-3.5 py-2 bg-[#182232] hover:bg-[#242e3e] text-amber-300 border border-amber-500/40 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer shadow-sm"
              >
                <Wrench className="w-3.5 h-3.5 text-amber-400" />
                <span>Verify & Repair Files</span>
              </button>
              <button
                type="button"
                onClick={handleResetDefaults}
                disabled={isResettingDefaults}
                className="px-3.5 py-2 bg-[#182232] hover:bg-[#242e3e] text-[#5ce1e6] border border-[#12c3ca]/40 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer shadow-sm"
              >
                <RotateCcw className={`w-3.5 h-3.5 text-[#12c3ca] ${isResettingDefaults ? "animate-spin" : ""}`} />
                <span>{resetSuccess ? "Settings Restored to Defaults!" : isResettingDefaults ? "Restoring..." : "Reset to Modpack Defaults"}</span>
              </button>
              {onRefreshAuth && (
                <button
                  type="button"
                  onClick={onRefreshAuth}
                  disabled={isRefreshingAuth}
                  className="px-3.5 py-2 bg-[#182232] hover:bg-[#242e3e] text-[#12c3ca] border border-[#12c3ca]/40 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer shadow-sm disabled:opacity-60"
                >
                  <RefreshCw className={`w-3.5 h-3.5 text-[#12c3ca] ${isRefreshingAuth ? "animate-spin" : ""}`} />
                  <span>{isRefreshingAuth ? "Refreshing Session..." : "Refresh Authorization"}</span>
                </button>
              )}
            </div>
          </div>

          {/* Danger Zone: Complete File Reset */}
          <div className="pt-3 border-t border-red-950/50">
            <div className="p-4 rounded-xl bg-red-950/20 border border-red-500/40 space-y-3">
              <div className="flex items-center gap-2 text-red-400 font-bold text-xs uppercase tracking-wider">
                <AlertTriangle className="w-4 h-4 text-red-400" />
                <span>Danger Zone</span>
              </div>
              <p className="text-[11px] text-[#8fa0bc] leading-relaxed">
                Completely deletes all game files, mods, configs, saves, and screenshots related to Create Client for a 100% clean factory reset.
              </p>

              {isConfirmingDelete ? (
                <div className="p-3 bg-red-950/80 border border-red-500 rounded-lg space-y-2.5">
                  <p className="text-xs font-bold text-red-200">
                    Are you sure? This will delete all files related to Create Client and mods. This action cannot be undone!
                  </p>
                  <div className="flex items-center gap-2 pt-1">
                    <button
                      type="button"
                      disabled={isDeleting}
                      onClick={handleDeleteAll}
                      className="px-3.5 py-1.5 bg-red-600 hover:bg-red-500 text-white font-bold text-xs rounded-lg transition-colors cursor-pointer flex items-center gap-1.5 shadow-md shadow-red-950/60"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                      <span>{isDeleting ? "Deleting All Files..." : "Yes, Delete Everything"}</span>
                    </button>
                    <button
                      type="button"
                      disabled={isDeleting}
                      onClick={() => setIsConfirmingDelete(false)}
                      className="px-3.5 py-1.5 bg-[#182232] hover:bg-[#242e3e] text-[#8fa0bc] hover:text-[#e4e8ee] border border-[#263042] text-xs font-medium rounded-lg transition-colors cursor-pointer"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => setIsConfirmingDelete(true)}
                  className="px-3.5 py-2 bg-red-500/15 hover:bg-red-500/25 text-red-300 border border-red-500/40 rounded-lg text-xs font-bold flex items-center gap-2 transition-all cursor-pointer shadow-sm hover:border-red-400"
                >
                  <Trash2 className="w-4 h-4 text-red-400" />
                  <span>Delete All Files</span>
                </button>
              )}
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
});
