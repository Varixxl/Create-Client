import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  X,
  Plus,
  Sliders,
  Copy,
  Zap,
  Box,
  Cpu,
  Sparkles,
  Flame,
  Layers,
} from "lucide-react";
import { ModpackProfile, ModloaderVersionItem } from "../types";
import { CustomDropdown } from "./CustomDropdown";

export type ProfileModalMode = "create" | "edit" | "duplicate";

interface ProfileModalProps {
  isOpen: boolean;
  mode: ProfileModalMode;
  targetProfile: ModpackProfile | null;
  defaultRamGb: number;
  defaultJvmFlags: string;
  onClose: () => void;
  onSaveCreate: (data: {
    name: string;
    description: string;
    loader: string;
    loaderVersion?: string;
    mcVersion: string;
    includeDefaultFabricMods: boolean;
    ramGb: number | null;
    jvmFlags: string | null;
    javaVersion?: number | null;
  }) => Promise<void>;
  onSaveEdit: (data: {
    profileId: string;
    name: string;
    description: string;
    ramGb: number | null;
    jvmFlags: string | null;
    javaVersion?: number | null;
  }) => Promise<void>;
  onSaveDuplicate: (sourceId: string, newName: string) => Promise<void>;
}

export const ProfileModal: React.FC<ProfileModalProps> = ({
  isOpen,
  mode,
  targetProfile,
  defaultRamGb,
  defaultJvmFlags,
  onClose,
  onSaveCreate,
  onSaveEdit,
  onSaveDuplicate,
}) => {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [loader, setLoader] = useState<"fabric" | "quilt" | "forge" | "neoforge" | "vanilla">("fabric");
  const [loaderVersion, setLoaderVersion] = useState("");
  const [loaderVersionsList, setLoaderVersionsList] = useState<ModloaderVersionItem[]>([]);
  const [mcVersion, setMcVersion] = useState("latest");
  const [includeSnapshots, setIncludeSnapshots] = useState(false);
  const [includeDefaultFabricMods, setIncludeDefaultFabricMods] = useState(true);
  const [useCustomRam, setUseCustomRam] = useState(false);
  const [ramGb, setRamGb] = useState(defaultRamGb || 6);
  const [useCustomFlags, setUseCustomFlags] = useState(false);
  const [jvmFlags, setJvmFlags] = useState(defaultJvmFlags || "");
  const [javaVersion, setJavaVersion] = useState<number | null>(null);
  const [mcVersions, setMcVersions] = useState<string[]>([
    "1.21.5",
    "1.21.4",
    "1.21.1",
    "1.20.4",
    "1.20.1",
  ]);
  const [isSaving, setIsSaving] = useState(false);

  // Fetch MC versions with snapshot toggle support
  const fetchMcVersions = (snapshots: boolean) => {
    invoke<string[]>("get_available_mc_versions", { includeSnapshots: snapshots })
      .then((vers) => {
        if (vers && vers.length > 0) {
          setMcVersions(vers);
        }
      })
      .catch(() => {});
  };

  useEffect(() => {
    if (!isOpen) return;
    fetchMcVersions(includeSnapshots);

    if (mode === "create") {
      setName("");
      setDescription("");
      setLoader("fabric");
      setLoaderVersion("");
      setMcVersion("latest");
      setIncludeDefaultFabricMods(true);
      setUseCustomRam(false);
      setRamGb(defaultRamGb || 6);
      setUseCustomFlags(false);
      setJvmFlags(defaultJvmFlags || "");
      setJavaVersion(null);
    } else if (mode === "edit" && targetProfile) {
      setName(targetProfile.name);
      setDescription(targetProfile.description);
      setUseCustomRam(targetProfile.ram_gb != null);
      setRamGb(targetProfile.ram_gb ?? defaultRamGb);
      setUseCustomFlags(targetProfile.jvm_flags != null && targetProfile.jvm_flags.trim() !== "");
      setJvmFlags(targetProfile.jvm_flags ?? defaultJvmFlags);
      setJavaVersion(targetProfile.java_version ?? null);
    } else if (mode === "duplicate" && targetProfile) {
      setName(`${targetProfile.name} (Copy)`);
      setJavaVersion(targetProfile.java_version ?? null);
    }
  }, [isOpen, mode, targetProfile, defaultRamGb, defaultJvmFlags]);

  // Fetch modloader versions whenever loader or MC version changes
  useEffect(() => {
    if (!isOpen || loader === "vanilla") {
      setLoaderVersionsList([]);
      setLoaderVersion("");
      return;
    }

    const targetVer = mcVersion === "latest" ? "1.20.1" : mcVersion;
    invoke<ModloaderVersionItem[]>("get_modloader_versions", {
      mcVersion: targetVer,
      loader,
    })
      .then((items) => {
        setLoaderVersionsList(items || []);
        if (items && items.length > 0) {
          const rec = items.find((i) => i.recommended) || items[0];
          setLoaderVersion(rec.version);
        } else {
          setLoaderVersion("");
        }
      })
      .catch(() => {
        setLoaderVersionsList([]);
      });
  }, [isOpen, loader, mcVersion]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setIsSaving(true);
    try {
      if (mode === "create") {
        await onSaveCreate({
          name: name.trim(),
          description: description.trim(),
          loader,
          loaderVersion: loader !== "vanilla" && loaderVersion ? loaderVersion : undefined,
          mcVersion,
          includeDefaultFabricMods: loader === "fabric" && includeDefaultFabricMods,
          ramGb: useCustomRam ? ramGb : null,
          jvmFlags: useCustomFlags && jvmFlags.trim() ? jvmFlags.trim() : null,
          javaVersion,
        });
      } else if (mode === "edit" && targetProfile) {
        await onSaveEdit({
          profileId: targetProfile.id,
          name: name.trim(),
          description: description.trim(),
          ramGb: useCustomRam ? ramGb : null,
          jvmFlags: useCustomFlags && jvmFlags.trim() ? jvmFlags.trim() : null,
          javaVersion,
        });
      } else if (mode === "duplicate" && targetProfile) {
        await onSaveDuplicate(targetProfile.id, name.trim());
      }
      onClose();
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
      <div className="w-full max-w-lg bg-[#0e131d] border border-[#2a384c] rounded-2xl shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="px-6 py-4 bg-[#141c2b] border-b border-[#243144] flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            {mode === "create" && <Plus className="w-5 h-5 text-[#12c3ca]" />}
            {mode === "edit" && <Sliders className="w-5 h-5 text-[#12c3ca]" />}
            {mode === "duplicate" && <Copy className="w-5 h-5 text-[#5ce1e6]" />}
            <h2 className="text-sm font-extrabold tracking-wide uppercase text-[#e4e8ee]">
              {mode === "create"
                ? "Create New Modpack Profile"
                : mode === "edit"
                ? `Configure Profile: ${targetProfile?.name}`
                : `Duplicate Profile: ${targetProfile?.name}`}
            </h2>
          </div>
          <button
            onClick={onClose}
            className="text-[#8fa0bc] hover:text-white p-1 rounded-lg hover:bg-white/5 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-4 max-h-[80vh] overflow-y-auto">
          {/* Profile Name */}
          <div className="space-y-1.5">
            <label className="text-xs font-bold text-[#c6d0df] block">
              Profile Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={mode === "edit" && targetProfile?.is_builtin}
              placeholder="e.g., Survival Fabric 1.21 or Builder Pack"
              className="w-full px-3.5 py-2.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none disabled:opacity-60"
              required
            />
            {mode === "edit" && targetProfile?.is_builtin && (
              <p className="text-[11px] text-[#8fa0bc]">
                Built-in profile names are pinned, but you can customize RAM and JVM overrides below.
              </p>
            )}
          </div>

          {mode !== "duplicate" && (
            <>
              {/* Description */}
              <div className="space-y-1.5">
                <label className="text-xs font-bold text-[#c6d0df] block">
                  Description (Optional)
                </label>
                <input
                  type="text"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  disabled={mode === "edit" && targetProfile?.is_builtin}
                  placeholder="Short note about this modpack instance..."
                  className="w-full px-3.5 py-2.5 rounded-xl bg-[#090d14] border border-[#263042] focus:border-[#12c3ca] text-xs text-[#e4e8ee] outline-none disabled:opacity-60"
                />
              </div>

              {/* Loader & MC Version (Create mode only) */}
              {mode === "create" && (
                <>
                  {/* Mod Loader Selector (Fabric, Quilt, Forge, NeoForge, Vanilla) */}
                  <div className="space-y-1.5">
                    <label className="text-xs font-bold text-[#c6d0df] block">
                      Mod Loader
                    </label>
                    <div className="grid grid-cols-5 gap-1.5">
                      <button
                        type="button"
                        onClick={() => setLoader("fabric")}
                        className={`py-2 px-1.5 rounded-xl border text-[11px] font-bold flex flex-col items-center justify-center gap-1 transition-all cursor-pointer ${
                          loader === "fabric"
                            ? "bg-[#12c3ca]/20 border-[#12c3ca] text-[#5ce1e6]"
                            : "bg-[#090d14] border-[#263042] text-[#8fa0bc] hover:text-white"
                        }`}
                      >
                        <Zap className="w-3.5 h-3.5 text-[#12c3ca]" />
                        <span>Fabric</span>
                      </button>
                      <button
                        type="button"
                        onClick={() => setLoader("quilt")}
                        className={`py-2 px-1.5 rounded-xl border text-[11px] font-bold flex flex-col items-center justify-center gap-1 transition-all cursor-pointer ${
                          loader === "quilt"
                            ? "bg-purple-500/20 border-purple-400 text-purple-300"
                            : "bg-[#090d14] border-[#263042] text-[#8fa0bc] hover:text-white"
                        }`}
                      >
                        <Layers className="w-3.5 h-3.5 text-purple-400" />
                        <span>Quilt</span>
                      </button>
                      <button
                        type="button"
                        onClick={() => setLoader("forge")}
                        className={`py-2 px-1.5 rounded-xl border text-[11px] font-bold flex flex-col items-center justify-center gap-1 transition-all cursor-pointer ${
                          loader === "forge"
                            ? "bg-amber-500/20 border-amber-400 text-amber-300"
                            : "bg-[#090d14] border-[#263042] text-[#8fa0bc] hover:text-white"
                        }`}
                      >
                        <Flame className="w-3.5 h-3.5 text-amber-400" />
                        <span>Forge</span>
                      </button>
                      <button
                        type="button"
                        onClick={() => setLoader("neoforge")}
                        className={`py-2 px-1.5 rounded-xl border text-[11px] font-bold flex flex-col items-center justify-center gap-1 transition-all cursor-pointer ${
                          loader === "neoforge"
                            ? "bg-orange-500/20 border-orange-400 text-orange-300"
                            : "bg-[#090d14] border-[#263042] text-[#8fa0bc] hover:text-white"
                        }`}
                      >
                        <Flame className="w-3.5 h-3.5 text-orange-400" />
                        <span>NeoForge</span>
                      </button>
                      <button
                        type="button"
                        onClick={() => setLoader("vanilla")}
                        className={`py-2 px-1.5 rounded-xl border text-[11px] font-bold flex flex-col items-center justify-center gap-1 transition-all cursor-pointer ${
                          loader === "vanilla"
                            ? "bg-emerald-500/20 border-emerald-400 text-emerald-300"
                            : "bg-[#090d14] border-[#263042] text-[#8fa0bc] hover:text-white"
                        }`}
                      >
                        <Box className="w-3.5 h-3.5 text-emerald-400" />
                        <span>Vanilla</span>
                      </button>
                    </div>
                  </div>

                  {/* Minecraft Version & Loader Version */}
                  <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between">
                        <label className="text-xs font-bold text-[#c6d0df] block">
                          Minecraft Version
                        </label>
                        <label className="flex items-center gap-1 text-[10px] text-[#8fa0bc] cursor-pointer hover:text-white">
                          <input
                            type="checkbox"
                            checked={includeSnapshots}
                            onChange={(e) => {
                              setIncludeSnapshots(e.target.checked);
                              fetchMcVersions(e.target.checked);
                            }}
                            className="w-3 h-3 accent-[#12c3ca] rounded cursor-pointer"
                          />
                          <span>Snapshots</span>
                        </label>
                      </div>
                      <CustomDropdown
                        value={mcVersion}
                        onChange={(val) => setMcVersion(String(val))}
                        searchable={true}
                        placeholder="Select Minecraft Version..."
                        options={[
                          { value: "latest", label: "Latest Stable Release (Auto)" },
                          ...mcVersions.map((ver) => ({
                            value: ver,
                            label: `Minecraft ${ver}`,
                          })),
                        ]}
                      />
                    </div>

                    {loader !== "vanilla" ? (
                      <div className="space-y-1.5">
                        <label className="text-xs font-bold text-[#c6d0df] block">
                          Loader Version
                        </label>
                        <CustomDropdown
                          value={loaderVersion}
                          onChange={(val) => setLoaderVersion(String(val))}
                          searchable={loaderVersionsList.length > 8}
                          placeholder="Recommended / Latest (Auto)"
                          options={[
                            { value: "", label: "Recommended / Latest (Auto)" },
                            ...loaderVersionsList.map((item) => ({
                              value: item.version,
                              label: item.version,
                              recommended: item.recommended,
                            })),
                          ]}
                        />
                      </div>
                    ) : (
                      <div className="space-y-1.5 opacity-60">
                        <label className="text-xs font-bold text-[#c6d0df] block">
                          Loader Version
                        </label>
                        <input
                          type="text"
                          disabled
                          value="Not Applicable (Vanilla)"
                          className="w-full px-3 py-2.5 rounded-xl bg-[#090d14] border border-[#263042] text-xs text-[#8fa0bc] outline-none"
                        />
                      </div>
                    )}
                  </div>

                  {/* Pre-install Fabric Optimization + QoL Bundle toggle */}
                  {loader === "fabric" && (
                    <label className="flex items-start gap-3 p-3.5 rounded-xl bg-[#12c3ca]/10 border border-[#12c3ca]/30 cursor-pointer hover:border-[#12c3ca]/60 transition-colors">
                      <input
                        type="checkbox"
                        checked={includeDefaultFabricMods}
                        onChange={(e) => setIncludeDefaultFabricMods(e.target.checked)}
                        className="mt-0.5 accent-[#12c3ca] w-4 h-4 rounded cursor-pointer"
                      />
                      <div className="space-y-0.5">
                        <div className="flex items-center gap-1.5 text-xs font-extrabold text-[#5ce1e6]">
                          <Sparkles className="w-3.5 h-3.5 text-[#f59e0b]" />
                          <span>Pre-install Fabric Performance & QoL Bundle (12 Mods)</span>
                        </div>
                        <p className="text-[11px] text-[#8fa0bc] leading-relaxed">
                          Automatically resolves & installs Fabric API, Sodium, Lithium, FerriteCore,
                          ImmediatelyFast, EntityCulling, Mod Menu, Cloth Config, Jade, AppleSkin,
                          Xaero&apos;s Minimap, and Zoomify from Modrinth.
                        </p>
                      </div>
                    </label>
                  )}
                </>
              )}

              {/* Per-Profile Memory & JVM Overrides */}
              <div className="pt-2 border-t border-[#1e293b] space-y-3">
                <div className="flex items-center gap-2 text-xs font-extrabold uppercase tracking-wider text-[#8fa0bc]">
                  <Cpu className="w-3.5 h-3.5 text-[#12c3ca]" />
                  <span>Instance Resource Overrides</span>
                </div>

                {/* Custom RAM Toggle */}
                <div className="p-3 rounded-xl bg-[#090d14] border border-[#222d3f] space-y-2.5">
                  <label className="flex items-center justify-between cursor-pointer">
                    <span className="text-xs font-bold text-[#c6d0df]">
                      Override Global RAM Allocation ({defaultRamGb} GB Default)
                    </span>
                    <input
                      type="checkbox"
                      checked={useCustomRam}
                      onChange={(e) => setUseCustomRam(e.target.checked)}
                      className="accent-[#12c3ca] w-4 h-4 cursor-pointer"
                    />
                  </label>
                  {useCustomRam && (
                    <div className="space-y-1 pt-1">
                      <div className="flex justify-between text-xs font-mono">
                        <span className="text-[#8fa0bc]">Allocated Memory:</span>
                        <span className="text-[#5ce1e6] font-bold">{ramGb} GB</span>
                      </div>
                      <input
                        type="range"
                        min={2}
                        max={24}
                        step={1}
                        value={ramGb}
                        onChange={(e) => setRamGb(Number(e.target.value))}
                        className="w-full accent-[#12c3ca] cursor-pointer"
                      />
                    </div>
                  )}
                </div>

                {/* Custom JVM Flags Toggle */}
                <div className="p-3 rounded-xl bg-[#090d14] border border-[#222d3f] space-y-2.5">
                  <label className="flex items-center justify-between cursor-pointer">
                    <span className="text-xs font-bold text-[#c6d0df]">
                      Override Global JVM Arguments
                    </span>
                    <input
                      type="checkbox"
                      checked={useCustomFlags}
                      onChange={(e) => setUseCustomFlags(e.target.checked)}
                      className="accent-[#12c3ca] w-4 h-4 cursor-pointer"
                    />
                  </label>
                  {useCustomFlags && (
                    <input
                      type="text"
                      value={jvmFlags}
                      onChange={(e) => setJvmFlags(e.target.value)}
                      placeholder="-XX:+UseZGC -XX:+ZGenerational -XX:+AlwaysPreTouch"
                      className="w-full px-3 py-2 rounded-lg bg-[#0e131d] border border-[#263042] focus:border-[#12c3ca] text-xs font-mono text-[#e4e8ee] outline-none"
                    />
                  )}
                </div>

                {/* Java Runtime Specification */}
                <div className="p-3 rounded-xl bg-[#090d14] border border-[#222d3f] space-y-2.5">
                  <div className="flex items-center justify-between">
                    <label className="text-xs font-bold text-[#c6d0df] block">
                      Java Runtime Specification
                    </label>
                    {targetProfile?.id === "create-server" ? (
                      <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-500/15 text-amber-300 border border-amber-500/30">
                        Forge 1.20.1 (Java 17)
                      </span>
                    ) : (
                      <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-blue-500/15 text-blue-300 border border-blue-500/30">
                        Per-Profile Runtime
                      </span>
                    )}
                  </div>
                  <CustomDropdown
                    value={javaVersion ?? 0}
                    onChange={(val) => {
                      const num = Number(val);
                      setJavaVersion(num === 0 ? null : num);
                    }}
                    placeholder="Auto-detect Compatible Runtime (Recommended)"
                    options={[
                      {
                        value: 0,
                        label: "Auto-detect Compatible Runtime (Recommended)",
                        description: "Selects the optimal Adoptium Temurin OpenJDK runtime automatically.",
                      },
                      {
                        value: 17,
                        label: "Java 17 (Forge 1.20.1 & Minecraft 1.18 - 1.20.4)",
                        description: "Required for Forge 1.20.1. Automatically omits incompatible Generational ZGC.",
                        badge: "G1GC Optimized",
                      },
                      {
                        value: 21,
                        label: "Java 21 (Minecraft 1.20.5 - 1.21.x)",
                        description: "Modern LTS runtime with Generational ZGC support.",
                        badge: "ZGC Ready",
                      },
                      {
                        value: 25,
                        label: "Java 25 (Minecraft 26.x+ Snapshots)",
                        description: "Bleeding edge JDK runtime.",
                      },
                      {
                        value: 8,
                        label: "Java 8 (Legacy Minecraft 1.7 - 1.16.5)",
                        description: "Legacy runtime for classic modpacks.",
                      },
                    ]}
                  />
                  <p className="text-[11px] text-[#8fa0bc] leading-relaxed">
                    {javaVersion === 17 || (!javaVersion && (mcVersion === "1.20.1" || targetProfile?.id === "create-server"))
                      ? "Java 17 runtime selected: Optimized G1GC enabled. Incompatible Generational ZGC (-XX:+ZGenerational) is automatically omitted."
                      : (javaVersion === 21 || javaVersion === 25 || (!javaVersion && (mcVersion.startsWith("1.21") || mcVersion.startsWith("26."))))
                      ? "Modern Java runtime selected: Full Generational ZGC enabled for ultra-low latency."
                      : "Managed Adoptium / Azul OpenJDK runtime provisioned automatically for this instance."}
                  </p>
                </div>
              </div>
            </>
          )}

          {/* Actions */}
          <div className="pt-3 flex items-center justify-end gap-2.5 border-t border-[#1e293b]">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl bg-[#182232] hover:bg-[#222f43] text-xs font-bold text-[#8fa0bc] hover:text-white transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSaving || !name.trim()}
              className="px-5 py-2 rounded-xl bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 font-extrabold text-xs transition-all cursor-pointer shadow-lg shadow-cyan-950/50 disabled:opacity-50"
            >
              {isSaving
                ? "Saving..."
                : mode === "create"
                ? "Create Profile"
                : mode === "duplicate"
                ? "Duplicate Profile"
                : "Save Changes"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
