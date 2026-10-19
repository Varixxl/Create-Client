import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Terminal,
  Copy,
  Trash2,
  FolderOpen,
  Search,
  AlertTriangle,
  RefreshCw,
  Check,
  Square,
} from "lucide-react";
import { InstanceLogsPayload, ModpackProfile } from "../types";

interface ConsolePanelProps {
  profile: ModpackProfile;
  isRunning: boolean;
  onForceClose: () => void;
  isForceClosing: boolean;
  onOpenFolder: (path: string) => void;
}

export const ConsolePanel: React.FC<ConsolePanelProps> = ({
  profile,
  isRunning,
  onForceClose,
  isForceClosing,
  onOpenFolder,
}) => {
  const [logs, setLogs] = useState<InstanceLogsPayload | null>(null);
  const [levelFilter, setLevelFilter] = useState<"ALL" | "WARN" | "ERROR">(
    "ALL"
  );
  const [searchText, setSearchText] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const [copied, setCopied] = useState(false);
  const [showCrashReport, setShowCrashReport] = useState(false);
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  const fetchLogs = async () => {
    try {
      const data = await invoke<InstanceLogsPayload>("get_instance_logs", {
        profileId: profile.id,
        maxLines: 600,
      });
      setLogs(data);
    } catch (err) {
      console.error("Failed to read instance logs:", err);
    }
  };

  useEffect(() => {
    fetchLogs();
    const interval = setInterval(fetchLogs, isRunning ? 1500 : 4000);
    return () => clearInterval(interval);
  }, [profile.id, isRunning]);

  useEffect(() => {
    if (autoScroll && scrollContainerRef.current) {
      scrollContainerRef.current.scrollTop =
        scrollContainerRef.current.scrollHeight;
    }
  }, [logs?.lines.length, autoScroll]);

  const classifyLine = (line: string): "ERROR" | "WARN" | "INFO" => {
    const upper = line.toUpperCase();
    if (
      upper.includes("/ERROR]") ||
      upper.includes("[ERROR]") ||
      upper.includes("EXCEPTION") ||
      upper.includes("STDERR") ||
      upper.startsWith("\tAT ")
    ) {
      return "ERROR";
    }
    if (upper.includes("/WARN]") || upper.includes("[WARN]")) {
      return "WARN";
    }
    return "INFO";
  };

  const filteredLines = (logs?.lines || []).filter((line) => {
    const lvl = classifyLine(line);
    if (levelFilter === "ERROR" && lvl !== "ERROR") return false;
    if (levelFilter === "WARN" && lvl === "INFO") return false;
    if (
      searchText.trim() &&
      !line.toLowerCase().includes(searchText.toLowerCase())
    ) {
      return false;
    }
    return true;
  });

  const handleCopyLogs = async () => {
    const text = filteredLines.join("\n");
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    } catch {
      // ignore
    }
  };

  const handleClearLogs = async () => {
    try {
      await invoke("clear_instance_logs", { profileId: profile.id });
      await fetchLogs();
    } catch (err) {
      console.error("Failed to clear logs:", err);
    }
  };

  return (
    <div className="relative z-10 flex flex-col h-full min-h-0 bg-[#0b0f17]/95 border border-[#243144] rounded-2xl shadow-2xl overflow-hidden backdrop-blur-md">
      {/* Top Console Toolbar */}
      <div className="px-5 py-3 bg-[#131b29] border-b border-[#243144] flex items-center justify-between gap-3 flex-wrap">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Terminal className="w-4 h-4 text-[#12c3ca]" />
            <span className="text-xs font-extrabold uppercase tracking-wider text-[#e4e8ee]">
              Live Game Output
            </span>
          </div>
          <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-[#0c1018] border border-[#243144] text-[#5ce1e6]">
            {logs?.log_source || "launcher_log.txt"}
          </span>

          {/* Level Filter Pills */}
          <div className="flex items-center bg-[#090d14] p-0.5 rounded-lg border border-[#243144]">
            {(["ALL", "WARN", "ERROR"] as const).map((lvl) => (
              <button
                key={lvl}
                onClick={() => setLevelFilter(lvl)}
                className={`px-2.5 py-1 rounded-md text-[10px] font-extrabold transition-colors cursor-pointer ${
                  levelFilter === lvl
                    ? lvl === "ERROR"
                      ? "bg-rose-500/25 text-rose-300"
                      : lvl === "WARN"
                      ? "bg-amber-500/25 text-amber-300"
                      : "bg-[#12c3ca]/25 text-[#5ce1e6]"
                    : "text-[#8fa0bc] hover:text-white"
                }`}
              >
                {lvl === "WARN" ? "WARN+" : lvl}
              </button>
            ))}
          </div>
        </div>

        {/* Right Controls */}
        <div className="flex items-center gap-2">
          <div className="relative w-44">
            <Search className="w-3.5 h-3.5 text-[#8fa0bc] absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              placeholder="Search log lines..."
              className="w-full pl-8 pr-2.5 py-1 rounded-lg bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none"
            />
          </div>

          <label className="flex items-center gap-1.5 text-[11px] font-bold text-[#8fa0bc] cursor-pointer px-2 py-1 rounded-lg bg-[#090d14] border border-[#243144]">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="accent-[#12c3ca] w-3.5 h-3.5 cursor-pointer"
            />
            <span>Auto-Scroll</span>
          </label>

          <button
            onClick={fetchLogs}
            title="Refresh Logs"
            className="p-1.5 rounded-lg bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#8fa0bc] hover:text-[#12c3ca] transition-colors cursor-pointer"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>

          <button
            onClick={handleCopyLogs}
            title="Copy Log Output"
            className="px-2.5 py-1 rounded-lg bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#e4e8ee] hover:text-[#5ce1e6] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 text-emerald-400" />
                <span className="text-emerald-300">Copied</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5 text-[#12c3ca]" />
                <span>Copy</span>
              </>
            )}
          </button>

          <button
            onClick={handleClearLogs}
            title="Clear Launcher Log"
            className="p-1.5 rounded-lg bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#8fa0bc] hover:text-amber-300 transition-colors cursor-pointer"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>

          <button
            onClick={() => onOpenFolder(`${profile.instance_dir}/logs`)}
            title="Open Instance Logs Folder"
            className="px-2.5 py-1 rounded-lg bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[#e4e8ee] hover:text-[#f59e0b] text-xs font-bold flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <FolderOpen className="w-3.5 h-3.5 text-[#f59e0b]" />
            <span>Logs</span>
          </button>

          {isRunning && (
            <button
              onClick={onForceClose}
              disabled={isForceClosing}
              className="px-3 py-1 rounded-lg bg-rose-950/90 hover:bg-rose-600 border border-rose-500/60 text-rose-200 hover:text-white text-xs font-extrabold flex items-center gap-1.5 transition-colors cursor-pointer"
            >
              <Square className="w-3 h-3 fill-current" />
              <span>{isForceClosing ? "Killing..." : "Kill Process"}</span>
            </button>
          )}
        </div>
      </div>

      {/* Crash Report Banner if detected */}
      {logs?.crash_report && (
        <div className="px-4 py-2.5 bg-rose-950/60 border-b border-rose-500/40 flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-xs font-bold text-rose-200">
              <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0" />
              <span>
                Crash report detected in instance{" "}
                <code className="font-mono text-rose-300">crash-reports/</code>{" "}
                folder
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setShowCrashReport(!showCrashReport)}
                className="px-2.5 py-1 rounded bg-rose-500/20 hover:bg-rose-500/30 border border-rose-400/40 text-rose-200 text-[11px] font-bold cursor-pointer"
              >
                {showCrashReport ? "Hide Crash Header" : "Inspect Crash Header"}
              </button>
              <button
                onClick={() =>
                  onOpenFolder(`${profile.instance_dir}/crash-reports`)
                }
                className="px-2.5 py-1 rounded bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-xs font-bold text-[#e4e8ee] cursor-pointer"
              >
                Open Crash Reports
              </button>
            </div>
          </div>
          {showCrashReport && (
            <pre className="p-2.5 rounded-lg bg-black/60 border border-rose-500/30 text-[11px] font-mono text-rose-200 overflow-x-auto max-h-36">
              {logs.crash_report}
            </pre>
          )}
        </div>
      )}

      {/* Terminal Output Stream */}
      <div
        ref={scrollContainerRef}
        className="flex-1 overflow-y-auto p-4 font-mono text-[11px] leading-relaxed select-text space-y-0.5"
      >
        {filteredLines.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-[#8fa0bc] space-y-1 select-none">
            <Terminal className="w-6 h-6 text-[#12c3ca]/50 mb-1" />
            <p className="text-xs font-bold text-[#c6d0df]">
              No console output to display for {profile.name}
            </p>
            <p className="text-[11px]">
              Launch the profile to stream live JVM and Minecraft output here.
            </p>
          </div>
        ) : (
          filteredLines.map((line, idx) => {
            const lvl = classifyLine(line);
            const colorClass =
              lvl === "ERROR"
                ? "text-rose-300 bg-rose-950/30 px-1.5 py-0.5 rounded"
                : lvl === "WARN"
                ? "text-amber-300 bg-amber-950/20 px-1.5 py-0.5 rounded"
                : "text-[#c6d0df]";
            return (
              <div
                key={idx}
                className={`whitespace-pre-wrap break-all ${colorClass}`}
              >
                {line}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
