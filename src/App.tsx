import React, { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Header } from "./components/Header";
import { ServerCard } from "./components/ServerCard";
import { SettingsModal } from "./components/SettingsModal";
import { ChangelogDrawer } from "./components/ChangelogDrawer";
import { LoginModal } from "./components/LoginModal";
import { ProfileSidebar } from "./components/ProfileSidebar";
import { ProfileModal, ProfileModalMode } from "./components/ProfileModal";
import { ModsPanel } from "./components/ModsPanel";
import { ConsolePanel } from "./components/ConsolePanel";
import { ServerStatusPanel } from "./components/ServerStatusPanel";
import { CreateServerModal } from "./components/CreateServerModal";
import panoramaImg from "./assets/panorama.webp";
import {
  Play,
  Flame,
  Zap,
  Box,
  AlertCircle,
  FileText,
  CheckCircle2,
  FolderOpen,
  Sparkles,
  Square,
  Package,
  Sliders,
  HardDrive,
  Camera,
  RefreshCw,
  Server,
} from "lucide-react";
import {
  AppConfig,
  AuthStatus,
  MinecraftProfile,
  ProgressPayload,
  ServerStatus,
  DeviceCodeInfo,
  ClientUpdateInfo,
  ModpackUpdateInfo,
  ModpackProfile,
  ProfilesState,
  WorkspaceTab,
  ServerInstance,
} from "./types";

