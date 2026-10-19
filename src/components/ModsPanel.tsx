import React, { useEffect, useState } from "react";
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
} from "lucide-react";
import {
  InstalledModInfo,
  ModpackProfile,
  ModrinthSearchResult,
} from "../types";

interface ModsPanelProps {
  profile: ModpackProfile;
  onOpenFolder: (path: string) => void;
  onSyncProfile: () => Promise<void>;
  isSyncing: boolean;
  onNotify: (msg: string) => void;
  onError: (msg: string) => void;
}

export const ModsPanel: React.FC<ModsPanelProps> = ({
  profile,
  onOpenFolder,
  onSyncProfile,
  isSyncing,
  onNotify,
  onError,
}) => {
  const [subTab, setSubTab] = useState<"installed" | "modrinth">("installed");
  const [installedMods, setInstalledMods] = useState<InstalledModInfo[]>([]);
  const [isLoadingMods, setIsLoadingMods] = useState(false);
  const [filterText, setFilterText] = useState("");

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ModrinthSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [installingSlug, setInstallingSlug] = useState<string | null>(null);

  useEffect(() => {
    loadInstalledMods();
    if (subTab === "modrinth") {
      handleSearchModrinth(searchQuery);
    }
  }, [profile.id]);

  const loadInstalledMods = async () => {
    setIsLoadingMods(true);
    try {
      const list = await invoke<InstalledModInfo[]>("list_installed_mods", {
        profileId: profile.id,
      });
      setInstalledMods(list);
    } catch (err: any) {
      onError(`Failed to load mods: ${err}`);
    } finally {
      setIsLoadingMods(false);
    }
  };

  const handleToggleMod = async (mod: InstalledModInfo) => {
    const nextEnabled = !mod.enabled;
    try {
      await invoke("toggle_mod", {
        profileId: profile.id,
        fileName: mod.file_name,
        enabled: nextEnabled,
      });
      await loadInstalledMods();
    } catch (err: any) {
      onError(`Failed to toggle mod: ${err}`);
    }
  };

  const handleRemoveMod = async (mod: InstalledModInfo) => {
    try {
      await invoke("remove_mod", {
        profileId: profile.id,
        fileName: mod.file_name,
      });
      onNotify(`Removed ${mod.title} from ${profile.name}.`);
      await loadInstalledMods();
    } catch (err: any) {
      onError(`Failed to remove mod: ${err}`);
    }
  };

  const handleSearchModrinth = async (q: string) => {
    setIsSearching(true);
    try {
      const results = await invoke<ModrinthSearchResult[]>(
        "search_modrinth_mods",
        {
          profileId: profile.id,
          query: q,
        }
      );
      setSearchResults(results);
    } catch (err: any) {
      onError(`Modrinth search failed: ${err}`);
    } finally {
      setIsSearching(false);
    }
  };

  const handleInstallMod = async (item: ModrinthSearchResult) => {
    setInstallingSlug(item.slug);
    try {
      await invoke("install_modrinth_mod", {
        profileId: profile.id,
        slug: item.slug,
      });
      onNotify(
        `Installed ${item.title} (+ required dependencies) into ${profile.name}!`
      );
      await loadInstalledMods();
      setSearchResults((prev) =>
        prev.map((r) =>
          r.slug === item.slug ? { ...r, installed: true } : r
        )
      );
    } catch (err: any) {
      onError(`Failed to install ${item.title}: ${err}`);
    } finally {
      setInstallingSlug(null);
    }
  };

  const filteredInstalled = installedMods.filter(
    (m) =>
      m.title.toLowerCase().includes(filterText.toLowerCase()) ||
      m.file_name.toLowerCase().includes(filterText.toLowerCase()) ||
      (m.slug && m.slug.toLowerCase().includes(filterText.toLowerCase()))
  );

  const enabledCount = installedMods.filter((m) => m.enabled).length;

  const formatSize = (bytes: number) => {
    if (bytes < 1024 * 1024) {
      return `${(bytes / 1024).toFixed(0)} KB`;
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="relative z-10 flex flex-col h-full min-h-0 bg-[#0d121c]/90 border border-[#243144] rounded-2xl shadow-2xl overflow-hidden backdrop-blur-md">
      {/* Top Toolbar */}
      <div className="px-5 py-3.5 bg-[#131b29] border-b border-[#243144] flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setSubTab("installed")}
            className={`px-3.5 py-1.5 rounded-xl text-xs font-extrabold flex items-center gap-2 transition-all cursor-pointer border ${
              subTab === "installed"
                ? "bg-[#12c3ca]/20 border-[#12c3ca] text-[#5ce1e6]"
                : "bg-[#0c1018] border-[#243144] text-[#8fa0bc] hover:text-white"
            }`}
          >
            <Package className="w-3.5 h-3.5" />
            <span>
              Installed Mods ({enabledCount}/{installedMods.length})
            </span>
          </button>

          {profile.loader !== "vanilla" && (
            <button
              onClick={() => {
                setSubTab("modrinth");
                if (searchResults.length === 0) {
                  handleSearchModrinth(searchQuery);
                }
              }}
              className={`px-3.5 py-1.5 rounded-xl text-xs font-extrabold flex items-center gap-2 transition-all cursor-pointer border ${
                subTab === "modrinth"
                  ? "bg-emerald-500/20 border-emerald-400 text-emerald-300"
                  : "bg-[#0c1018] border-[#243144] text-[#8fa0bc] hover:text-white"
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>Browse Modrinth</span>
            </button>
          )}
        </div>

        {/* Search Input + Quick Actions */}
        <div className="flex items-center gap-2.5 flex-1 justify-end max-w-xl">
          {subTab === "installed" ? (
            <div className="relative flex-1 max-w-xs">
              <Search className="w-3.5 h-3.5 text-[#8fa0bc] absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                type="text"
                value={filterText}
                onChange={(e) => setFilterText(e.target.value)}
                placeholder="Filter installed mods..."
                className="w-full pl-8 pr-3 py-1.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none"
              />
            </div>
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                handleSearchModrinth(searchQuery);
              }}
              className="relative flex-1 max-w-sm flex items-center gap-1.5"
            >
              <div className="relative flex-1">
                <Search className="w-3.5 h-3.5 text-[#8fa0bc] absolute left-3 top-1/2 -translate-y-1/2" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={`Search Modrinth (${profile.loader.toUpperCase()})...`}
                  className="w-full pl-8 pr-3 py-1.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-emerald-400 text-xs text-[#e4e8ee] outline-none"
                />
              </div>
              <button
                type="submit"
                disabled={isSearching}
                className="px-3 py-1.5 rounded-xl bg-emerald-500/20 hover:bg-emerald-500/30 border border-emerald-400/50 text-emerald-300 text-xs font-bold cursor-pointer"
              >
                {isSearching ? "..." : "Search"}
              </button>
            </form>
          )}

          <button
            onClick={async () => {
              await onSyncProfile();
              await loadInstalledMods();
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

          <button
            onClick={() => onOpenFolder(`${profile.instance_dir}/mods`)}
            title="Open Instance Mods Folder"
            className="px-3 py-1.5 rounded-xl bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#e4e8ee] hover:text-[#f59e0b] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <FolderOpen className="w-3.5 h-3.5 text-[#f59e0b]" />
            <span>Mods Folder</span>
          </button>
        </div>
      </div>

      {/* Body List */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {subTab === "installed" ? (
          isLoadingMods ? (
            <div className="h-full flex items-center justify-center text-xs text-[#8fa0bc]">
              Loading installed mods...
            </div>
          ) : filteredInstalled.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-center p-8 space-y-3">
              <Sparkles className="w-8 h-8 text-[#12c3ca]/60" />
              <div className="space-y-1">
                <p className="text-sm font-bold text-[#e4e8ee]">
                  {installedMods.length === 0
                    ? "No mod archives downloaded in this instance yet"
                    : "No mods match your search filter"}
                </p>
                <p className="text-xs text-[#8fa0bc] max-w-md">
                  {profile.modrinth_slugs.length > 0 && installedMods.length === 0
                    ? `This profile has ${profile.modrinth_slugs.length} configured Modrinth mods (${profile.modrinth_slugs.slice(0, 5).join(", ")}, etc.). Click "Sync Mods" or "Play Game" to automatically resolve and download them!`
                    : "Click 'Browse Modrinth' above to discover and install mods directly into this profile."}
                </p>
              </div>
              {profile.modrinth_slugs.length > 0 && installedMods.length === 0 && (
                <button
                  onClick={async () => {
                    await onSyncProfile();
                    await loadInstalledMods();
                  }}
                  disabled={isSyncing}
                  className="px-4 py-2 rounded-xl bg-gradient-to-r from-[#0d949b] to-[#12c3ca] text-slate-950 font-extrabold text-xs flex items-center gap-2 cursor-pointer shadow-lg"
                >
                  <Download className="w-4 h-4" />
                  <span>
                    {isSyncing
                      ? "Downloading Mods..."
                      : `Download ${profile.modrinth_slugs.length} Profile Mods Now`}
                  </span>
                </button>
              )}
            </div>
          ) : (
            filteredInstalled.map((mod) => (
              <div
                key={mod.file_name}
                className={`p-3 rounded-xl border flex items-center justify-between gap-4 transition-all ${
                  mod.enabled
                    ? "bg-[#111826]/90 border-[#243144] hover:border-[#324560]"
                    : "bg-[#0b0f17]/60 border-[#1a2332] opacity-60"
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  {mod.icon_url ? (
                    <img
                      src={mod.icon_url}
                      alt={mod.title}
                      className="w-9 h-9 rounded-lg object-cover bg-[#182232] border border-[#2a384c] shrink-0"
                    />
                  ) : (
                    <div className="w-9 h-9 rounded-lg bg-[#182232] border border-[#2a384c] flex items-center justify-center text-[#12c3ca] shrink-0">
                      <Package className="w-4 h-4" />
                    </div>
                  )}

                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-extrabold text-[#e4e8ee] truncate">
                        {mod.title}
                      </span>
                      <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#182232] text-[#5ce1e6] border border-[#263042]">
                        {mod.version_number}
                      </span>
                      <span className="text-[10px] font-mono text-[#8fa0bc]">
                        {formatSize(mod.size_bytes)}
                      </span>
                      {!mod.enabled && (
                        <span className="text-[10px] font-bold px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 border border-amber-500/30">
                          Disabled
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-[#8fa0bc] truncate mt-0.5">
                      {mod.description}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  <button
                    onClick={() => handleToggleMod(mod)}
                    title={mod.enabled ? "Disable Mod" : "Enable Mod"}
                    className={`px-3 py-1.5 rounded-lg text-xs font-bold flex items-center gap-1.5 border transition-colors cursor-pointer ${
                      mod.enabled
                        ? "bg-emerald-500/15 border-emerald-500/40 text-emerald-300 hover:bg-emerald-500/25"
                        : "bg-[#182232] border-[#2a384c] text-[#8fa0bc] hover:text-white"
                    }`}
                  >
                    <Power className="w-3.5 h-3.5" />
                    <span>{mod.enabled ? "Enabled" : "Disabled"}</span>
                  </button>

                  {profile.id !== "create-server" && (
                    <button
                      onClick={() => handleRemoveMod(mod)}
                      title="Remove Mod"
                      className="p-1.5 rounded-lg bg-[#141c2b] hover:bg-rose-950/80 text-[#8fa0bc] hover:text-rose-400 border border-[#263042] hover:border-rose-500/40 transition-colors cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              </div>
            ))
          )
        ) : isSearching ? (
          <div className="h-full flex items-center justify-center text-xs text-[#8fa0bc]">
            Searching Modrinth for compatible {profile.loader.toUpperCase()} mods...
          </div>
        ) : (
          searchResults.map((hit) => (
            <div
              key={hit.project_id}
              className="p-3 rounded-xl bg-[#111826]/90 border border-[#243144] hover:border-[#324560] flex items-center justify-between gap-4 transition-all"
            >
              <div className="flex items-center gap-3 min-w-0">
                {hit.icon_url ? (
                  <img
                    src={hit.icon_url}
                    alt={hit.title}
                    className="w-10 h-10 rounded-lg object-cover bg-[#182232] border border-[#2a384c] shrink-0"
                  />
                ) : (
                  <div className="w-10 h-10 rounded-lg bg-[#182232] border border-[#2a384c] flex items-center justify-center text-emerald-400 shrink-0">
                    <Package className="w-5 h-5" />
                  </div>
                )}

                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-extrabold text-[#e4e8ee] truncate">
                      {hit.title}
                    </span>
                    <span className="text-[10px] text-[#8fa0bc]">
                      by {hit.author}
                    </span>
                    <span className="text-[10px] font-mono text-[#5ce1e6]">
                      • {(hit.downloads / 1000).toFixed(0)}k downloads
                    </span>
                  </div>
                  <p className="text-[11px] text-[#8fa0bc] truncate mt-0.5">
                    {hit.description}
                  </p>
                </div>
              </div>

              <div className="shrink-0">
                {hit.installed ? (
                  <span className="px-3 py-1.5 rounded-xl bg-emerald-500/15 border border-emerald-500/40 text-emerald-300 text-xs font-bold flex items-center gap-1.5">
                    <Check className="w-3.5 h-3.5" />
                    <span>Added</span>
                  </span>
                ) : (
                  <button
                    onClick={() => handleInstallMod(hit)}
                    disabled={installingSlug === hit.slug}
                    className="px-3.5 py-1.5 rounded-xl bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 font-extrabold text-xs flex items-center gap-1.5 transition-all cursor-pointer shadow-md disabled:opacity-50"
                  >
                    <Download className="w-3.5 h-3.5" />
                    <span>
                      {installingSlug === hit.slug ? "Installing..." : "Install"}
                    </span>
                  </button>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
