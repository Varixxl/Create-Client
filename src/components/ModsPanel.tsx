import React, { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Search,
  Package,
  Download,
  Trash2,
  FolderOpen,
  RefreshCw,
  Check,
  Sparkles,
  Power,
  Globe,
  Flame,
  Palette,
  Sun,
  Key,
  AlertTriangle,
  ShieldCheck,
} from "lucide-react";
import {
  InstalledModInfo,
  ModpackProfile,
  ContentSearchResult,
  ContentType,
  ContentSource,
} from "../types";

interface ModsPanelProps {
  profile: ModpackProfile;
  curseforgeApiKey?: string;
  onOpenSettings?: () => void;
  onOpenFolder: (path: string) => void;
  onSyncProfile: () => Promise<void>;
  isSyncing: boolean;
  onNotify: (msg: string) => void;
  onError: (msg: string) => void;
}

export const ModsPanel: React.FC<ModsPanelProps> = React.memo(({
  profile,
  curseforgeApiKey = "",
  onOpenSettings,
  onOpenFolder,
  onSyncProfile,
  isSyncing,
  onNotify,
  onError,
}) => {
  const [activeView, setActiveView] = useState<"installed" | "browse">("installed");
  const [contentType, setContentType] = useState<ContentType>("mod");
  const [contentSource, setContentSource] = useState<ContentSource>("modrinth");

  const [installedItems, setInstalledItems] = useState<InstalledModInfo[]>([]);
  const [isLoadingInstalled, setIsLoadingInstalled] = useState(false);
  const [filterText, setFilterText] = useState("");

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ContentSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [installingId, setInstallingId] = useState<string | null>(null);

  useEffect(() => {
    loadInstalledContent(contentType);
    if (activeView === "browse") {
      handleSearch(searchQuery, contentType, contentSource);
    }
  }, [profile.id, contentType]);

  const loadInstalledContent = async (type: ContentType) => {
    setIsLoadingInstalled(true);
    try {
      const list = await invoke<InstalledModInfo[]>("list_installed_content", {
        profileId: profile.id,
        contentType: type,
      });
      setInstalledItems(list || []);
    } catch (err: any) {
      onError(`Failed to load ${type}s: ${err}`);
    } finally {
      setIsLoadingInstalled(false);
    }
  };

  const handleToggleContent = async (item: InstalledModInfo) => {
    const nextEnabled = !item.enabled;
    try {
      await invoke("toggle_content", {
        profileId: profile.id,
        contentType,
        fileName: item.file_name,
        enabled: nextEnabled,
      });
      await loadInstalledContent(contentType);
    } catch (err: any) {
      onError(`Failed to toggle: ${err}`);
    }
  };

  const handleRemoveContent = async (item: InstalledModInfo) => {
    try {
      await invoke("remove_content", {
        profileId: profile.id,
        contentType,
        fileName: item.file_name,
      });
      onNotify(`Removed "${item.title}" from ${profile.name}.`);
      await loadInstalledContent(contentType);
    } catch (err: any) {
      onError(`Failed to remove: ${err}`);
    }
  };

  const handleSearch = async (
    q: string,
    cType: ContentType,
    src: ContentSource
  ) => {
    if (src === "curseforge" && !curseforgeApiKey.trim()) {
      setSearchResults([]);
      return;
    }

    setIsSearching(true);
    try {
      const targetMc = profile.resolved_mc_version || (profile.mc_version !== "latest" ? profile.mc_version : undefined);
      const results = await invoke<ContentSearchResult[]>("search_content", {
        query: q,
        contentType: cType,
        source: src,
        mcVersion: targetMc,
        loader: profile.loader,
        curseforgeKey: curseforgeApiKey.trim() || null,
        profileId: profile.id,
      });
      setSearchResults(results || []);
    } catch (err: any) {
      onError(`Search failed: ${err}`);
    } finally {
      setIsSearching(false);
    }
  };

  const handleInstallContent = async (item: ContentSearchResult) => {
    setInstallingId(item.project_id);
    try {
      await invoke<string>("install_content", {
        profileId: profile.id,
        projectId: item.project_id,
        versionId: null,
        contentType: item.content_type,
        source: item.source,
        curseforgeKey: curseforgeApiKey.trim() || null,
      });
      onNotify(`Installed ${item.title} into ${profile.name}!`);
      await loadInstalledContent(contentType);
      setSearchResults((prev) =>
        prev.map((r) =>
          r.project_id === item.project_id ? { ...r, installed: true } : r
        )
      );
    } catch (err: any) {
      onError(`Failed to install ${item.title}: ${err}`);
    } finally {
      setInstallingId(null);
    }
  };

  const filteredInstalled = useMemo(() => {
    if (!filterText.trim()) return installedItems;
    const lower = filterText.toLowerCase();
    return installedItems.filter(
      (m) =>
        m.title.toLowerCase().includes(lower) ||
        m.file_name.toLowerCase().includes(lower) ||
        (m.slug && m.slug.toLowerCase().includes(lower))
    );
  }, [installedItems, filterText]);

  const enabledCount = useMemo(
    () => installedItems.filter((m) => m.enabled).length,
    [installedItems]
  );

  const getFolderSubdir = () => {
    switch (contentType) {
      case "resourcepack":
        return "resourcepacks";
      case "shader":
        return "shaderpacks";
      default:
        return "mods";
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024 * 1024) {
      return `${(bytes / 1024).toFixed(0)} KB`;
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const hasCurseForgeKey = !!curseforgeApiKey.trim();

  return (
    <div className="relative z-10 flex flex-col h-full min-h-0 bg-[#0d121c] border border-[#243144] rounded-2xl shadow-2xl overflow-hidden">
      {/* Top Header: Content Types & View Switcher */}
      <div className="px-5 py-3 bg-[#131b29] border-b border-[#243144] flex items-center justify-between gap-4 flex-wrap">
        {/* Left: Content Type Pills (Mods | Resource Packs | Shaders) */}
        <div className="flex items-center gap-1.5 p-1 rounded-xl bg-[#0a0d14] border border-[#222d3e]">
          <button
            onClick={() => setContentType("mod")}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold flex items-center gap-1.5 transition-all cursor-pointer ${
              contentType === "mod"
                ? "bg-[#12c3ca]/20 border border-[#12c3ca]/60 text-[#5ce1e6]"
                : "text-[#8fa0bc] hover:text-white"
            }`}
          >
            <Package className="w-3.5 h-3.5" />
            <span>Mods</span>
          </button>

          <button
            onClick={() => setContentType("resourcepack")}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold flex items-center gap-1.5 transition-all cursor-pointer ${
              contentType === "resourcepack"
                ? "bg-[#12c3ca]/20 border border-[#12c3ca]/60 text-[#5ce1e6]"
                : "text-[#8fa0bc] hover:text-white"
            }`}
          >
            <Palette className="w-3.5 h-3.5" />
            <span>Resource Packs</span>
          </button>

          <button
            onClick={() => setContentType("shader")}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold flex items-center gap-1.5 transition-all cursor-pointer ${
              contentType === "shader"
                ? "bg-[#12c3ca]/20 border border-[#12c3ca]/60 text-[#5ce1e6]"
                : "text-[#8fa0bc] hover:text-white"
            }`}
          >
            <Sun className="w-3.5 h-3.5" />
            <span>Shaders</span>
          </button>
        </div>

        {/* Center: Installed vs Browse Toggle */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 p-1 rounded-xl bg-[#0a0d14] border border-[#222d3e]">
            <button
              onClick={() => setActiveView("installed")}
              className={`px-3 py-1.5 rounded-lg text-xs font-extrabold flex items-center gap-1.5 transition-all cursor-pointer ${
                activeView === "installed"
                  ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] text-slate-950 shadow-md"
                  : "text-[#8fa0bc] hover:text-white"
              }`}
            >
              <span>Installed ({enabledCount}/{installedItems.length})</span>
            </button>

            <button
              onClick={() => {
                setActiveView("browse");
                if (searchResults.length === 0) {
                  handleSearch(searchQuery, contentType, contentSource);
                }
              }}
              className={`px-3 py-1.5 rounded-lg text-xs font-extrabold flex items-center gap-1.5 transition-all cursor-pointer ${
                activeView === "browse"
                  ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] text-slate-950 shadow-md"
                  : "text-[#8fa0bc] hover:text-white"
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>Browse Online</span>
            </button>
          </div>
        </div>

        {/* Right: Quick Folder & Sync Buttons */}
        <div className="flex items-center gap-2">
          {contentType === "mod" && (
            <button
              onClick={async () => {
                await onSyncProfile();
                await loadInstalledContent("mod");
              }}
              disabled={isSyncing}
              title="Sync & Download Profile Mods"
              className="px-3 py-1.5 rounded-xl bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#5ce1e6] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
            >
              <RefreshCw
                className={`w-3.5 h-3.5 ${isSyncing ? "animate-spin" : ""}`}
              />
              <span>{isSyncing ? "Syncing..." : "Sync Mods"}</span>
            </button>
          )}

          <button
            onClick={() => onOpenFolder(`${profile.instance_dir}/${getFolderSubdir()}`)}
            title={`Open Instance ${getFolderSubdir()} Folder`}
            className="px-3 py-1.5 rounded-xl bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#e4e8ee] hover:text-[#f59e0b] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <FolderOpen className="w-3.5 h-3.5 text-[#f59e0b]" />
            <span className="capitalize">{getFolderSubdir()} Folder</span>
          </button>
        </div>
      </div>

      {/* Sub-toolbar: Search & Source Selection */}
      <div className="px-5 py-2.5 bg-[#0e1420] border-b border-[#1f2b3c] flex items-center justify-between gap-4">
        {activeView === "browse" ? (
          <div className="flex items-center gap-3 flex-1 flex-wrap">
            {/* Source Toggle: Modrinth vs CurseForge */}
            <div className="flex items-center gap-1 p-0.5 rounded-lg bg-[#090d14] border border-[#243144]">
              <button
                type="button"
                onClick={() => {
                  setContentSource("modrinth");
                  handleSearch(searchQuery, contentType, "modrinth");
                }}
                className={`px-3 py-1 rounded-md text-xs font-bold flex items-center gap-1.5 transition-all cursor-pointer ${
                  contentSource === "modrinth"
                    ? "bg-emerald-500/20 text-emerald-300 border border-emerald-400/50"
                    : "text-[#8fa0bc] hover:text-white"
                }`}
              >
                <Globe className="w-3 h-3" />
                <span>Modrinth</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  setContentSource("curseforge");
                  handleSearch(searchQuery, contentType, "curseforge");
                }}
                className={`px-3 py-1 rounded-md text-xs font-bold flex items-center gap-1.5 transition-all cursor-pointer ${
                  contentSource === "curseforge"
                    ? "bg-amber-500/20 text-amber-300 border border-amber-400/50"
                    : "text-[#8fa0bc] hover:text-white"
                }`}
              >
                <Flame className="w-3 h-3 text-amber-400" />
                <span>CurseForge</span>
              </button>
            </div>

            {/* Search Input */}
            <form
              onSubmit={(e) => {
                e.preventDefault();
                handleSearch(searchQuery, contentType, contentSource);
              }}
              className="flex-1 max-w-md relative flex items-center gap-1.5"
            >
              <div className="relative flex-1">
                <Search className="w-3.5 h-3.5 text-[#8fa0bc] absolute left-3 top-1/2 -translate-y-1/2" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={`Search ${contentSource === "modrinth" ? "Modrinth" : "CurseForge"} for ${contentType}s...`}
                  className="w-full pl-8 pr-3 py-1.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none"
                />
              </div>
              <button
                type="submit"
                disabled={isSearching}
                className="px-3.5 py-1.5 rounded-xl bg-[#12c3ca]/20 hover:bg-[#12c3ca]/30 border border-[#12c3ca]/50 text-[#5ce1e6] text-xs font-bold cursor-pointer"
              >
                {isSearching ? "Searching..." : "Search"}
              </button>
            </form>
          </div>
        ) : (
          <div className="relative flex-1 max-w-sm">
            <Search className="w-3.5 h-3.5 text-[#8fa0bc] absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
              placeholder={`Filter installed ${contentType}s...`}
              className="w-full pl-8 pr-3 py-1.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none"
            />
          </div>
        )}

        {/* Right side: Active Profile Scope Badge */}
        <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-[#090d14] border border-[#232f42] text-[10px] font-mono shrink-0">
          <ShieldCheck className="w-3.5 h-3.5 text-[#12c3ca]" />
          <span className="text-[#8fa0bc] hidden sm:inline">Scope:</span>
          <span className="font-bold text-[#e4e8ee]">{profile.name}</span>
          <span className="text-[#3a475d]">|</span>
          <span className="font-bold text-[#f59e0b] uppercase">{profile.loader}</span>
          <span className="text-[#5ce1e6]">{profile.resolved_mc_version || profile.mc_version}</span>
        </div>
      </div>

      {/* Vanilla Mod Incompatibility Warning Banner */}
      {activeView === "browse" && contentType === "mod" && profile.loader.toLowerCase() === "vanilla" && (
        <div className="p-3 bg-amber-500/10 border-b border-amber-500/30 flex items-center gap-2.5 text-xs text-amber-300">
          <AlertTriangle className="w-4 h-4 shrink-0 text-amber-400" />
          <span>
            Vanilla Minecraft profiles do not support mods. Create or switch to a Forge, Fabric, Quilt, or NeoForge profile to install mods, or select Resource Packs or Shaders.
          </span>
        </div>
      )}

      {/* CurseForge API Key Warning Banner */}
      {activeView === "browse" && contentSource === "curseforge" && !hasCurseForgeKey && (
        <div className="p-3 bg-amber-500/10 border-b border-amber-500/30 flex items-center justify-between gap-4">
          <div className="flex items-center gap-2.5 text-xs text-amber-300">
            <Key className="w-4 h-4 shrink-0" />
            <span>
              CurseForge requires a free Eternal API key to search & download. Configure your key in Settings.
            </span>
          </div>
          {onOpenSettings && (
            <button
              onClick={onOpenSettings}
              className="px-3 py-1 rounded-lg bg-amber-500 hover:bg-amber-400 text-slate-950 font-bold text-xs cursor-pointer shadow-md"
            >
              Configure in Settings
            </button>
          )}
        </div>
      )}

      {/* Main Body List */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2.5 custom-scrollbar">
        {activeView === "installed" ? (
          isLoadingInstalled ? (
            <div className="h-full flex items-center justify-center text-xs text-[#8fa0bc]">
              Loading installed {contentType}s...
            </div>
          ) : filteredInstalled.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-center p-8 space-y-3">
              <Sparkles className="w-8 h-8 text-[#12c3ca]/60" />
              <div className="space-y-1">
                <p className="text-sm font-bold text-[#e4e8ee]">
                  {installedItems.length === 0
                    ? `No ${contentType}s installed in this instance yet`
                    : `No ${contentType}s match your filter`}
                </p>
                <p className="text-xs text-[#8fa0bc] max-w-md">
                  Switch to &quot;Browse Online&quot; above to search and install from Modrinth or CurseForge directly.
                </p>
              </div>
            </div>
          ) : (
            filteredInstalled.map((item) => (
              <div
                key={item.file_name}
                className={`p-3 rounded-xl border transition-all flex items-center justify-between gap-3 ${
                  item.enabled
                    ? "bg-[#111724]/80 border-[#233247] hover:border-[#324560]"
                    : "bg-[#0b0f17]/60 border-[#1c2636] opacity-60"
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className="w-10 h-10 rounded-xl bg-[#162030] border border-[#24344d] flex items-center justify-center shrink-0 overflow-hidden">
                    {item.icon_url ? (
                      <img
                        src={item.icon_url}
                        alt={item.title}
                        className="w-full h-full object-cover"
                        onError={(e) => {
                          e.currentTarget.style.display = "none";
                        }}
                      />
                    ) : contentType === "resourcepack" ? (
                      <Palette className="w-5 h-5 text-emerald-400" />
                    ) : contentType === "shader" ? (
                      <Sun className="w-5 h-5 text-amber-400" />
                    ) : (
                      <Package className="w-5 h-5 text-[#12c3ca]" />
                    )}
                  </div>

                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <h4 className="text-xs font-bold text-[#e4e8ee] truncate">
                        {item.title}
                      </h4>
                      <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-[#182232] text-[#8fa0bc] border border-[#243144]">
                        {formatSize(item.size_bytes)}
                      </span>
                      {!item.enabled && (
                        <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-rose-500/20 text-rose-300 border border-rose-500/40">
                          Disabled
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-[#8fa0bc] font-mono truncate">
                      {item.file_name}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleToggleContent(item)}
                    title={item.enabled ? "Disable" : "Enable"}
                    className={`p-1.5 rounded-lg border text-xs transition-colors cursor-pointer ${
                      item.enabled
                        ? "bg-[#12c3ca]/15 border-[#12c3ca]/40 text-[#5ce1e6] hover:bg-[#12c3ca]/25"
                        : "bg-[#162030] border-[#24344d] text-[#8fa0bc] hover:text-white"
                    }`}
                  >
                    <Power className="w-3.5 h-3.5" />
                  </button>

                  <button
                    onClick={() => handleRemoveContent(item)}
                    title="Remove"
                    className="p-1.5 rounded-lg bg-[#0e121a] hover:bg-rose-950/70 border border-[#263042] text-[#8fa0bc] hover:text-rose-400 transition-colors cursor-pointer"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            ))
          )
        ) : (
          /* Online Browse View */
          isSearching ? (
            <div className="h-full flex items-center justify-center text-xs text-[#8fa0bc]">
              Searching {contentSource === "modrinth" ? "Modrinth" : "CurseForge"}...
            </div>
          ) : searchResults.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-center p-8 space-y-2">
              <Search className="w-8 h-8 text-[#475569]" />
              <p className="text-xs text-[#8fa0bc]">
                {contentSource === "curseforge" && !hasCurseForgeKey
                  ? "Enter your CurseForge API key in Settings to search CurseForge."
                  : `No results found for "${searchQuery}".`}
              </p>
            </div>
          ) : (
            searchResults.map((item) => {
              const isInstalling = installingId === item.project_id;

              return (
                <div
                  key={`${item.source}-${item.project_id}`}
                  className="p-3.5 rounded-xl border border-[#233247] bg-[#111724]/80 hover:border-[#334661] transition-all flex items-start justify-between gap-4"
                >
                  <div className="flex items-start gap-3 min-w-0">
                    <div className="w-12 h-12 rounded-xl bg-[#162030] border border-[#24344d] flex items-center justify-center shrink-0 overflow-hidden">
                      {item.icon_url ? (
                        <img
                          src={item.icon_url}
                          alt={item.title}
                          className="w-full h-full object-cover"
                          onError={(e) => {
                            e.currentTarget.style.display = "none";
                          }}
                        />
                      ) : (
                        <Package className="w-6 h-6 text-[#12c3ca]" />
                      )}
                    </div>

                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <h4 className="text-xs font-bold text-[#e4e8ee] truncate">
                          {item.title}
                        </h4>
                        <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-[#182232] text-[#8fa0bc] border border-[#243144]">
                          by {item.author}
                        </span>
                        <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-[#182232] text-[#5ce1e6] border border-[#243144]">
                          {item.downloads.toLocaleString()} DLs
                        </span>
                        <span
                          className={`text-[9px] font-mono uppercase px-1.5 py-0.2 rounded font-bold border ${
                            item.source === "modrinth"
                              ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                              : "bg-amber-500/15 text-amber-300 border-amber-500/30"
                          }`}
                        >
                          {item.source}
                        </span>
                      </div>
                      <p className="text-[11px] text-[#8fa0bc] line-clamp-2 mt-1">
                        {item.description || "No description provided."}
                      </p>
                    </div>
                  </div>

                  <div className="shrink-0 flex items-center gap-2">
                    {item.installed ? (
                      <div className="px-3 py-1.5 rounded-xl bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 text-xs font-bold flex items-center gap-1.5">
                        <Check className="w-3.5 h-3.5" />
                        <span>Installed</span>
                      </div>
                    ) : contentType === "mod" && profile.loader.toLowerCase() === "vanilla" ? (
                      <div
                        title="Vanilla profiles do not support mod loading"
                        className="px-3 py-1.5 rounded-xl bg-gray-800/60 border border-gray-700/50 text-gray-400 text-xs font-semibold cursor-not-allowed opacity-60"
                      >
                        Incompatible (Vanilla)
                      </div>
                    ) : (
                      <button
                        onClick={() => handleInstallContent(item)}
                        disabled={isInstalling}
                        className="px-3.5 py-1.5 rounded-xl bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 font-extrabold text-xs flex items-center gap-1.5 transition-all cursor-pointer shadow-md disabled:opacity-50"
                      >
                        <Download className="w-3.5 h-3.5" />
                        <span>{isInstalling ? "Installing..." : "Install"}</span>
                      </button>
                    )}
                  </div>
                </div>
              );
            })
          )
        )}
      </div>
    </div>
  );
});