export const App: React.FC = () => {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [profilesState, setProfilesState] = useState<ProfilesState | null>(null);
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("play");

  const [profile, setProfile] = useState<MinecraftProfile | null>(null);
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);

  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [deviceInfo, setDeviceInfo] = useState<DeviceCodeInfo | null>(null);
  const [isLoginModalOpen, setIsLoginModalOpen] = useState(false);
  const pollingRef = useRef<number | null>(null);

  const [isSyncing, setIsSyncing] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [isRefreshingServer, setIsRefreshingServer] = useState(false);
  const [launchSuccess, setLaunchSuccess] = useState(false);

  const [clientUpdate, setClientUpdate] = useState<ClientUpdateInfo | null>(null);
  const [modpackUpdate, setModpackUpdate] = useState<ModpackUpdateInfo | null>(null);
  const [isCheckingUpdates, setIsCheckingUpdates] = useState(false);
  const [isUpdatingClient, setIsUpdatingClient] = useState(false);
  const [isForceClosing, setIsForceClosing] = useState(false);
  const [updateToast, setUpdateToast] = useState<string | null>(null);

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isChangelogOpen, setIsChangelogOpen] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Profile Modal state
  const [isProfileModalOpen, setIsProfileModalOpen] = useState(false);
  const [profileModalMode, setProfileModalMode] = useState<ProfileModalMode>("create");
  const [modalTargetProfile, setModalTargetProfile] = useState<ModpackProfile | null>(null);
  const [profileToDelete, setProfileToDelete] = useState<ModpackProfile | null>(null);

  // Dedicated Server State
  const [servers, setServers] = useState<ServerInstance[]>([]);
  const [isCreateServerModalOpen, setIsCreateServerModalOpen] = useState(false);

  const activeModpack: ModpackProfile | null =
    profilesState?.profiles.find((p) => p.id === profilesState.selected_profile_id) ||
    profilesState?.profiles[0] ||
    null;

  const isCreateServerProfile = !activeModpack || activeModpack.id === "create-server";
  const profileServers = servers.filter((s) => s.profile_id === activeModpack?.id);

  useEffect(() => {
    loadConfig();
    loadProfiles();
    loadServers();
    checkAuth();
    refreshServer();
    handleCheckAllUpdates(false);

    invoke<boolean>("get_game_status")
      .then((running) => setIsRunning(running))
      .catch(() => {});

    const unlistenProgressPromise = listen<ProgressPayload>("download-progress", (event) => {
      setProgress(event.payload);
    });

    const unlistenGamePromise = listen<boolean>("game-status", (event) => {
      setIsRunning(event.payload);
      if (event.payload) {
        setIsLaunching(false);
      }
    });

    const unlistenErrorPromise = listen<string>("game-error", (event) => {
      setErrorMsg(`Minecraft process stopped unexpectedly:\n${event.payload}`);
    });

    const authRefreshInterval = setInterval(() => {
      checkAuth();
    }, 30 * 60 * 1000);

    return () => {
      clearInterval(authRefreshInterval);
      if (pollingRef.current) clearInterval(pollingRef.current);
      unlistenProgressPromise.then((unlisten) => unlisten());
      unlistenGamePromise.then((unlisten) => unlisten());
      unlistenErrorPromise.then((unlisten) => unlisten());
    };
  }, []);

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      setConfig(cfg);
    } catch (err) {
      console.error("Failed to load config:", err);
    }
  };

  const loadProfiles = async () => {
    try {
      const state = await invoke<ProfilesState>("get_profiles");
      setProfilesState(state);
    } catch (err) {
      console.error("Failed to load profiles:", err);
    }
  };

  const loadServers = async () => {
    try {
      const list = await invoke<ServerInstance[]>("get_all_servers");
      setServers(list || []);
    } catch (err) {
      console.error("Failed to load servers:", err);
    }
  };

  const handleSelectProfile = async (profileId: string) => {
    try {
      const state = await invoke<ProfilesState>("select_profile", { profileId });
      setProfilesState(state);
      setErrorMsg(null);
    } catch (err: any) {
      setErrorMsg(`Failed to select profile: ${err}`);
    }
  };

  const handleSaveCreateProfile = async (data: {
    name: string;
    description: string;
    loader: string;
    loaderVersion?: string;
    mcVersion: string;
    includeDefaultFabricMods: boolean;
    ramGb: number | null;
    jvmFlags: string | null;
    javaVersion?: number | null;
  }) => {
    try {
      const state = await invoke<ProfilesState>("create_profile", {
        name: data.name,
        description: data.description,
        loader: data.loader,
        loaderVersion: data.loaderVersion || null,
        mcVersion: data.mcVersion,
        includeDefaultFabricMods: data.includeDefaultFabricMods,
        ramGb: data.ramGb,
        jvmFlags: data.jvmFlags,
        javaVersion: data.javaVersion || null,
      });
      setProfilesState(state);
      showToast(`Created profile "${data.name}"!`);
    } catch (err: any) {
      setErrorMsg(`Failed to create profile: ${err}`);
    }
  };

  const handleSaveEditProfile = async (data: {
    profileId: string;
    name: string;
    description: string;
    ramGb: number | null;
    jvmFlags: string | null;
    javaVersion?: number | null;
  }) => {
    try {
      const state = await invoke<ProfilesState>("update_profile", {
        profileId: data.profileId,
        name: data.name,
        description: data.description || null,
        ramGb: data.ramGb,
        jvmFlags: data.jvmFlags,
        javaVersion: data.javaVersion ?? null,
      });
      setProfilesState(state);
      showToast(`Updated profile "${data.name}".`);
    } catch (err: any) {
      setErrorMsg(`Failed to update profile: ${err}`);
    }
  };

  const handleSaveDuplicateProfile = async (sourceId: string, newName: string) => {
    try {
      const state = await invoke<ProfilesState>("duplicate_profile", {
        sourceId,
        newName,
      });
      setProfilesState(state);
      showToast(`Duplicated profile as "${newName}"!`);
    } catch (err: any) {
      setErrorMsg(`Failed to duplicate profile: ${err}`);
    }
  };

  const handleConfirmDeleteProfile = async () => {
    if (!profileToDelete) return;
    try {
      const deletedName = profileToDelete.name;
      const state = await invoke<ProfilesState>("delete_profile", {
        profileId: profileToDelete.id,
      });
      setProfilesState(state);
      setProfileToDelete(null);
      showToast(`Deleted profile "${deletedName}".`);
    } catch (err: any) {
      setErrorMsg(`Failed to delete profile: ${err}`);
      setProfileToDelete(null);
    }
  };

  const showToast = (msg: string) => {
    setUpdateToast(msg);
    setTimeout(() => setUpdateToast(null), 4000);
  };

  const checkAuth = async () => {
    try {
      const auth = await invoke<AuthStatus>("get_auth_status");
      setProfile(auth.profile);
    } catch (err) {
      console.error("Failed to check auth:", err);
    }
  };

  const refreshServer = async () => {
    setIsRefreshingServer(true);
    try {
      const status = await invoke<ServerStatus>("get_server_status");
      setServerStatus(status);
    } catch (err) {
      console.error("Failed to ping server:", err);
    } finally {
      setIsRefreshingServer(false);
    }
  };

  const handleLogin = async () => {
    setErrorMsg(null);
    setIsLoggingIn(true);
    try {
      const info = await invoke<DeviceCodeInfo>("start_device_login");
      setDeviceInfo(info);
      setIsLoginModalOpen(true);
      startPolling(info.device_code, info.interval || 5);
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsLoggingIn(false);
    }
  };

  const startPolling = (deviceCode: string, intervalSec: number) => {
    if (pollingRef.current) clearInterval(pollingRef.current);
    pollingRef.current = setInterval(async () => {
      try {
        const newProfile = await invoke<MinecraftProfile | null>("poll_device_login", { deviceCode });
        if (newProfile) {
          setProfile(newProfile);
          setIsLoginModalOpen(false);
          setDeviceInfo(null);
          if (pollingRef.current) clearInterval(pollingRef.current);
        }
      } catch (err: any) {
        setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
        setIsLoginModalOpen(false);
        if (pollingRef.current) clearInterval(pollingRef.current);
      }
    }, Math.max(3000, intervalSec * 1000));
  };

  const handleCloseLoginModal = () => {
    setIsLoginModalOpen(false);
    setDeviceInfo(null);
    if (pollingRef.current) clearInterval(pollingRef.current);
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      setProfile(null);
    } catch (err) {
      console.error("Logout failed:", err);
    }
  };

  const handleSaveConfig = async (updated: AppConfig) => {
    try {
      await invoke("save_config", { config: updated });
      setConfig(updated);
      await loadProfiles();
    } catch (err: any) {
      setErrorMsg(`Failed to save settings: ${err}`);
    }
  };

  const handleOpenFolder = async (path: string) => {
    try {
      await invoke("open_folder", { path });
    } catch (err) {
      console.error("Failed to open folder:", err);
    }
  };

  const handleOpenDiscord = async () => {
    try {
      await invoke("open_url", { url: "https://discord.gg/CFqskvhF8k" });
    } catch (err) {
      console.error("Failed to open Discord:", err);
    }
  };

  const handleCheckAllUpdates = async (manual = false) => {
    setIsCheckingUpdates(true);
    if (manual) setUpdateToast(null);

    let clientRes: ClientUpdateInfo | null = null;
    let modpackRes: ModpackUpdateInfo | null = null;

    try {
      clientRes = await invoke<ClientUpdateInfo>("check_client_update");
      setClientUpdate(clientRes);
    } catch (err) {
      console.warn("Client update check failed:", err);
    }

    try {
      modpackRes = await invoke<ModpackUpdateInfo>("check_modpack_update");
      setModpackUpdate(modpackRes);
    } catch (err) {
      console.warn("Modpack update check failed:", err);
    }

    setIsCheckingUpdates(false);

    if (manual) {
      if (clientRes?.has_update && modpackRes?.has_update) {
        setUpdateToast(`Updates available: Client (${clientRes.latest_version}) & Modpack (${modpackRes.latest_version})`);
      } else if (clientRes?.has_update) {
        setUpdateToast(`Client update available (${clientRes.latest_version})! Click Update Available above.`);
      } else if (modpackRes?.has_update) {
        setUpdateToast(`Modpack update available (${modpackRes.latest_version})! Click Update Available below.`);
      } else {
        const cVer = clientRes?.current_version || "1.0.9";
        const mVer = modpackRes?.current_version || "1.0.13";
        showToast(`Everything is up to date! (Client v${cVer} • Create Server v${mVer})`);
      }
    }
  };

  const handlePerformClientUpdate = async () => {
    if (!clientUpdate?.download_url) {
      if (clientUpdate?.release_url) {
        await invoke("open_url", { url: clientUpdate.release_url });
      }
      return;
    }
    setIsUpdatingClient(true);
    setUpdateToast(`Downloading Create Client ${clientUpdate.latest_version}...`);
    try {
      const res = await invoke<string>("perform_client_update", {
        downloadUrl: clientUpdate.download_url,
      });
      setUpdateToast(res);
    } catch (err: any) {
      setUpdateToast(`Update failed: ${err}`);
      setTimeout(() => setUpdateToast(null), 5000);
    } finally {
      setIsUpdatingClient(false);
    }
  };

  const handleDeleteAllFiles = async () => {
    try {
      await invoke("delete_all_game_data");
      setModpackUpdate({
        has_update: true,
        current_version: "Not Installed",
        latest_version: modpackUpdate?.latest_version || "1.0.13",
        changelog: null,
      });
      showToast("All Create Server game files and mods have been completely deleted.");
    } catch (err: any) {
      setErrorMsg(`Failed to delete game files: ${err}`);
    }
  };

  const handleResetDefaults = async () => {
    try {
      await invoke("reset_modpack_settings");
      showToast("Modpack settings (options.txt) restored to official defaults.");
    } catch (err: any) {
      setErrorMsg(`Failed to reset settings: ${err}`);
    }
  };

  const handleSyncActiveProfile = async (forceRepair = false) => {
    if (!activeModpack) return;
    setErrorMsg(null);
    setIsSyncing(true);
    try {
      const res = await invoke<string>("sync_profile", {
        profileId: activeModpack.id,
        forceRepair,
      });
      if (activeModpack.id === "create-server") {
        setModpackUpdate((prev) =>
          prev ? { ...prev, has_update: false, current_version: res || prev.latest_version } : null
        );
      }
      await loadProfiles();
      showToast(`Synced ${activeModpack.name} (${res})`);
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setIsSyncing(false);
      setTimeout(() => setProgress(null), 2000);
    }
  };

  const handlePlay = async () => {
    setErrorMsg(null);

    if (!profile) {
      handleLogin();
      return;
    }

    const targetId = activeModpack?.id || "create-server";
    setIsSyncing(true);
    try {
      await invoke("sync_profile", { profileId: targetId, forceRepair: false });
      if (targetId === "create-server") {
        setModpackUpdate((prev) =>
          prev ? { ...prev, has_update: false, current_version: prev.latest_version } : null
        );
      }
      await loadProfiles();

      setIsSyncing(false);
      setIsLaunching(true);
      await invoke("launch_profile", { profileId: targetId });
      checkAuth();

      setIsRunning(true);
      setIsLaunching(false);
      setLaunchSuccess(true);
      setTimeout(() => {
        setLaunchSuccess(false);
        setProgress(null);
      }, 3000);
    } catch (err: any) {
      setIsSyncing(false);
      setIsLaunching(false);
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    }
  };

  const handleForceClose = async () => {
    setIsForceClosing(true);
    try {
      await invoke("force_close_game");
      setIsRunning(false);
      showToast("Minecraft was force closed.");
    } catch (err: any) {
      setErrorMsg(`Failed to force close: ${err}`);
    } finally {
      setIsForceClosing(false);
    }
  };

  const handleForceRepair = async () => {
    setIsSettingsOpen(false);
    await handleSyncActiveProfile(true);
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 MB";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  const formatSpeed = (bytesSec: number) => {
    if (bytesSec === 0) return "";
    const mbSec = bytesSec / (1024 * 1024);
    return `• ${mbSec.toFixed(1)} MB/s`;
  };

  const activeInstanceDir = activeModpack?.instance_dir || config?.game_dir || "";
  const effectiveRam = activeModpack?.ram_gb ?? config?.ram_gb ?? 6;

  return (
    <div className="flex flex-col h-screen w-screen bg-[#0a0d13] text-[#e4e8ee] overflow-hidden font-sans select-none relative">
      {/* Seamless Faded Minecraft Panoramic Background Layer */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none select-none z-0">
        <div className="flex h-full w-max animate-panorama-scroll will-change-transform opacity-25">
          <img
            src={panoramaImg}
            alt=""
            className="h-full w-auto max-w-none object-cover"
            draggable={false}
          />
          <img
            src={panoramaImg}
            alt=""
            className="h-full w-auto max-w-none object-cover"
            draggable={false}
          />
        </div>
        {/* Transparent dark atmospheric theme tint */}
        <div className="absolute inset-0 bg-radial from-[#12c3ca]/5 via-[#0a0d13]/85 to-[#0a0d13]" />
      </div>

      {/* Top Header */}
      <Header
        profile={profile}
        activeModpack={activeModpack}
        activeTab={activeTab}
        serverCount={profileServers.length}
        onChangeTab={setActiveTab}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onLogin={handleLogin}
        onLogout={handleLogout}
        isLoggingIn={isLoggingIn}
        clientUpdate={clientUpdate}
        onCheckUpdates={() => handleCheckAllUpdates(true)}
        isCheckingUpdates={isCheckingUpdates}
        onUpdateClient={handlePerformClientUpdate}
        isUpdatingClient={isUpdatingClient}
      />

      {/* Main Split Workspace: Left Profiles Rail + Right Active Workspace */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        <ProfileSidebar
          profiles={profilesState?.profiles || []}
          selectedProfileId={profilesState?.selected_profile_id || "create-server"}
          progress={progress}
          onSelectProfile={handleSelectProfile}
          onCreateProfile={() => {
            setModalTargetProfile(null);
            setProfileModalMode("create");
            setIsProfileModalOpen(true);
          }}
          onEditProfile={(p) => {
            setModalTargetProfile(p);
            setProfileModalMode("edit");
            setIsProfileModalOpen(true);
          }}
          onDuplicateProfile={(p) => {
            setModalTargetProfile(p);
            setProfileModalMode("duplicate");
            setIsProfileModalOpen(true);
          }}
          onDeleteProfile={(p) => setProfileToDelete(p)}
          onOpenFolder={handleOpenFolder}
          disabled={isSyncing || isLaunching}
        />

        {/* Right Workspace Content */}
        <main className="flex-1 relative z-10 flex flex-col justify-between p-6 overflow-hidden min-w-0">
          {/* Main Atmosphere Overlay */}
          <div className="absolute inset-0 bg-radial from-[#12c3ca]/5 via-[#0e121a]/80 to-[#0a0d13]/90 pointer-events-none" />
          <div className="absolute top-0 right-0 w-[550px] h-[550px] bg-[#12c3ca]/5 rounded-full blur-3xl pointer-events-none" />
          <div className="absolute bottom-0 left-0 w-[450px] h-[450px] bg-[#f59e0b]/5 rounded-full blur-3xl pointer-events-none" />

          {/* Active Workspace Tab Body */}
          {activeTab === "mods" && activeModpack ? (
            <div className="flex-1 min-h-0 flex flex-col">
              <ModsPanel
                profile={activeModpack}
                curseforgeApiKey={config?.curseforge_api_key}
                onOpenSettings={() => setIsSettingsOpen(true)}
                onOpenFolder={handleOpenFolder}
                onSyncProfile={() => handleSyncActiveProfile(false)}
                isSyncing={isSyncing}
                onNotify={showToast}
                onError={(m) => setErrorMsg(m)}
              />
            </div>
          ) : activeTab === "servers" && activeModpack ? (
            <div className="flex-1 min-h-0 flex flex-col">
              <ServerStatusPanel
                profile={activeModpack}
                servers={servers}
                onRefreshServers={loadServers}
                onCreateServer={() => setIsCreateServerModalOpen(true)}
                onOpenFolder={handleOpenFolder}
                onNotify={showToast}
                onError={(m) => setErrorMsg(m)}
              />
            </div>
          ) : activeTab === "console" && activeModpack ? (
            <div className="flex-1 min-h-0 flex flex-col">
              <ConsolePanel
                profile={activeModpack}
                isRunning={isRunning}
                onForceClose={handleForceClose}
                isForceClosing={isForceClosing}
                onOpenFolder={handleOpenFolder}
              />
            </div>
          ) : (
            /* PLAY / DASHBOARD VIEW */
            <>
              {/* Top Floating Row: Active Profile Hero & Server Card (or Instance Summary Card) */}
              <div className="relative z-10 flex items-start justify-between gap-6">
                <div className="space-y-2.5 max-w-xl">
                  <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-[#12c3ca]/10 border border-[#12c3ca]/40 text-[#5ce1e6] text-xs font-bold font-mono">
                    {isCreateServerProfile ? (
                      <>
                        <Flame className="w-3.5 h-3.5 text-[#f59e0b]" />
                        <span>OFFICIAL SERVER MODPACK</span>
                      </>
                    ) : activeModpack?.loader === "fabric" ? (
                      <>
                        <Zap className="w-3.5 h-3.5 text-[#12c3ca]" />
                        <span>
                          FABRIC INSTANCE •{" "}
                          {activeModpack.mc_version === "latest"
                            ? `LATEST STABLE (${activeModpack.resolved_mc_version || "AUTO"})`
                            : `MC ${activeModpack.mc_version}`}
                        </span>
                      </>
                    ) : (
                      <>
                        <Box className="w-3.5 h-3.5 text-emerald-400" />
                        <span>VANILLA INSTANCE • MC {activeModpack?.mc_version}</span>
                      </>
                    )}
                  </div>

                  <h2 className="text-3xl font-extrabold tracking-tight drop-shadow-md">
                    {isCreateServerProfile ? (
                      <>
                        <span className="text-[#f59e0b]">Create </span>
                        <span className="text-[#e4e8ee]">Server</span>
                      </>
                    ) : (
                      <span className="text-[#e4e8ee]">{activeModpack?.name}</span>
                    )}
                  </h2>

                  <p className="text-xs text-[#9fb0ca] leading-relaxed">
                    {activeModpack?.description}
                  </p>

                  {/* Instance Pills & Quick Actions */}
                  <div className="pt-1 flex items-center gap-2 flex-wrap">
                    <span className="px-2.5 py-1 rounded-lg bg-[#141c2b] border border-[#263449] text-[11px] font-mono text-[#5ce1e6]">
                      RAM: {effectiveRam} GB
                    </span>
                    {activeModpack?.loader_version && (
                      <span className="px-2.5 py-1 rounded-lg bg-[#141c2b] border border-[#263449] text-[11px] font-mono text-[#8fa0bc]">
                        Loader: {activeModpack.loader_version}
                      </span>
                    )}
                    {!isCreateServerProfile && activeModpack && (
                      <span className="px-2.5 py-1 rounded-lg bg-[#141c2b] border border-[#263449] text-[11px] font-mono text-amber-300">
                        {activeModpack.modrinth_slugs.length} Curated Modrinth Mods
                      </span>
                    )}

                    <button
                      onClick={() => setActiveTab("mods")}
                      className="px-2.5 py-1 rounded-lg bg-[#12c3ca]/15 hover:bg-[#12c3ca]/25 border border-[#12c3ca]/40 text-[11px] font-bold text-[#5ce1e6] flex items-center gap-1.5 transition-colors cursor-pointer"
                    >
                      <Package className="w-3.5 h-3.5" />
                      <span>Manage Content</span>
                    </button>

                    <button
                      onClick={() => setIsCreateServerModalOpen(true)}
                      className="px-2.5 py-1 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/40 text-[11px] font-bold text-emerald-300 flex items-center gap-1.5 transition-colors cursor-pointer"
                    >
                      <Server className="w-3.5 h-3.5" />
                      <span>Create Server (Beta)</span>
                    </button>

                    {profileServers.length > 0 && (
                      <button
                        onClick={() => setActiveTab("servers")}
                        className="px-2.5 py-1 rounded-lg bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[11px] font-bold text-[#5ce1e6] flex items-center gap-1.5 transition-colors cursor-pointer"
                      >
                        <Server className="w-3.5 h-3.5" />
                        <span>Servers ({profileServers.length})</span>
                      </button>
                    )}

                    {activeModpack && (
                      <button
                        onClick={() => {
                          setModalTargetProfile(activeModpack);
                          setProfileModalMode("edit");
                          setIsProfileModalOpen(true);
                        }}
                        className="px-2.5 py-1 rounded-lg bg-[#182232] hover:bg-[#222f43] border border-[#2a384c] text-[11px] font-bold text-[#c6d0df] hover:text-white flex items-center gap-1.5 transition-colors cursor-pointer"
                      >
                        <Sliders className="w-3.5 h-3.5 text-[#12c3ca]" />
                        <span>Profile Overrides</span>
                      </button>
                    )}
                  </div>
                </div>

                {isCreateServerProfile ? (
                  <ServerCard
                    status={serverStatus}
                    onRefresh={refreshServer}
                    isLoading={isRefreshingServer}
                  />
                ) : (
                  <div className="bg-[#0e121af2] border border-[#263042] rounded-xl overflow-hidden shadow-xl backdrop-blur-md min-w-[300px]">
                    <div className="bg-[#182232] px-3.5 py-2 border-b border-[#263042] flex items-center justify-between">
                      <div className="flex items-center gap-1.5 font-mono text-xs">
                        <span className="font-bold text-[#12c3ca]">INSTANCE</span>
                        <span className="text-[#3a465a]">|</span>
                        <span className="font-semibold text-[#e4e8ee] truncate max-w-[150px]">
                          {activeModpack?.name}
                        </span>
                      </div>
                      <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-[#12c3ca]/15 text-[#5ce1e6] border border-[#12c3ca]/30">
                        {activeModpack?.loader || "vanilla"}
                      </span>
                    </div>

                    <div className="p-3.5 space-y-2 text-xs">
                      <div className="flex items-center justify-between text-[#8fa0bc]">
                        <span className="text-[11px]">Minecraft:</span>
                        <span className="font-mono text-[#e4e8ee] font-medium">
                          {activeModpack?.mc_version === "latest"
                            ? `Latest (${activeModpack.resolved_mc_version || "Auto"})`
                            : activeModpack?.mc_version}
                        </span>
                      </div>

                      <div className="flex items-center justify-between text-[#8fa0bc]">
                        <span className="text-[11px]">Allocated RAM:</span>
                        <span className="font-mono text-[#5ce1e6] font-medium">
                          {effectiveRam} GB
                        </span>
                      </div>

                      <div className="flex items-center justify-between text-[#8fa0bc]">
                        <span className="text-[11px]">Installed Content:</span>
                        <span className="font-mono text-amber-300 font-medium">
                          {activeModpack?.modrinth_slugs?.length ?? 0} Mods
                        </span>
                      </div>

                      {profileServers.length > 0 && (
                        <div className="flex items-center justify-between pt-1 border-t border-[#1c2432] text-[#8fa0bc]">
                          <span className="text-[11px]">Dedicated Servers:</span>
                          <span className="font-mono text-emerald-400 font-medium">
                            {profileServers.length} Instance{profileServers.length > 1 ? "s" : ""}
                          </span>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>

              {/* Bottom Section: Progress Bar & Big Play Button */}
              <div className="relative z-10 space-y-4 pt-6">
                {/* Progress / Download Status Bar */}
                {(isSyncing || progress) && (
                  <div className="bg-[#0e121af5] border border-[#263042] rounded-xl p-3.5 backdrop-blur-md shadow-xl space-y-2">
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-bold text-[#12c3ca]">
                        {progress?.task || `Syncing ${activeModpack?.name || "modpack"}...`}
                      </span>
                      <span className="font-mono text-[#8fa0bc]">
                        {progress && progress.total_bytes > 0
                          ? `${formatBytes(progress.current_bytes)} / ${formatBytes(
                              progress.total_bytes
                            )} (${progress.percentage.toFixed(0)}%) ${formatSpeed(
                              progress.speed_bytes_sec
                            )}`
                          : ""}
                      </span>
                    </div>
                    <div className="w-full h-2.5 bg-[#182232] rounded-full overflow-hidden border border-[#263042]">
                      <div
                        className="h-full bg-gradient-to-r from-[#0d949b] via-[#12c3ca] to-[#5ce1e6] transition-all duration-150 rounded-full shadow-[0_0_12px_rgba(18,195,202,0.6)]"
                        style={{
                          width: `${Math.min(100, Math.max(5, progress?.percentage ?? 10))}%`,
                        }}
                      />
                    </div>
                  </div>
                )}

                {/* Controls Bar */}
                <div className="flex items-center justify-between gap-4 flex-wrap">
                  {/* Left Controls: Instance Quick Folders & Server Update Log */}
                  <div className="flex items-center gap-2 flex-wrap">
                    <button
                      onClick={() => handleOpenFolder(activeInstanceDir)}
                      className="px-3.5 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
                      title="Open Profile Instance Directory"
                    >
                      <FolderOpen className="w-4 h-4 text-[#f59e0b]" />
                      <span>Instance Folder</span>
                    </button>

                    <button
                      onClick={() => handleOpenFolder(`${activeInstanceDir}/saves`)}
                      className="px-3.5 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
                      title="Open Worlds / Saves Folder"
                    >
                      <HardDrive className="w-4 h-4 text-[#5ce1e6]" />
                      <span>Saves</span>
                    </button>

                    <button
                      onClick={() => handleOpenFolder(`${activeInstanceDir}/screenshots`)}
                      className="px-3.5 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
                      title="Open Screenshots Folder"
                    >
                      <Camera className="w-4 h-4 text-emerald-400" />
                      <span>Screenshots</span>
                    </button>

                    {isCreateServerProfile ? (
                      <button
                        onClick={() => setIsChangelogOpen(true)}
                        className="px-3.5 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md"
                      >
                        <FileText className="w-4 h-4 text-[#12c3ca]" />
                        <span>Update Log</span>
                      </button>
                    ) : (
                      <button
                        onClick={() => handleSyncActiveProfile(false)}
                        disabled={isSyncing || isLaunching}
                        className="px-3.5 py-3 rounded-xl bg-[#182232] hover:bg-[#242e3e] border border-[#263042] hover:border-[#12c3ca]/60 text-[#e4e8ee] hover:text-[#12c3ca] flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-md disabled:opacity-50"
                        title="Sync & Update Latest Compatible MC + Loader + Modrinth Mods"
                      >
                        <RefreshCw
                          className={`w-4 h-4 text-[#12c3ca] ${
                            isSyncing ? "animate-spin" : ""
                          }`}
                        />
                        <span>Sync Instance</span>
                      </button>
                    )}
                  </div>

                  {/* Right Controls: Force Close, Play Button & Discord */}
                  <div className="flex items-center gap-3">
                    {isRunning && (
                      <button
                        onClick={handleForceClose}
                        disabled={isForceClosing}
                        className="h-14 px-6 rounded-2xl bg-rose-950/80 hover:bg-rose-600 border border-rose-500/60 hover:border-rose-400 text-rose-200 hover:text-white flex items-center gap-2.5 text-xs font-black uppercase tracking-wider transition-all duration-200 cursor-pointer shadow-xl shadow-rose-950/50 hover:scale-[1.02] active:scale-[0.98]"
                        title="Force Close Minecraft"
                      >
                        <Square className="w-4 h-4 fill-current text-rose-400" />
                        <span>{isForceClosing ? "Closing..." : "Force Close"}</span>
                      </button>
                    )}

                    {/* Launch / Play / Running Button */}
                    <button
                      onClick={handlePlay}
                      disabled={isSyncing || isLaunching || isRunning}
                      className={`h-14 px-9 rounded-2xl font-black text-sm tracking-wider uppercase transition-all duration-200 flex items-center justify-center gap-3 shadow-xl ${
                        isRunning
                          ? "bg-emerald-950/80 border border-emerald-500/60 text-emerald-300 cursor-not-allowed shadow-emerald-950/50"
                          : launchSuccess
                          ? "bg-emerald-600 text-white shadow-emerald-950/60"
                          : isSyncing || isLaunching
                          ? "bg-[#182232] text-[#8fa0bc] border border-[#263042] cursor-not-allowed opacity-80"
                          : !profile
                          ? "bg-gradient-to-r from-[#0d949b] to-[#12c3ca] hover:from-[#12c3ca] hover:to-[#5ce1e6] text-slate-950 shadow-cyan-950/60 hover:scale-[1.02] cursor-pointer"
                          : isCreateServerProfile && modpackUpdate?.has_update
                          ? "bg-gradient-to-r from-amber-400 via-amber-300 to-amber-400 hover:brightness-110 text-slate-950 shadow-amber-950/80 hover:scale-[1.02] active:scale-[0.98] cursor-pointer animate-pulse"
                          : "bg-gradient-to-r from-[#0d949b] via-[#12c3ca] to-[#5ce1e6] hover:brightness-110 text-slate-950 shadow-cyan-950/70 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
                      }`}
                    >
                      {isRunning ? (
                        <>
                          <span className="w-3 h-3 rounded-full bg-emerald-400 animate-pulse shadow-[0_0_8px_#34d399]" />
                          <span>Running</span>
                        </>
                      ) : launchSuccess ? (
                        <>
                          <CheckCircle2 className="w-5 h-5 text-white animate-bounce" />
                          <span>Minecraft Launched!</span>
                        </>
                      ) : isLaunching ? (
                        <>
                          <div className="w-5 h-5 border-2 border-slate-900 border-t-white rounded-full animate-spin" />
                          <span>Launching Game...</span>
                        </>
                      ) : isSyncing ? (
                        <>
                          <div className="w-5 h-5 border-2 border-slate-900 border-t-white rounded-full animate-spin" />
                          <span>Syncing Profile...</span>
                        </>
                      ) : !profile ? (
                        <>
                          <Play className="w-5 h-5 fill-current" />
                          <span>Sign In & Play</span>
                        </>
                      ) : isCreateServerProfile && modpackUpdate?.has_update ? (
                        <>
                          <Sparkles className="w-6 h-6 fill-current text-slate-950 animate-bounce" />
                          <span>Update Available</span>
                        </>
                      ) : (
                        <>
                          <Play className="w-6 h-6 fill-current" />
                          <span>
                            {isCreateServerProfile ? "Play Create Server" : "Play Profile"}
                          </span>
                        </>
                      )}
                    </button>

                    {/* Discord Link Button */}
                    <button
                      onClick={handleOpenDiscord}
                      className="h-14 px-5 rounded-2xl bg-[#5865F2]/20 hover:bg-[#5865F2]/35 border border-[#5865F2]/40 hover:border-[#5865F2] text-[#9ba5fe] hover:text-white flex items-center gap-2 text-xs font-bold transition-all cursor-pointer shadow-lg hover:scale-[1.02] active:scale-[0.98]"
                      title="Join Create Server Discord"
                    >
                      <svg className="w-5 h-5 fill-current" viewBox="0 0 24 24">
                        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.893.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                      </svg>
                      <span>Discord</span>
                    </button>
                  </div>
                </div>
              </div>
            </>
          )}
        </main>
      </div>

      {/* Create / Edit / Duplicate Profile Modal */}
      <ProfileModal
        isOpen={isProfileModalOpen}
        mode={profileModalMode}
        targetProfile={modalTargetProfile}
        defaultRamGb={config?.ram_gb || 6}
        defaultJvmFlags={config?.jvm_flags || ""}
        onClose={() => setIsProfileModalOpen(false)}
        onSaveCreate={handleSaveCreateProfile}
        onSaveEdit={handleSaveEditProfile}
        onSaveDuplicate={handleSaveDuplicateProfile}
      />

      {/* Pre-installed Server Creator Modal (Beta) */}
      {isCreateServerModalOpen && activeModpack && (
        <CreateServerModal
          isOpen={isCreateServerModalOpen}
          profile={activeModpack}
          onClose={() => setIsCreateServerModalOpen(false)}
          onServerCreated={(newServer) => {
            setServers((prev) => [...prev, newServer]);
            showToast(`Created server "${newServer.name}"!`);
            setActiveTab("servers");
          }}
          onError={(m) => setErrorMsg(m)}
        />
      )}

      {/* Delete Profile Confirmation Dialog */}
      {profileToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
          <div className="w-full max-w-md bg-[#0e131d] border border-rose-500/40 rounded-2xl p-6 space-y-4 shadow-2xl">
            <h3 className="text-sm font-extrabold text-rose-300 uppercase tracking-wide">
              Delete Modpack Profile?
            </h3>
            <p className="text-xs text-[#c6d0df] leading-relaxed">
              Are you sure you want to permanently delete{" "}
              <span className="font-bold text-white">{profileToDelete.name}</span> and its
              instance folder? Other profiles will not be affected.
            </p>
            <div className="flex justify-end gap-2.5 pt-2">
              <button
                onClick={() => setProfileToDelete(null)}
                className="px-4 py-2 rounded-xl bg-[#182232] hover:bg-[#222f43] text-xs font-bold text-[#8fa0bc] hover:text-white cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmDeleteProfile}
                className="px-4 py-2 rounded-xl bg-rose-600 hover:bg-rose-500 text-xs font-extrabold text-white cursor-pointer shadow-lg"
              >
                Delete Profile
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Global Settings Modal */}
      {config && (
        <SettingsModal
          isOpen={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
          config={config}
          onSaveConfig={handleSaveConfig}
          onOpenFolder={handleOpenFolder}
          onForceRepair={handleForceRepair}
          onDeleteAllFiles={handleDeleteAllFiles}
          onResetDefaults={handleResetDefaults}
        />
      )}

      {/* Changelog Drawer */}
      <ChangelogDrawer
        isOpen={isChangelogOpen}
        onClose={() => setIsChangelogOpen(false)}
        version={
          modpackUpdate?.current_version && modpackUpdate.current_version !== "Not Installed"
            ? `v${modpackUpdate.current_version.replace(/^v/, "")}`
            : modpackUpdate?.latest_version && modpackUpdate.latest_version !== "Unknown"
            ? `v${modpackUpdate.latest_version.replace(/^v/, "")}`
            : "v1.0.13"
        }
      />

      {/* Device Code Login Modal */}
      <LoginModal
        isOpen={isLoginModalOpen}
        onClose={handleCloseLoginModal}
        deviceInfo={deviceInfo}
      />

      {/* Floating Toast / Notification Banners (Non-intrusive, Bottom-Right) */}
      {(updateToast || errorMsg) && (
        <div className="fixed bottom-6 right-6 z-50 flex flex-col gap-2.5 max-w-sm pointer-events-auto">
          {updateToast && (
            <div className="p-3.5 rounded-xl bg-[#0e121af8] border border-[#12c3ca]/60 text-[#5ce1e6] text-xs flex items-center justify-between gap-3 shadow-2xl backdrop-blur-md animate-in slide-in-from-bottom-2 fade-in duration-200">
              <div className="flex items-center gap-2 min-w-0">
                <CheckCircle2 className="w-4 h-4 text-[#12c3ca] shrink-0" />
                <span className="truncate">{updateToast}</span>
              </div>
              <button
                onClick={() => setUpdateToast(null)}
                className="text-[#8fa0bc] hover:text-[#e4e8ee] text-xs font-bold px-2 py-0.5 rounded hover:bg-white/5 cursor-pointer shrink-0"
              >
                Dismiss
              </button>
            </div>
          )}

          {errorMsg && (
            <div className="p-3.5 rounded-xl bg-red-950/95 border border-red-500/60 text-red-200 text-xs flex items-center justify-between gap-3 shadow-2xl backdrop-blur-md animate-in slide-in-from-bottom-2 fade-in duration-200">
              <div className="flex items-center gap-2.5 min-w-0">
                <AlertCircle className="w-4 h-4 text-red-400 shrink-0" />
                <span className="line-clamp-2">{errorMsg}</span>
              </div>
              <button
                onClick={() => setErrorMsg(null)}
                className="text-red-400 hover:text-red-200 text-xs font-bold px-2 py-0.5 rounded hover:bg-white/5 cursor-pointer shrink-0"
              >
                Dismiss
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default App;
