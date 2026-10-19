import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, ExternalLink, RefreshCw } from "lucide-react";

interface ChangelogDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  version: string;
}

export const ChangelogDrawer: React.FC<ChangelogDrawerProps> = ({
  isOpen,
  onClose,
  version,
}) => {
  const [logText, setLogText] = useState<string>("");
  const [isLoading, setIsLoading] = useState<boolean>(false);

  useEffect(() => {
    if (isOpen) {
      loadLog();
    }
  }, [isOpen]);

  const loadLog = async () => {
    setIsLoading(true);
    try {
      const res = await invoke<string>("get_update_log");
      setLogText(res);
    } catch (err) {
      setLogText("Failed to load server update log.");
    } finally {
      setIsLoading(false);
    }
  };

  if (!isOpen) return null;

  // Simple and safe markdown line formatter
  const renderFormattedLine = (line: string, index: number) => {
    const trimmed = line.trim();

    if (trimmed.startsWith("# ")) {
      return (
        <h1 key={index} className="text-lg font-extrabold text-[#f59e0b] my-2">
          {trimmed.replace(/^#\s+/, "")}
        </h1>
      );
    }

    if (trimmed.startsWith("## ")) {
      return (
        <div key={index} className="mt-6 pt-4 border-t border-[#263042]/80">
          <h2 className="text-sm font-bold text-[#12c3ca] flex items-center gap-2">
            <span>{trimmed.replace(/^##\s+/, "")}</span>
          </h2>
        </div>
      );
    }

    if (trimmed.startsWith("### ")) {
      return (
        <h3 key={index} className="text-xs font-bold text-[#5ce1e6] mt-3 mb-1">
          {trimmed.replace(/^###\s+/, "")}
        </h3>
      );
    }

    if (trimmed.startsWith("- ")) {
      const content = trimmed.replace(/^-\s+/, "");
      return (
        <div key={index} className="flex items-start gap-2 text-[11px] text-[#cad2df] my-1.5 pl-2 leading-relaxed">
          <span className="w-1.5 h-1.5 rounded-full bg-[#12c3ca] shrink-0 mt-1.5" />
          <span dangerouslySetInnerHTML={{ __html: formatInlineMarkdown(content) }} />
        </div>
      );
    }

    if (trimmed === "---") {
      return <hr key={index} className="border-[#263042] my-4" />;
    }

    if (trimmed.length === 0) {
      return <div key={index} className="h-1.5" />;
    }

    return (
      <p
        key={index}
        className="text-xs text-[#8fa0bc] my-1 leading-relaxed"
        dangerouslySetInnerHTML={{ __html: formatInlineMarkdown(trimmed) }}
      />
    );
  };

  const escapeHtml = (unsafe: string): string => {
    return unsafe
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  };

  const formatInlineMarkdown = (text: string): string => {
    return escapeHtml(text)
      .replace(/\*\*(.*?)\*\*/g, "<strong class='text-[#e4e8ee] font-semibold'>$1</strong>")
      .replace(/`([^`]+)`/g, "<code class='px-1.5 py-0.5 rounded bg-[#182232] text-[#5ce1e6] font-mono text-[10.5px] border border-[#263042]'>$1</code>")
      .replace(/\*([^*]+)\*/g, "<em class='text-[#8fa0bc]'>$1</em>");
  };

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-end bg-black/75">
      <div className="w-full max-w-xl h-full bg-[#0e121a] border-l border-[#263042] shadow-2xl flex flex-col animate-in slide-in-from-right duration-200">
        {/* FancyMenu Header Bar */}
        <div className="bg-[#182232] px-6 py-4 border-b border-[#263042] flex items-center justify-between shadow-md">
          <div className="flex items-center gap-1.5 font-mono text-sm">
            <span className="font-extrabold text-[#f59e0b]">SERVER UPDATE LOG</span>
            <span className="text-[#3a465a]">|</span>
            <span className="font-semibold text-[#12c3ca]">Create Server</span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={loadLog}
              disabled={isLoading}
              title="Refresh log"
              className="p-1 rounded text-[#8fa0bc] hover:text-[#12c3ca] hover:bg-white/5 transition-colors cursor-pointer"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin text-[#12c3ca]" : ""}`} />
            </button>
            <button
              onClick={() => invoke("open_url", { url: "https://discord.gg/CFqskvhF8k" })}
              className="px-2.5 py-1 rounded bg-[#5865F2]/20 hover:bg-[#5865F2]/30 border border-[#5865F2]/50 text-[#9ba5fe] hover:text-white text-[11px] font-bold flex items-center gap-1.5 transition-colors cursor-pointer"
            >
              <ExternalLink className="w-3 h-3" />
              Discord
            </button>
            <button
              onClick={onClose}
              className="p-1 rounded text-[#8fa0bc] hover:text-[#12c3ca] hover:bg-white/5 transition-colors cursor-pointer"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto p-6 space-y-2 text-xs text-[#e4e8ee]">
          {/* Header Info Card */}
          <div className="p-3.5 mb-4 rounded-xl bg-[#182232]/80 border border-[#263042] flex items-center justify-between">
            <div>
              <span className="font-bold text-[#f59e0b] block text-sm">Create Server Official Updates</span>
              <span className="text-[11px] text-[#8fa0bc]">Minecraft 1.20.1 (Forge 47.4.20)</span>
            </div>
            <span className="px-2.5 py-1 rounded bg-[#12c3ca]/15 border border-[#12c3ca]/40 text-[#5ce1e6] font-mono font-bold text-xs">
              {version || "v1.0.0"}
            </span>
          </div>

          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-20 text-[#8fa0bc] gap-3">
              <RefreshCw className="w-6 h-6 animate-spin text-[#12c3ca]" />
              <span className="text-xs">Loading latest update notes...</span>
            </div>
          ) : (
            <div className="space-y-1">
              {logText.split("\n").map((line, idx) => renderFormattedLine(line, idx))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
