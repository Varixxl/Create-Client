import React, { useState } from "react";
import { X, Copy, Check, ExternalLink, ShieldCheck } from "lucide-react";
import { DeviceCodeInfo } from "../types";

interface LoginModalProps {
  isOpen: boolean;
  onClose: () => void;
  deviceInfo: DeviceCodeInfo | null;
}

export const LoginModal: React.FC<LoginModalProps> = ({
  isOpen,
  onClose,
  deviceInfo,
}) => {
  const [copied, setCopied] = useState(false);

  if (!isOpen || !deviceInfo) return null;

  const handleCopy = () => {
    navigator.clipboard.writeText(deviceInfo.user_code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4 animate-in fade-in duration-150">
      <div className="bg-[#0e121af8] border border-[#263042] w-full max-w-md rounded-2xl shadow-2xl overflow-hidden flex flex-col">
        {/* FancyMenu Header Bar */}
        <div className="px-5 py-3.5 bg-[#182232] border-b border-[#263042] flex items-center justify-between shadow-md">
          <div className="flex items-center gap-1.5 font-mono text-xs">
            <span className="font-extrabold text-[#f59e0b]">MICROSOFT LOGIN</span>
            <span className="text-[#3a465a]">|</span>
            <span className="font-semibold text-[#12c3ca]">Authentication</span>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded text-[#8fa0bc] hover:text-[#12c3ca] hover:bg-white/5 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Modal Body */}
        <div className="p-6 text-center space-y-5 text-xs text-[#e4e8ee]">
          <div className="w-12 h-12 mx-auto rounded-full bg-[#12c3ca]/10 border border-[#12c3ca]/40 flex items-center justify-center shadow-lg shadow-cyan-950/40">
            <ShieldCheck className="w-6 h-6 text-[#12c3ca]" />
          </div>

          <div className="space-y-1">
            <h3 className="text-base font-bold text-white">Enter Your Login Code</h3>
            <p className="text-xs text-[#8fa0bc] max-w-xs mx-auto">
              We opened <a href={deviceInfo.verification_uri} target="_blank" rel="noreferrer" className="text-[#12c3ca] underline">microsoft.com/link</a> in your browser. Enter this code to link your Minecraft account:
            </p>
          </div>

          {/* User Code Display Card */}
          <div className="p-4 rounded-xl bg-[#182232] border border-[#263042] space-y-3">
            <div className="font-mono text-2xl font-black tracking-widest text-[#5ce1e6] drop-shadow-[0_0_10px_rgba(18,195,202,0.4)] select-all">
              {deviceInfo.user_code}
            </div>

            <div className="flex items-center justify-center gap-2">
              <button
                onClick={handleCopy}
                className="px-4 py-2 bg-[#0e121a] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-xs font-bold rounded-lg text-[#e4e8ee] flex items-center gap-1.5 transition-all cursor-pointer"
              >
                {copied ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Copied to Clipboard!</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5 text-[#12c3ca]" />
                    <span>Copy Code</span>
                  </>
                )}
              </button>

              <a
                href={deviceInfo.verification_uri}
                target="_blank"
                rel="noreferrer"
                className="px-4 py-2 bg-[#12c3ca]/15 hover:bg-[#12c3ca]/25 border border-[#12c3ca]/40 text-xs font-bold rounded-lg text-[#5ce1e6] flex items-center gap-1.5 transition-all"
              >
                <ExternalLink className="w-3.5 h-3.5" />
                <span>Open Browser</span>
              </a>
            </div>
          </div>

          {/* Waiting animation */}
          <div className="flex items-center justify-center gap-2 pt-1 text-[11px] text-[#8fa0bc]">
            <div className="w-3.5 h-3.5 border-2 border-[#3a465a] border-t-[#12c3ca] rounded-full animate-spin" />
            <span>Waiting for approval in browser...</span>
          </div>
        </div>

        {/* Footer */}
        <div className="px-5 py-3 bg-[#182232] border-t border-[#263042] flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-lg text-xs font-medium text-[#8fa0bc] hover:text-[#e4e8ee] hover:bg-white/5 transition-colors cursor-pointer"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
};
