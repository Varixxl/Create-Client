import React from "react";
import {
  Flame,
  Zap,
  Box,
  Plus,
  Copy,
  Sliders,
  FolderOpen,
  Trash2,
  ArrowDownCircle,
} from "lucide-react";
import { ModpackProfile, ProgressPayload } from "../types";

interface ProfileSidebarProps {
  profiles: ModpackProfile[];
  selectedProfileId: string;
  progress?: ProgressPayload | null;
  onSelectProfile: (id: string) => void;
  onCreateProfile: () => void;
  onEditProfile: (profile: ModpackProfile) => void;
  onDuplicateProfile: (profile: ModpackProfile) => void;
  onDeleteProfile: (profile: ModpackProfile) => void;
  onOpenFolder: (path: string) => void;
  disabled?: boolean;
}

export const ProfileSidebar: React.FC<ProfileSidebarProps> = ({
  profiles,
  selectedProfileId,
  progress,
  onSelectProfile,
  onCreateProfile,
  onEditProfile,
  onDuplicateProfile,
  onDeleteProfile,
  onOpenFolder,
  disabled = false,
}) => {
  const getLoaderBadge = (p: ModpackProfile) => {
    const ver =
      p.mc_version === "latest"
        ? p.resolved_mc_version
          ? `${p.resolved_mc_version} (Latest)`
          : "Latest"
        : p.mc_version;
    if (p.loader === "forge") {
      return {
        label: `Forge ${ver}`,
        cls: "bg-amber-500/15 text-amber-300 border-amber-500/35",
      };
    }
    if (p.loader === "fabric") {
      return {
        label: `Fabric ${ver}`,
        cls: "bg-[#12c3ca]/15 text-[#5ce1e6] border-[#12c3ca]/35",
      };
    }
    return {
      label: `Vanilla ${ver}`,
      cls: "bg-emerald-500/15 text-emerald-300 border-emerald-500/35",
    };
  };

  const getProfileIcon = (p: ModpackProfile, isSelected: boolean) => {
    if (p.id === "create-server" || p.loader === "forge") {
      return (
        <div
          className={`w-9 h-9 rounded-xl flex items-center justify-center shrink-0 border transition-all ${
            isSelected
              ? "bg-amber-500/20 border-amber-400/60 text-amber-300 shadow-[0_0_12px_rgba(245,158,11,0.25)]"
              : "bg-[#141c2b] border-[#263042] text-amber-400/80"
          }`}
        >
          <Flame className="w-4 h-4" />
        </div>
      );
    }
    if (p.loader === "fabric") {
      return (
        <div
          className={`w-9 h-9 rounded-xl flex items-center justify-center shrink-0 border transition-all ${
            isSelected
              ? "bg-[#12c3ca]/20 border-[#12c3ca]/60 text-[#5ce1e6] shadow-[0_0_12px_rgba(18,195,202,0.25)]"
              : "bg-[#141c2b] border-[#263042] text-[#12c3ca]/80"
          }`}
        >
          <Zap className="w-4 h-4" />
        </div>
      );
    }
    return (
      <div
        className={`w-9 h-9 rounded-xl flex items-center justify-center shrink-0 border transition-all ${
          isSelected
            ? "bg-emerald-500/20 border-emerald-400/60 text-emerald-300"
            : "bg-[#141c2b] border-[#263042] text-emerald-400/80"
        }`}
      >
        <Box className="w-4 h-4" />
      </div>
    );
  };

  return (
    <aside className="w-68 bg-[#0d121c]/95 border-r border-[#232f42] flex flex-col justify-between z-20 select-none shrink-0">
      {/* Top Section: Header + Profiles List */}
      <div className="flex flex-col flex-1 min-h-0">
        <div className="px-4 pt-4 pb-2.5 flex items-center justify-between border-b border-[#1c2636]">
          <span className="text-[11px] font-extrabold uppercase tracking-wider text-[#8fa0bc]">
            Modpack Profiles
          </span>
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-[#182232] text-[#5ce1e6] border border-[#263042]">
            {profiles.length}
          </span>
        </div>

        <div className="flex-1 overflow-y-auto p-2.5 space-y-2 custom-scrollbar">
          {profiles.map((p) => {
            const isSelected = p.id === selectedProfileId;
            const badge = getLoaderBadge(p);

            return (
              <div
                key={p.id}
                onClick={() => !disabled && onSelectProfile(p.id)}
                className={`group relative rounded-xl p-2.5 border transition-all duration-150 ${
                  disabled ? "cursor-not-allowed opacity-75" : "cursor-pointer"
                } ${
                  isSelected
                    ? "bg-gradient-to-r from-[#162235] to-[#121b2b] border-[#12c3ca]/60 shadow-lg shadow-cyan-950/30"
                    : "bg-[#111723]/70 hover:bg-[#162030] border-[#202b3d] hover:border-[#2e3d56]"
                }`}
              >
                {/* Active left accent bar */}
                {isSelected && (
                  <div className="absolute left-0 top-2.5 bottom-2.5 w-1 rounded-r-full bg-[#12c3ca] shadow-[0_0_8px_#12c3ca]" />
                )}

                <div className="flex items-start gap-2.5">
                  {getProfileIcon(p, isSelected)}

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between gap-1">
                      <h3
                        className={`text-xs font-extrabold truncate ${
                          isSelected ? "text-[#e4e8ee]" : "text-[#c6d0df]"
                        }`}
                      >
                        {p.name}
                      </h3>
                    </div>

                    <div className="mt-1 flex items-center gap-1.5 flex-wrap">
                      <span
                        className={`text-[9px] font-mono font-bold px-1.5 py-0.5 rounded border ${badge.cls}`}
                      >
                        {badge.label}
                      </span>
                      {p.is_builtin && (
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-[#182232] text-[#8fa0bc] border border-[#263042]">
                          Built-in
                        </span>
                      )}
                      {p.ram_gb && (
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 border border-purple-500/30">
                          {p.ram_gb}GB
                        </span>
                      )}
                      {p.java_version && (
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300 border border-blue-500/30">
                          Java {p.java_version}
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Quick Action Bar for Selected or Hovered Profile */}
                <div
                  className={`mt-2 pt-2 border-t border-[#222f43]/80 flex items-center justify-between gap-1 ${
                    isSelected ? "flex" : "hidden group-hover:flex"
                  }`}
                  onClick={(e) => e.stopPropagation()}
                >
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => onEditProfile(p)}
                      title="Edit Profile & Overrides"
                      className="p-1.5 rounded-lg bg-[#0e121a] hover:bg-[#1e293b] text-[#8fa0bc] hover:text-[#12c3ca] border border-[#263042] transition-colors cursor-pointer"
                    >
                      <Sliders className="w-3 h-3" />
                    </button>
                    <button
                      onClick={() => onDuplicateProfile(p)}
                      title="Duplicate Profile"
                      className="p-1.5 rounded-lg bg-[#0e121a] hover:bg-[#1e293b] text-[#8fa0bc] hover:text-[#5ce1e6] border border-[#263042] transition-colors cursor-pointer"
                    >
                      <Copy className="w-3 h-3" />
                    </button>
                    <button
                      onClick={() => onOpenFolder(p.instance_dir)}
                      title="Open Instance Folder"
                      className="p-1.5 rounded-lg bg-[#0e121a] hover:bg-[#1e293b] text-[#8fa0bc] hover:text-[#f59e0b] border border-[#263042] transition-colors cursor-pointer"
                    >
                      <FolderOpen className="w-3 h-3" />
                    </button>
                  </div>

                  {!p.is_builtin && (
                    <button
                      onClick={() => onDeleteProfile(p)}
                      title="Delete Profile"
                      className="p-1.5 rounded-lg bg-[#0e121a] hover:bg-rose-950/70 text-[#8fa0bc] hover:text-rose-400 border border-[#263042] hover:border-rose-500/40 transition-colors cursor-pointer"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Bottom Action & Download Progress Bar (Feature 4) */}
      <div className="p-3 border-t border-[#1c2636] bg-[#0a0e16] space-y-2.5">
        {progress && progress.percentage < 100 && (
          <div className="p-2.5 rounded-xl bg-[#111724] border border-[#233247] shadow-inner space-y-1.5 animate-in fade-in duration-200">
            <div className="flex items-center justify-between text-[11px]">
              <div className="flex items-center gap-1.5 truncate max-w-[170px] text-[#5ce1e6] font-bold">
                <ArrowDownCircle className="w-3.5 h-3.5 animate-bounce shrink-0" />
                <span className="truncate">{progress.task}</span>
              </div>
              <span className="font-mono text-[10px] text-[#8fa0bc]">
                {progress.percentage.toFixed(0)}%
              </span>
            </div>

            <div className="w-full h-1.5 rounded-full bg-[#1b2536] overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-[#0d949b] to-[#12c3ca] rounded-full transition-all duration-200"
                style={{ width: `${Math.min(100, Math.max(0, progress.percentage))}%` }}
              />
            </div>

            <div className="flex items-center justify-between text-[9px] font-mono text-[#8fa0bc]">
              <span>
                {progress.total_bytes > 0
                  ? `${(progress.current_bytes / (1024 * 1024)).toFixed(1)} / ${(progress.total_bytes / (1024 * 1024)).toFixed(1)} MB`
                  : `${(progress.current_bytes / (1024 * 1024)).toFixed(1)} MB`}
              </span>
              {progress.speed_bytes_sec > 0 && (
                <span className="text-[#5ce1e6]">
                  {(progress.speed_bytes_sec / (1024 * 1024)).toFixed(1)} MB/s
                </span>
              )}
            </div>
          </div>
        )}

        <button
          onClick={onCreateProfile}
          disabled={disabled}
          className="w-full py-2.5 px-3 rounded-xl bg-gradient-to-r from-[#12c3ca]/15 to-[#0d949b]/15 hover:from-[#12c3ca]/25 hover:to-[#0d949b]/25 border border-[#12c3ca]/40 hover:border-[#12c3ca] text-[#5ce1e6] font-bold text-xs flex items-center justify-center gap-2 transition-all cursor-pointer shadow-md disabled:opacity-50"
        >
          <Plus className="w-4 h-4" />
          <span>New Modpack Profile</span>
        </button>
      </div>
    </aside>
  );
};
