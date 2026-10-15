import React from "react";
import { Server, Users, Wifi, RotateCw } from "lucide-react";
import { ServerStatus } from "../types";

interface ServerCardProps {
  status: ServerStatus | null;
  onRefresh: () => void;
  isLoading: boolean;
}

export const ServerCard: React.FC<ServerCardProps> = ({
  status,
  onRefresh,
  isLoading,
}) => {
  const isOnline = status?.online ?? false;
  const ping = status?.ping_ms ?? 0;
  const playersOnline = status?.online_players ?? 0;
  const maxPlayers = status?.max_players ?? 20;

  // Latency color
  const getPingColor = (ms: number) => {
    if (!isOnline) return "text-slate-500";
    if (ms < 70) return "text-emerald-400";
    if (ms < 150) return "text-amber-400";
    return "text-red-400";
  };

  return (
    <div className="bg-[#141920]/80 border border-[#26303d] rounded-xl p-4 backdrop-blur-md shadow-lg shadow-black/40 min-w-[280px]">
      <div className="flex items-center justify-between mb-2.5">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 rounded bg-amber-500/10 border border-amber-500/30 flex items-center justify-center">
            <Server className="w-3.5 h-3.5 text-amber-400" />
          </div>
          <span className="font-semibold text-sm text-slate-100">
            {status?.server_name || "Create Server"}
          </span>
        </div>

        {/* Refresh button */}
        <button
          onClick={onRefresh}
          disabled={isLoading}
          title="Refresh server status"
          className="p-1 rounded text-slate-400 hover:text-amber-400 hover:bg-white/5 transition-colors cursor-pointer disabled:opacity-40"
        >
          <RotateCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin" : ""}`} />
        </button>
      </div>

      <div className="flex items-center justify-between text-xs pt-1 border-t border-[#202833]">
        {/* Status Dot */}
        <div className="flex items-center gap-2">
          <span
            className={`w-2 h-2 rounded-full ${
              isOnline
                ? "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.7)]"
                : "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.7)]"
            }`}
          />
          <span className={isOnline ? "text-emerald-300 font-medium" : "text-red-400 font-medium"}>
            {isOnline ? "Server Online" : "Server Offline"}
          </span>
        </div>

        {/* Player Count */}
        <div className="flex items-center gap-1 text-slate-300">
          <Users className="w-3.5 h-3.5 text-slate-400" />
          <span>
            {playersOnline} / {maxPlayers}
          </span>
        </div>

        {/* Ping */}
        <div className="flex items-center gap-1">
          <Wifi className={`w-3.5 h-3.5 ${getPingColor(ping)}`} />
          <span className={`font-mono text-[11px] ${getPingColor(ping)}`}>
            {isOnline ? `${ping}ms` : "--"}
          </span>
        </div>
      </div>
    </div>
  );
};
