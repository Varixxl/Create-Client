import React from "react";
import { X, ExternalLink, Calendar, CheckCircle, Zap, Shield, Sparkles } from "lucide-react";

interface ChangelogDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  version: string;
}

export const ChangelogDrawer: React.FC<ChangelogDrawerProps> = ({
  isOpen,
  onClose,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-end bg-black/70 backdrop-blur-sm">
      <div className="w-full max-w-xl h-full bg-[#0e121af8] border-l border-[#263042] shadow-2xl flex flex-col animate-in slide-in-from-right duration-200">
        {/* FancyMenu Header Bar */}
        <div className="bg-[#182232] px-6 py-4 border-b border-[#263042] flex items-center justify-between shadow-md">
          <div className="flex items-center gap-1.5 font-mono text-sm">
            <span className="font-extrabold text-[#f59e0b]">SERVER UPDATE LOG</span>
            <span className="text-[#3a465a]">|</span>
            <span className="font-semibold text-[#12c3ca]">Create Server</span>
          </div>
          <div className="flex items-center gap-2">
            <a
              href="https://discord.gg/CFqskvhF8k"
              target="_blank"
              rel="noreferrer"
              className="px-2.5 py-1 rounded bg-[#5865F2]/20 hover:bg-[#5865F2]/30 border border-[#5865F2]/50 text-[#9ba5fe] hover:text-white text-[11px] font-bold flex items-center gap-1.5 transition-colors"
            >
              <ExternalLink className="w-3 h-3" />
              Discord
            </a>
            <button
              onClick={onClose}
              className="p-1 rounded text-[#8fa0bc] hover:text-[#12c3ca] hover:bg-white/5 transition-colors cursor-pointer"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Patch notes list */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6 text-xs text-[#e4e8ee]">
          {/* Header Info Card */}
          <div className="p-3.5 rounded-xl bg-[#182232]/80 border border-[#263042] flex items-center justify-between">
            <div>
              <span className="font-bold text-[#f59e0b] block text-sm">Create Server Official Updates</span>
              <span className="text-[11px] text-[#8fa0bc]">Minecraft 1.20.1 (Forge 47.4.20)</span>
            </div>
            <span className="px-2.5 py-1 rounded bg-[#12c3ca]/15 border border-[#12c3ca]/40 text-[#5ce1e6] font-mono font-bold text-xs">
              v1.2.6
            </span>
          </div>

          {/* v1.2.6 */}
          <div className="space-y-3 pb-5 border-b border-[#1c2432]">
            <div className="flex items-center justify-between">
              <h3 className="font-bold text-sm text-[#12c3ca] flex items-center gap-2">
                <span>[v1.2.6] Daytime Voting & Phantom Removal</span>
              </h3>
              <span className="text-[10px] text-[#8fa0bc] flex items-center gap-1">
                <Calendar className="w-3 h-3" /> October 12, 2026
              </span>
            </div>
            <div className="space-y-2 text-[#cad2df] pl-2">
              <div className="flex items-start gap-2">
                <Sparkles className="w-3.5 h-3.5 text-[#f59e0b] shrink-0 mt-0.5" />
                <span><b>Democratic Time & Weather Skip:</b> Added Server Vote mod. Players can initiate day or weather votes anytime using <code>/vote day</code>, <code>/voteday</code>, or <code>/vote weather</code>.</span>
              </div>
              <div className="flex items-start gap-2">
                <CheckCircle className="w-3.5 h-3.5 text-[#12c3ca] shrink-0 mt-0.5" />
                <span><b>Interactive Chat Voting:</b> Clickable [YES] and [NO] buttons directly in chat with sound notifications; passes when the majority votes yes.</span>
              </div>
              <div className="flex items-start gap-2">
                <Shield className="w-3.5 h-3.5 text-[#5ce1e6] shrink-0 mt-0.5" />
                <span><b>Phantom Removal:</b> Insomnia disabled server-wide. Alternative shapeless recipes added for Phantom Membrane (Feather + Slimeball + Leather/Bone).</span>
              </div>
            </div>
          </div>

          {/* v1.2.5 */}
          <div className="space-y-3 pb-5 border-b border-[#1c2432]">
            <div className="flex items-center justify-between">
              <h3 className="font-bold text-sm text-[#12c3ca] flex items-center gap-2">
                <span>[v1.2.5] Controls & Inventory Tweaks</span>
              </h3>
              <span className="text-[10px] text-[#8fa0bc] flex items-center gap-1">
                <Calendar className="w-3 h-3" /> October 9, 2026
              </span>
            </div>
            <div className="space-y-2 text-[#cad2df] pl-2">
              <div className="flex items-start gap-2">
                <Zap className="w-3.5 h-3.5 text-[#f59e0b] shrink-0 mt-0.5" />
                <span><b>WorldEdit Navigation Wand Deactivated:</b> Set navigation trace distance to -1; right-clicking while holding a compass no longer triggers unwanted teleportation.</span>
              </div>
              <div className="flex items-start gap-2">
                <CheckCircle className="w-3.5 h-3.5 text-[#12c3ca] shrink-0 mt-0.5" />
                <span><b>Mouse Tweaks RMB Placement:</b> Restored normal right-click single item placement into crafting tables and inventory slots.</span>
              </div>
            </div>
          </div>

          {/* v1.2.4 */}
          <div className="space-y-3 pb-5 border-b border-[#1c2432]">
            <div className="flex items-center justify-between">
              <h3 className="font-bold text-sm text-[#12c3ca] flex items-center gap-2">
                <span>[v1.2.4] Factions Territory & Map Integration Bridge</span>
              </h3>
              <span className="text-[10px] text-[#8fa0bc] flex items-center gap-1">
                <Calendar className="w-3 h-3" /> October 9, 2026
              </span>
            </div>
            <div className="space-y-2 text-[#cad2df] pl-2">
              <div className="flex items-start gap-2">
                <CheckCircle className="w-3.5 h-3.5 text-[#12c3ca] shrink-0 mt-0.5" />
                <span><b>Live Territory Overlays:</b> Custom Factions Xaero Bridge mod renders territory fills and borders seamlessly on Xaero's World Map and Minimap.</span>
              </div>
              <div className="flex items-start gap-2">
                <CheckCircle className="w-3.5 h-3.5 text-[#12c3ca] shrink-0 mt-0.5" />
                <span><b>Real-Time Hover Tooltips:</b> Hovering claimed chunks displays the owning faction name with instant packet synchronization.</span>
              </div>
            </div>
          </div>

          {/* v1.2.0 - Performance */}
          <div className="space-y-3 pb-3">
            <div className="flex items-center justify-between">
              <h3 className="font-bold text-sm text-[#12c3ca] flex items-center gap-2">
                <span>[v1.2.0] Engine & Performance Overhaul</span>
              </h3>
              <span className="text-[10px] text-[#8fa0bc] flex items-center gap-1">
                <Calendar className="w-3 h-3" /> October 9, 2026
              </span>
            </div>
            <div className="space-y-2 text-[#cad2df] pl-2">
              <div className="flex items-start gap-2">
                <Zap className="w-3.5 h-3.5 text-[#f59e0b] shrink-0 mt-0.5" />
                <span><b>ModernFix & ServerCore:</b> Asynchronous chunk I/O pipelines, entity tick throttles, and Forge memory leak patches enabled.</span>
              </div>
              <div className="flex items-start gap-2">
                <Shield className="w-3.5 h-3.5 text-[#5ce1e6] shrink-0 mt-0.5" />
                <span><b>Distant Horizons Optimization:</b> Rebalanced packet generation to guarantee stable 20.0 TPS and silky smooth LOD transitions.</span>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 bg-[#182232] border-t border-[#263042] flex justify-end">
          <button
            onClick={onClose}
            className="px-5 py-2 bg-[#0e121a] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 rounded-lg text-[#e4e8ee] font-bold text-xs transition-colors cursor-pointer"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
