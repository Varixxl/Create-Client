import React, { useState } from "react";
import { X, FolderOpen, Wrench, HelpCircle, Check, RotateCcw } from "lucide-react";
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
  const [clientId, setClientId] = useState(config.azure_client_id);
  const [showAzureGuide, setShowAzureGuide] = useState(false);
  const [savedSuccess, setSavedSuccess] = useState(false);

  if (!isOpen) return null;

  const handleSave = async () => {
    const updated: AppConfig = {
      ...config,
      ram_gb: ram,
      jvm_flags: jvmFlags,
      game_dir: gameDir,
      azure_client_id: clientId.trim(),
    };
    await onSaveConfig(updated);
    setSavedSuccess(true);
    setTimeout(() => {
      setSavedSuccess(false);
      onClose();
    }, 600);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <div className="bg-[#141920] border border-[#2b3644] w-full max-w-xl rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="px-6 py-4 border-b border-[#252f3c] flex items-center justify-between bg-[#181f29]">
          <h2 className="text-base font-bold text-slate-100 flex items-center gap-2">
            <span>Client Settings</span>
          </h2>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Form Body */}
        <div className="p-6 overflow-y-auto space-y-6 text-sm">
          {/* RAM Allocation Slider */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="font-semibold text-slate-200">Allocated RAM</label>
              <span className="font-mono text-xs px-2.5 py-1 rounded bg-amber-500/10 border border-amber-500/30 text-amber-400 font-bold">
                {ram} GB {ram >= 6 && ram <= 8 ? "(Recommended)" : ""}
              </span>
            </div>
            <input
              type="range"
              min="4"
              max="16"
              step="1"
              value={ram}
              onChange={(e) => setRam(Number(e.target.value))}
              className="w-full h-2 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-amber-500"
            />
            <div className="flex justify-between text-[11px] text-slate-500 mt-1 font-mono">
              <span>4 GB</span>
              <span>6 GB</span>
              <span>8 GB (Optimal)</span>
              <span>12 GB</span>
              <span>16 GB</span>
            </div>
          </div>

          {/* Java Runtime Flags */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="font-semibold text-slate-200">Java Runtime Flags (JVM)</label>
              <button
                type="button"
                onClick={() => setJvmFlags(DEFAULT_JVM_FLAGS)}
                className="text-[11px] text-amber-400 hover:text-amber-300 flex items-center gap-1 cursor-pointer"
              >
                <RotateCcw className="w-3 h-3" />
                Reset Defaults
              </button>
            </div>
            <textarea
              rows={3}
              value={jvmFlags}
              onChange={(e) => setJvmFlags(e.target.value)}
              className="w-full bg-[#0d1015] border border-[#2b3644] focus:border-amber-500/80 rounded-lg p-2.5 text-xs font-mono text-slate-300 focus:outline-none transition-colors resize-none"
              placeholder="-XX:+UseG1GC..."
            />
          </div>

          {/* Game Directory */}
          <div>
            <label className="block font-semibold text-slate-200 mb-2">Game Directory</label>
            <div className="flex gap-2">
              <input
                type="text"
                value={gameDir}
                onChange={(e) => setGameDir(e.target.value)}
                className="flex-1 bg-[#0d1015] border border-[#2b3644] rounded-lg px-3 py-2 text-xs font-mono text-slate-300 focus:outline-none focus:border-amber-500/80"
              />
              <button
                onClick={() => onOpenFolder(gameDir)}
                title="Open Folder in File Manager"
                className="px-3 py-2 bg-[#1b232e] hover:bg-[#232c3a] border border-[#2e3b4b] rounded-lg text-slate-300 flex items-center gap-1.5 text-xs font-medium transition-colors cursor-pointer"
              >
                <FolderOpen className="w-3.5 h-3.5 text-amber-400" />
                Open
              </button>
            </div>
          </div>

          {/* Microsoft Azure Client ID */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="font-semibold text-slate-200 flex items-center gap-1.5">
                <span>Microsoft Azure Client ID</span>
                <button
                  type="button"
                  onClick={() => setShowAzureGuide(!showAzureGuide)}
                  className="text-amber-400 hover:text-amber-300 cursor-pointer"
                  title="View setup guide"
                >
                  <HelpCircle className="w-3.5 h-3.5" />
                </button>
              </label>
            </div>
            <input
              type="text"
              placeholder="e.g. 00000000-0000-0000-0000-000000000000"
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
              className="w-full bg-[#0d1015] border border-[#2b3644] rounded-lg px-3 py-2 text-xs font-mono text-slate-300 focus:outline-none focus:border-amber-500/80"
            />
            {showAzureGuide && (
              <div className="mt-2.5 p-3 rounded-lg bg-amber-500/5 border border-amber-500/20 text-xs text-amber-200/90 space-y-1">
                <p className="font-semibold text-amber-400">Quick Azure Setup (Free):</p>
                <ol className="list-decimal pl-4 space-y-0.5 text-[11px] text-slate-300">
                  <li>Visit portal.azure.com and open <b>App registrations</b>.</li>
                  <li>Click <b>+ New registration</b>, name it <i>Create Client</i>.</li>
                  <li>Set Account type: <i>Personal Microsoft accounts only</i>.</li>
                  <li>Set Redirect URI: <i>Mobile and desktop applications</i> &rarr; <code>http://localhost:8483/callback</code></li>
                  <li>Copy Application (client) ID and paste here.</li>
                </ol>
              </div>
            )}
          </div>

          {/* Maintenance / Repair */}
          <div className="pt-2 border-t border-[#252f3c]">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-slate-200">Troubleshooting</h3>
                <p className="text-xs text-slate-400">Force re-verify and repair local modpack files</p>
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
        <div className="px-6 py-4 bg-[#181f29] border-t border-[#252f3c] flex items-center justify-end gap-3">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-lg text-xs font-medium text-slate-400 hover:text-slate-200 hover:bg-white/5 transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="px-5 py-2 rounded-lg bg-amber-600 hover:bg-amber-500 text-slate-950 font-bold text-xs flex items-center gap-1.5 transition-colors cursor-pointer shadow-md"
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
