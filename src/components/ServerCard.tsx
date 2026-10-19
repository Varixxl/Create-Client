import React from "react";
import { Users, Wifi, RotateCw } from "lucide-react";
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

  const getPingColor = (ms: number) => {
    if (!isOnline) return "text-[#8fa0bc]";
    if (ms < 70) return "text-[#12c3ca]";
    if (ms < 150) return "text-[#f59e0b]";
    return "text-red-400";
  };

  return (
    <div className="bg-[#0e121af8] border border-[#263042] rounded-xl overflow-hidden shadow-xl min-w-[300px]">
      {/* FancyMenu Top Bar */}
      <div className="bg-[#182232] px-3.5 py-2 border-b border-[#263042] flex items-center justify-between">
        <div className="flex items-center gap-1.5 font-mono text-xs">
          <span className="font-bold text-[#f59e0b]">SERVER</span>
          <span className="text-[#3a465a]">|</span>
          <span className="font-semibold text-[#12c3ca]">Create Server</span>
        </div>

        <button
          onClick={onRefresh}
          disabled={isLoading}
          title="Refresh server status"
          className="p-1 rounded text-[#8fa0bc] hover:text-[#12c3ca] hover:bg-white/5 transition-colors cursor-pointer disabled:opacity-40"
        >
          <RotateCw className={`w-3 h-3 ${isLoading ? "animate-spin" : ""}`} />
        </button>
      </div>

      {/* Body info */}
      <div className="p-3.5 space-y-2 text-xs">
        <div className="flex items-center justify-between">
          {/* Status Indicator */}
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                isOnline
                  ? "bg-[#12c3ca] shadow-[0_0_8px_rgba(18,195,202,0.8)]"
                  : "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.8)]"
              }`}
            />
            <span className={isOnline ? "text-[#5ce1e6] font-semibold" : "text-red-400 font-semibold"}>
              {isOnline ? "Server Online" : "Server Offline"}
            </span>
          </div>

          {/* Latency */}
          <div className="flex items-center gap-1">
            <Wifi className={`w-3.5 h-3.5 ${getPingColor(ping)}`} />
            <span className={`font-mono text-[11px] ${getPingColor(ping)}`}>
              {isOnline ? `${ping}ms` : "--"}
            </span>
          </div>
        </div>

        {/* Player stats bar */}
        <div className="flex items-center justify-between pt-1 border-t border-[#1c2432] text-[#8fa0bc]">
          <span className="text-[11px]">Online Players:</span>
          <div className="flex items-center gap-1 text-[#e4e8ee] font-mono font-medium">
            <Users className="w-3 h-3 text-[#12c3ca]" />
            <span>
              {playersOnline} / {maxPlayers}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
