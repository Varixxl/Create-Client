import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Server,
  FolderOpen,
  RefreshCw,
  Trash2,
  AlertTriangle,
  Plus,
} from "lucide-react";
import {
  ModpackProfile,
  ServerInstance,
  ServerStatus,
  ServerDivergenceInfo,
} from "../types";

interface ServerStatusPanelProps {
  profile: ModpackProfile;
  servers: ServerInstance[];
  onRefreshServers: () => Promise<void>;
  onCreateServer: () => void;
  onOpenFolder: (path: string) => void;
  onNotify: (msg: string) => void;
  onError: (msg: string) => void;
}

export const ServerStatusPanel: React.FC<ServerStatusPanelProps> = ({
  profile,
  servers,
  onRefreshServers,
  onCreateServer,
  onOpenFolder,
  onNotify,
  onError,
}) => {
  const [serverStatuses, setServerStatuses] = useState<Record<string, ServerStatus>>({});
  const [divergences, setDivergences] = useState<Record<string, ServerDivergenceInfo>>({});
  const [isPinging, setIsPinging] = useState<Record<string, boolean>>({});
  const [isSyncing, setIsSyncing] = useState<Record<string, boolean>>({});
  const [serverToDelete, setServerToDelete] = useState<ServerInstance | null>(null);

  const profileServers = servers.filter((s) => s.profile_id === profile.id);

  useEffect(() => {
    profileServers.forEach((s) => {
      pingServer(s.id);
      checkDivergence(s.id);
    });
  }, [profile.id, servers.length]);

  const pingServer = async (serverId: string) => {
    setIsPinging((prev) => ({ ...prev, [serverId]: true }));
    try {
      const status = await invoke<ServerStatus>("ping_server_instance", { serverId });
      setServerStatuses((prev) => ({ ...prev, [serverId]: status }));
    } catch {
      // Set offline fallback
      setServerStatuses((prev) => ({
        ...prev,
        [serverId]: {
          server_name: "Local Server",
          online: false,
          ping_ms: 0,
          online_players: 0,
          max_players: 0,
          motd: "Offline / Not running",
          version: "Unknown",
        },
      }));
    } finally {
      setIsPinging((prev) => ({ ...prev, [serverId]: false }));
    }
  };

  const checkDivergence = async (serverId: string) => {
    try {
      const div = await invoke<ServerDivergenceInfo>("check_server_profile_divergence", {
        serverId,
      });
      setDivergences((prev) => ({ ...prev, [serverId]: div }));
    } catch (err) {
      console.error("Divergence check failed:", err);
    }
  };

  const handleSyncServer = async (serverId: string) => {
    setIsSyncing((prev) => ({ ...prev, [serverId]: true }));
    try {
      const msg = await invoke<string>("sync_server_with_profile", { serverId });
      onNotify(msg);
      await checkDivergence(serverId);
      await onRefreshServers();
    } catch (err: any) {
      onError(`Failed to sync server mods: ${err}`);
    } finally {
      setIsSyncing((prev) => ({ ...prev, [serverId]: false }));
    }
  };

  const handleDeleteServer = async () => {
    if (!serverToDelete) return;
    try {
      await invoke("delete_server", { serverId: serverToDelete.id });
      onNotify(`Deleted server "${serverToDelete.name}".`);
      setServerToDelete(null);
      await onRefreshServers();
    } catch (err: any) {
      onError(`Failed to delete server: ${err}`);
      setServerToDelete(null);
    }
  };

  return (
    <div className="relative z-10 flex flex-col h-full min-h-0 bg-[#0d121c] border border-[#243144] rounded-2xl shadow-2xl overflow-hidden">
      {/* Top Header Toolbar */}
      <div className="px-6 py-4 bg-[#131b29] border-b border-[#243144] flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-emerald-500/20 border border-emerald-400/40 flex items-center justify-center text-emerald-300">
            <Server className="w-4 h-4" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-sm font-extrabold tracking-wide uppercase text-[#e4e8ee]">
                Dedicated Servers ({profileServers.length})
              </h2>
              <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-[#182232] text-[#5ce1e6] border border-[#263042]">
                Profile: {profile.name}
              </span>
            </div>
            <p className="text-[11px] text-[#8fa0bc]">
              Pre-installed server instances generated from this modpack profile.
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2.5">
          <button
            onClick={onCreateServer}
            className="px-3.5 py-2 rounded-xl bg-gradient-to-r from-emerald-500 to-teal-400 hover:from-emerald-400 hover:to-teal-300 text-slate-950 text-xs font-bold flex items-center gap-1.5 transition-all cursor-pointer shadow-lg shadow-emerald-950/40"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Create Another Server (Beta)</span>
          </button>
        </div>
      </div>

      {/* Servers List Area */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4 custom-scrollbar">
        {profileServers.length === 0 ? (
          <div className="h-64 flex flex-col items-center justify-center text-center p-8 rounded-2xl border border-dashed border-[#263449] bg-[#090d14]/50">
            <Server className="w-10 h-10 text-[#475569] mb-3" />
            <h3 className="text-sm font-bold text-[#c6d0df] mb-1">
              No Servers Created For This Profile Yet
            </h3>
            <p className="text-xs text-[#8fa0bc] max-w-md mb-4">
              You can generate a dedicated server from {profile.name} anytime. Client-only mods
              will be filtered automatically.
            </p>
            <button
              onClick={onCreateServer}
              className="px-4 py-2 rounded-xl bg-[#12c3ca]/20 hover:bg-[#12c3ca]/30 border border-[#12c3ca]/50 text-[#5ce1e6] text-xs font-bold flex items-center gap-2 transition-colors cursor-pointer"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Create Server (Beta)</span>
            </button>
          </div>
        ) : (
          profileServers.map((server) => {
            const status = serverStatuses[server.id];
            const div = divergences[server.id];
            const hasDivergence =
              div && (div.added_to_profile.length > 0 || div.removed_from_profile.length > 0);
            const isPingLoading = isPinging[server.id];
            const isSyncLoading = isSyncing[server.id];

            return (
              <div
                key={server.id}
                className="rounded-2xl border border-[#243144] bg-[#111723]/80 overflow-hidden shadow-lg hover:border-[#2e3d56] transition-all"
              >
                {/* Server Card Header */}
                <div className="p-4 bg-[#141c2b] border-b border-[#202b3d] flex items-center justify-between gap-4">
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-3.5 h-3.5 rounded-full ${
                        status?.online
                          ? "bg-emerald-400 shadow-[0_0_10px_#34d399]"
                          : "bg-slate-600"
                      }`}
                    />
                    <div>
                      <div className="flex items-center gap-2">
                        <h3 className="text-sm font-bold text-[#e4e8ee]">{server.name}</h3>
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-[#182232] text-[#8fa0bc] border border-[#243144]">
                          {server.loader.toUpperCase()} {server.mc_version}
                        </span>
                      </div>
                      <p className="text-[11px] font-mono text-[#8fa0bc] truncate max-w-md">
                        {server.server_dir}
                      </p>
                    </div>
                  </div>

                  {/* Actions Toolbar */}
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => pingServer(server.id)}
                      disabled={isPingLoading}
                      title="Ping Server"
                      className="px-2.5 py-1.5 rounded-lg bg-[#0e121a] hover:bg-[#1e293b] text-[#8fa0bc] hover:text-[#5ce1e6] border border-[#263042] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
                    >
                      <RefreshCw
                        className={`w-3.5 h-3.5 ${isPingLoading ? "animate-spin" : ""}`}
                      />
                      <span>{isPingLoading ? "Pinging..." : "Refresh Status"}</span>
                    </button>

                    <button
                      onClick={() => onOpenFolder(server.server_dir)}
                      title="Open Server Directory"
                      className="px-2.5 py-1.5 rounded-lg bg-[#0e121a] hover:bg-[#1e293b] text-[#8fa0bc] hover:text-[#f59e0b] border border-[#263042] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer"
                    >
                      <FolderOpen className="w-3.5 h-3.5" />
                      <span>Open Folder</span>
                    </button>

                    <button
                      onClick={() => setServerToDelete(server)}
                      title="Delete Server"
                      className="p-1.5 rounded-lg bg-[#0e121a] hover:bg-rose-950/70 text-[#8fa0bc] hover:text-rose-400 border border-[#263042] hover:border-rose-500/40 transition-colors cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                {/* Server Status Stats Bar */}
                <div className="px-5 py-3.5 grid grid-cols-4 gap-4 bg-[#0a0e16]/60 text-xs border-b border-[#1c2636]">
                  <div>
                    <span className="text-[11px] text-[#8fa0bc] block">SLP Status</span>
                    <span
                      className={`font-mono font-bold ${
                        status?.online ? "text-emerald-400" : "text-slate-400"
                      }`}
                    >
                      {status?.online ? `Online (${status.ping_ms} ms)` : "Offline (Not Running)"}
                    </span>
                  </div>

                  <div>
                    <span className="text-[11px] text-[#8fa0bc] block">Players</span>
                    <span className="font-mono font-bold text-[#e4e8ee]">
                      {status?.online ? `${status.online_players} / ${status.max_players}` : "—"}
                    </span>
                  </div>

                  <div>
                    <span className="text-[11px] text-[#8fa0bc] block">Server MOTD</span>
                    <span className="font-mono text-[#c6d0df] truncate block">
                      {status?.online ? status.motd : "Start via run.sh or run.bat"}
                    </span>
                  </div>

                  <div>
                    <span className="text-[11px] text-[#8fa0bc] block">Server Mods</span>
                    <span className="font-mono font-bold text-[#5ce1e6]">
                      {server.installed_mods.length} Mods Installed
                    </span>
                  </div>
                </div>

                {/* Divergence Alert & Sync Banner (Feature 6) */}
                {hasDivergence && (
                  <div className="p-3.5 bg-amber-500/10 border-t border-amber-500/30 flex items-center justify-between gap-4">
                    <div className="flex items-center gap-2.5">
                      <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" />
                      <div className="text-xs">
                        <span className="font-bold text-amber-300">
                          Profile mods have changed:
                        </span>{" "}
                        <span className="text-[#c6d0df]">
                          {div.added_to_profile.length > 0 &&
                            `+${div.added_to_profile.length} added to profile `}
                          {div.removed_from_profile.length > 0 &&
                            `-${div.removed_from_profile.length} removed from profile`}
                        </span>
                      </div>
                    </div>

                    <button
                      onClick={() => handleSyncServer(server.id)}
                      disabled={isSyncLoading}
                      className="px-3 py-1.5 rounded-lg bg-amber-500 hover:bg-amber-400 text-slate-950 font-extrabold text-xs flex items-center gap-1.5 transition-all cursor-pointer shadow-md disabled:opacity-50"
                    >
                      <RefreshCw
                        className={`w-3.5 h-3.5 ${isSyncLoading ? "animate-spin" : ""}`}
                      />
                      <span>{isSyncLoading ? "Syncing..." : "Sync Server Mods Now"}</span>
                    </button>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Delete Confirmation Modal */}
      {serverToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4">
          <div className="w-full max-w-sm bg-[#0e131d] border border-rose-500/40 rounded-2xl shadow-2xl p-5 space-y-4">
            <div className="flex items-center gap-2.5 text-rose-400 font-bold text-sm">
              <Trash2 className="w-4 h-4" />
              <span>Delete Dedicated Server</span>
            </div>
            <p className="text-xs text-[#c6d0df] leading-relaxed">
              Are you sure you want to delete server instance{" "}
              <strong className="text-white">&quot;{serverToDelete.name}&quot;</strong>? This
              will remove the server folder and its world data permanently.
            </p>
            <div className="flex items-center justify-end gap-2.5 pt-2">
              <button
                onClick={() => setServerToDelete(null)}
                className="px-3.5 py-1.5 rounded-xl bg-[#182232] hover:bg-[#222f43] text-xs font-bold text-[#8fa0bc] transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={handleDeleteServer}
                className="px-4 py-1.5 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold transition-colors cursor-pointer shadow-lg shadow-rose-950/50"
              >
                Delete Server
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
