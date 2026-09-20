import NexusUpdateButton from "./NexusUpdateButton";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  useEffect,
  useMemo,
  useState,
} from "react";

import {
  invoke,
} from "@tauri-apps/api/core";

import {
  listen,
} from "@tauri-apps/api/event";

import {
  getCurrentWindow,
} from "@tauri-apps/api/window";

import appIcon from "./assets/app-icon.png";

import "./App.css";

import GameSettingsPage from "./GameSettingsPage";
import BrowseModsPage from "./BrowseModsPage";
import DownloadsPage from "./DownloadsPage";
import DownloadProgress from "./DownloadProgress";
import {
  initializeDownloadStore,
} from "./downloadStore";
type EldenRingInstallation = {
  found: boolean;
  steam_path: string | null;
  game_path: string | null;
  executable_path: string | null;
  proton_prefix: string | null;
};

type EngineKind =
| "auto"
| "me3"
| "modengine2";

type Profile = {
  id: string;
  name: string;
  engine: EngineKind;
};

type ProfileConfig = {
  selected_profile: string;
  profiles: Profile[];
};

type PlatformInfo = {
  os: string;
  display_name: string;
  uses_proton: boolean;
  supported: boolean;
};

type EngineStatus = {
  id: string;
  name: string;
  description: string;
  installed: boolean;
  preferred: boolean;
  engine_path: string;
  executable_path: string | null;
  installed_version: string | null;
  latest_version: string | null;
  update_available: boolean;
};

type EngineOverview = {
  preferred_engine: string;
  me3: EngineStatus;
  modengine2: EngineStatus;
};

type EngineDecision = {
  requested: EngineKind;
  selected: EngineKind;
  reason: string;
};

type ModContentType =
| "assets"
| "native_dll"
| "config"
| "unknown";

type CompatibilityLevel =
| "universal"
| "compatibility_required"
| "mod_engine2_required"
| "unknown";

type ModContent = {
  content_type: ModContentType;
  relative_path: string;
};

type InstalledMod = {
  id: string;
  name: string;
  version: string | null;
  enabled: boolean;
  compatibility: CompatibilityLevel;
  contents: ModContent[];
};

type ModArchiveVariant = {
  id: string;
  name: string;
  relative_path: string;
};

type ModArchiveAnalysis = {
  archive_path: string;
  display_name: string;
  variants: ModArchiveVariant[];
};

type ModpackImportFailure = {
  name: string;
  reason: string;
};

type ModpackImportResult = {
  profile_id: string;
  profile_name: string;
  installed_count: number;
  skipped_manual: string[];
  failures: ModpackImportFailure[];
};

type NexusDownloadProgressEvent = {
  mod_id: number;
  file_id: number;
  file_name: string;
  downloaded_bytes: number;
  total_bytes: number | null;
  percent: number | null;
  state: string;
};

type ConflictType =
| "asset"
| "native";

type ModConflict = {
  conflict_type: ConflictType;
  relative_path: string;
  mod_ids: string[];
  mod_names: string[];
  winner_mod_id: string | null;
  winner_mod_name: string | null;
  overridden_mod_ids: string[];
  overridden_mod_names: string[];
};

type ConflictReport = {
  profile_id: string;
  conflict_count: number;
  conflicts: ModConflict[];
};

type Page =
| "dashboard"
| "browse"
| "mods"
| "game_settings"
| "profiles"
| "downloads"
| "settings";

type AppSettings = {
  confirmSingleUninstall: boolean;
  confirmBulkUninstall: boolean;
  expandModsByDefault: boolean;
  reducedEffects: boolean;
  showInternalPaths: boolean;
  showRuntimeDetails: boolean;
  verboseConflicts: boolean;
};

const defaultAppSettings: AppSettings = {
  confirmSingleUninstall: true,
  confirmBulkUninstall: true,
  expandModsByDefault: false,
  reducedEffects: true,
  showInternalPaths: false,
  showRuntimeDetails: false,
  verboseConflicts: false,
};

const settingsStorageKey =
  "elden-mod-manager.settings.v1";

function loadAppSettings(): AppSettings {
  try {
    const stored =
      localStorage.getItem(
        settingsStorageKey,
      );

    if (!stored) {
      return defaultAppSettings;
    }

    return {
      ...defaultAppSettings,
      ...JSON.parse(stored),
    };
  } catch {
    return defaultAppSettings;
  }
}

const emptyProfiles: ProfileConfig = {
  selected_profile: "default",
  profiles: [],
};

function engineDisplayName(
  engine: EngineKind,
) {
  if (engine === "me3") {
    return "me3";
  }

  if (engine === "modengine2") {
    return "Mod Engine 2";
  }

  return "Automatic";
}

function shortPath(
  path: string | null,
) {
  if (!path) {
    return "Not detected";
  }

  if (path.startsWith("/home/")) {
    const parts =
    path.split("/");

    if (parts.length > 3) {
      return `~/${parts
        .slice(3)
        .join("/")}`;
    }
  }

  return path;
}

function App() {
  const [
    page,
    setPage,
  ] =
  useState<Page>(
    "dashboard",
  );

  const [
    installation,
    setInstallation,
  ] =
  useState<EldenRingInstallation | null>(
    null,
  );

  const [
    profiles,
    setProfiles,
  ] =
  useState<ProfileConfig>(
    emptyProfiles,
  );

  const [
    platform,
    setPlatform,
  ] =
  useState<PlatformInfo | null>(
    null,
  );

  const [
    engines,
    setEngines,
  ] =
  useState<EngineOverview | null>(
    null,
  );

  const [
    engineDecision,
    setEngineDecision,
  ] =
  useState<EngineDecision | null>(
    null,
  );

  const [
    installedMods,
    setInstalledMods,
  ] =
  useState<InstalledMod[]>(
    [],
  );

  const [
    conflictReport,
    setConflictReport,
  ] =
  useState<ConflictReport | null>(
    null,
  );

  const [
    pendingArchive,
    setPendingArchive,
  ] =
  useState<ModArchiveAnalysis | null>(
    null,
  );

  const [
    selectedVariantId,
    setSelectedVariantId,
  ] =
  useState<string | null>(
    null,
  );

  const [
    loading,
    setLoading,
  ] =
  useState(true);

  const [
    loadingMods,
    setLoadingMods,
  ] =
  useState(false);

  const [
    launching,
    setLaunching,
  ] =
  useState(false);

  const [
    redetectingGame,
    setRedetectingGame,
  ] =
  useState(false);

  const [
    launchingModded,
    setLaunchingModded,
  ] =
  useState(false);

  const [
    importingMod,
    setImportingMod,
  ] =
  useState(false);

  const [
    installingEngine,
    setInstallingEngine,
  ] =
  useState<string | null>(
    null,
  );

  const [
    newProfileName,
    setNewProfileName,
  ] =
  useState("");

  const [
    showProfileModal,
    setShowProfileModal,
  ] =
  useState(false);

  const [
    showModdedLaunchModal,
    setShowModdedLaunchModal,
  ] =
  useState(false);

  const [
    message,
    setMessage,
  ] =
  useState<string | null>(
    null,
  );

  const [
    appSettings,
    setAppSettings,
  ] =
  useState<AppSettings>(
    loadAppSettings,
  );

  const selectedProfile =
  useMemo(
    () =>
    profiles.profiles.find(
      (profile) =>
      profile.id ===
      profiles.selected_profile,
    ) ?? null,
    [
      profiles,
    ],
  );

  const selectedVariant =
  useMemo(
    () =>
    pendingArchive
    ?.variants
    .find(
      (variant) =>
      variant.id ===
      selectedVariantId,
    ) ?? null,
    [
      pendingArchive,
      selectedVariantId,
    ],
  );

  async function redetectEldenRing() {
    if (redetectingGame) {
      return;
    }

    setRedetectingGame(true);

    try {
      const result =
      await invoke<EldenRingInstallation>(
        "detect_elden_ring",
      );

      setInstallation(result);

      if (result.found) {
        setMessage(
          `Elden Ring detected at ${result.game_path ?? "unknown location"}.`,
        );
      } else {
        setMessage(
          "Elden Ring was not found. Check that Steam knows about its current library location.",
        );
      }
    } catch (error) {
      console.error(error);

      setMessage(
        `Failed to redetect Elden Ring: ${String(error)}`,
      );
    } finally {
      setRedetectingGame(false);
    }
  }

  async function refreshEngines() {
    const result =
    await invoke<EngineOverview>(
      "get_engine_overview",
    );

    setEngines(
      result,
    );

    return result;
  }

  async function refreshConflicts(
    profileId?: string,
  ) {
    const id =
    profileId ??
    profiles.selected_profile;

    if (!id) {
      setConflictReport(
        null,
      );

      return;
    }

    try {
      const result =
      await invoke<ConflictReport>(
        "get_profile_conflicts",
        {
          profileId:
          id,
        },
      );

      setConflictReport(
        result,
      );
    } catch (error) {
      console.error(
        error,
      );

      setConflictReport(
        null,
      );
    }
  }

  async function refreshMods(
    profileId?: string,
  ) {
    const id =
    profileId ??
    profiles.selected_profile;

    if (!id) {
      setInstalledMods([]);
      return;
    }

    setLoadingMods(
      true,
    );

    try {
      const result =
      await invoke<InstalledMod[]>(
        "list_profile_mods",
        {
          profileId:
          id,
        },
      );

      setInstalledMods(
        result,
      );
    } catch (error) {
      console.error(
        error,
      );

      setInstalledMods(
        [],
      );
    } finally {
      setLoadingMods(
        false,
      );
    }
  }

  async function refreshEngineDecision(
    profileId?: string,
  ) {
    const id =
    profileId ??
    profiles.selected_profile;

    if (!id) {
      return;
    }

    try {
      const result =
      await invoke<EngineDecision>(
        "get_profile_engine_decision",
        {
          id,
        },
      );

      setEngineDecision(
        result,
      );
    } catch (error) {
      console.error(
        error,
      );

      setEngineDecision(
        null,
      );
    }
  }

  async function loadEverything() {
    setLoading(
      true,
    );

    try {
      const [
        installationResult,
        profileResult,
        platformResult,
        engineResult,
      ] =
      await Promise.all([
        invoke<EldenRingInstallation>(
          "detect_elden_ring",
        ),

        invoke<ProfileConfig>(
          "get_profiles",
        ),

        invoke<PlatformInfo>(
          "get_platform_info",
        ),

        invoke<EngineOverview>(
          "get_engine_overview",
        ),
      ]);

      setInstallation(
        installationResult,
      );

      setProfiles(
        profileResult,
      );

      setPlatform(
        platformResult,
      );

      setEngines(
        engineResult,
      );

      if (
        profileResult
        .selected_profile
      ) {
        const [
          decision,
          mods,
          conflicts,
        ] =
        await Promise.all([
          invoke<EngineDecision>(
            "get_profile_engine_decision",
            {
              id:
              profileResult
              .selected_profile,
            },
          ),

          invoke<InstalledMod[]>(
            "list_profile_mods",
            {
              profileId:
              profileResult
              .selected_profile,
            },
          ),

          invoke<ConflictReport>(
            "get_profile_conflicts",
            {
              profileId:
              profileResult
              .selected_profile,
            },
          ),
        ]);

        setEngineDecision(
          decision,
        );

        setInstalledMods(
          mods,
        );

        setConflictReport(
          conflicts,
        );
      }
    } catch (error) {
      console.error(
        error,
      );

      setMessage(
        `Failed to load manager state: ${String(
          error,
        )}`,
      );
    } finally {
      setLoading(
        false,
      );
    }
  }

  const [availableUpdateVersion, setAvailableUpdateVersion] =
    useState<string | null>(null);
  const [installingUpdate, setInstallingUpdate] =
    useState(false);

  useEffect(() => {
    void initializeDownloadStore();
    loadEverything();

    void (async () => {
      try {
        const update = await check();

        if (!update) {
          return;
        }

        setAvailableUpdateVersion(update.version);
        await update.close();
      } catch (error) {
        console.error("Automatic update check failed:", error);
      }
    })();
  }, []);

  async function installAvailableUpdate() {
    if (installingUpdate) {
      return;
    }

    setInstallingUpdate(true);

    try {
      const update = await check();

      if (!update) {
        setAvailableUpdateVersion(null);
        setMessage("Elden Mod Manager is already up to date.");
        return;
      }

      setMessage(`Downloading Elden Mod Manager ${update.version}...`);
      await update.downloadAndInstall();

      setMessage(`Elden Mod Manager ${update.version} installed. Restarting...`);
      await relaunch();
    } catch (error) {
      console.error("Update installation failed:", error);
      setMessage(`Failed to install update: ${String(error)}`);
      setInstallingUpdate(false);
    }
  }

  useEffect(() => {
    try {
      localStorage.setItem(
        settingsStorageKey,
        JSON.stringify(
          appSettings,
        ),
      );
    } catch {
      // Settings persistence is non-critical.
    }
  }, [
    appSettings,
  ]);

  function updateAppSetting(
    key: keyof AppSettings,
    value: boolean,
  ) {
    setAppSettings(
      (current) => ({
        ...current,
        [key]: value,
      }),
    );
  }

  function resetAppSettings() {
    setAppSettings(
      defaultAppSettings,
    );

    setMessage(
      "Settings reset to defaults.",
    );
  }

  async function launchVanilla() {
    setLaunching(
      true,
    );

    setMessage(
      null,
    );

    try {
      await invoke(
        "launch_elden_ring_vanilla",
      );
    } catch (error) {
      setMessage(
        `Failed to launch Elden Ring: ${String(
          error,
        )}`,
      );
    } finally {
      setLaunching(
        false,
      );
    }
  }

  async function launchModded() {
    if (!selectedProfile) {
      setMessage(
        "No profile is selected.",
      );

      return;
    }

    setLaunchingModded(
      true,
    );

    setMessage(
      null,
    );

    try {
      await invoke(
        "launch_elden_ring_modded",
        {
          profileId:
          selectedProfile.id,
        },
      );

      setShowModdedLaunchModal(
        false,
      );

      setMessage(
        `Launching ${selectedProfile.name} through me3.`,
      );
    } catch (error) {
      console.error(
        error,
      );

      setMessage(
        `Failed to launch modded Elden Ring: ${String(
          error,
        )}`,
      );
    } finally {
      setLaunchingModded(
        false,
      );
    }
  }

  async function installOrUpdateMe3() {
    if (installingEngine) {
      return;
    }

    setInstallingEngine(
      "me3",
    );

    setMessage(
      null,
    );

    try {
      const result =
      await invoke<EngineStatus>(
        "install_me3",
      );

      await refreshEngines();

      if (
        result.installed_version
      ) {
        setMessage(
          `me3 ${result.installed_version} installed successfully.`,
        );
      } else {
        setMessage(
          "me3 installed successfully.",
        );
      }
    } catch (error) {
      console.error(
        error,
      );

      setMessage(
        `me3 installation failed: ${String(
          error,
        )}`,
      );
    } finally {
      setInstallingEngine(
        null,
      );
    }
  }

  async function installChosenVariant(
    analysis: ModArchiveAnalysis,
    variant: ModArchiveVariant,
  ) {
    if (!selectedProfile) {
      return;
    }

    const installed =
    await invoke<InstalledMod>(
      "import_mod_variant",
      {
        profileId:
        selectedProfile.id,

        archivePath:
        analysis.archive_path,

        variantPath:
        variant.relative_path,

        variantName:
        variant.name,

        multipleVariants:
        analysis.variants.length > 1,
      },
    );

    await refreshMods(
      selectedProfile.id,
    );

    await refreshEngineDecision(
      selectedProfile.id,
    );

    await refreshConflicts(
      selectedProfile.id,
    );

    setMessage(
      `${installed.name} installed into ${selectedProfile.name}.`,
    );
  }

  async function installModFromFile() {
    if (!selectedProfile) {
      setMessage(
        "Select a profile first.",
      );

      return;
    }

    setImportingMod(
      true,
    );

    setMessage(
      null,
    );

    try {
      const archivePath =
      await invoke<string | null>(
        "pick_mod_archive",
      );

      if (!archivePath) {
        return;
      }

      const analysis =
      await invoke<ModArchiveAnalysis>(
        "analyze_mod_archive",
        {
          archivePath,
        },
      );

      if (
        analysis
        .variants
        .length === 0
      ) {
        throw new Error(
          "No installable variants were detected.",
        );
      }

      if (
        analysis
        .variants
        .length === 1
      ) {
        await installChosenVariant(
          analysis,
          analysis.variants[0],
        );

        return;
      }

      setPendingArchive(
        analysis,
      );

      setSelectedVariantId(
        analysis
        .variants[0]
        .id,
      );
    } catch (error) {
      console.error(
        error,
      );

      setMessage(
        `Mod import failed: ${String(
          error,
        )}`,
      );
    } finally {
      setImportingMod(
        false,
      );
    }
  }

  async function confirmVariantInstall() {
    if (
      !pendingArchive ||
      !selectedVariant
    ) {
      return;
    }

    setImportingMod(
      true,
    );

    setMessage(
      null,
    );

    try {
      await installChosenVariant(
        pendingArchive,
        selectedVariant,
      );

      setPendingArchive(
        null,
      );

      setSelectedVariantId(
        null,
      );
    } catch (error) {
      console.error(
        error,
      );

      setMessage(
        `Mod import failed: ${String(
          error,
        )}`,
      );
    } finally {
      setImportingMod(
        false,
      );
    }
  }

  async function toggleMod(
    mod: InstalledMod,
  ) {
    if (!selectedProfile) {
      return;
    }

    try {
      const result =
      await invoke<InstalledMod[]>(
        "set_mod_enabled",
        {
          profileId:
          selectedProfile.id,

          modId:
          mod.id,

          enabled:
          !mod.enabled,
        },
      );

      setInstalledMods(
        result,
      );

      await refreshEngineDecision(
        selectedProfile.id,
      );

      await refreshConflicts(
        selectedProfile.id,
      );
    } catch (error) {
      setMessage(
        `Failed to update mod: ${String(
          error,
        )}`,
      );
    }
  }

  async function moveInstalledMod(
    mod: InstalledMod,
    direction: "up" | "down",
  ) {
    if (!selectedProfile) {
      return;
    }

    try {
      const result =
      await invoke<InstalledMod[]>(
        "move_mod",
        {
          profileId:
          selectedProfile.id,

          modId:
          mod.id,

          direction,
        },
      );

      setInstalledMods(
        result,
      );

      await refreshConflicts(
        selectedProfile.id,
      );
    } catch (error) {
      setMessage(
        `Failed to change load order: ${String(
          error,
        )}`,
      );
    }
  }

  async function reorderInstalledMods(
    modIds: string[],
  ) {
    if (!selectedProfile) {
      return;
    }

    try {
      const result =
        await invoke<InstalledMod[]>(
          "set_mod_order",
          {
            profileId:
              selectedProfile.id,

            modIds,
          },
        );

      setInstalledMods(
        result,
      );

      await refreshConflicts(
        selectedProfile.id,
      );
    } catch (error) {
      setMessage(
        `Failed to change load order: ${String(
          error,
        )}`,
      );
    }
  }

  async function uninstallInstalledMod(
    mod: InstalledMod,
  ) {
    if (!selectedProfile) {
      return;
    }

    try {
      const result =
      await invoke<InstalledMod[]>(
        "uninstall_mod",
        {
          profileId:
          selectedProfile.id,

          modId:
          mod.id,
        },
      );

      setInstalledMods(
        result,
      );

      await refreshEngineDecision(
        selectedProfile.id,
      );

      await refreshConflicts(
        selectedProfile.id,
      );

      setMessage(
        `${mod.name} uninstalled.`,
      );
    } catch (error) {
      setMessage(
        `Failed to uninstall mod: ${String(
          error,
        )}`,
      );
    }
  }

  async function uninstallInstalledMods(
    modIds: string[],
  ) {
    if (
      !selectedProfile ||
      modIds.length === 0
    ) {
      return;
    }

    try {
      let result =
        installedMods;

      /*
       * Delete sequentially instead of in parallel.
       *
       * Each uninstall updates the same profile.json,
       * so racing several writes would be a bad idea.
       */
      for (const modId of modIds) {
        result =
          await invoke<InstalledMod[]>(
            "uninstall_mod",
            {
              profileId:
                selectedProfile.id,

              modId,
            },
          );
      }

      setInstalledMods(
        result,
      );

      await Promise.all([
        refreshEngineDecision(
          selectedProfile.id,
        ),

        refreshConflicts(
          selectedProfile.id,
        ),
      ]);

      setMessage(
        `${modIds.length} mod${
          modIds.length === 1
            ? ""
            : "s"
        } uninstalled.`,
      );
    } catch (error) {
      await Promise.all([
        refreshMods(
          selectedProfile.id,
        ),

        refreshEngineDecision(
          selectedProfile.id,
        ),

        refreshConflicts(
          selectedProfile.id,
        ),
      ]);

      setMessage(
        `Failed to uninstall selected mods: ${String(
          error,
        )}`,
      );

      throw error;
    }
  }

  async function createProfile() {
    const name =
    newProfileName.trim();

    if (!name) {
      return;
    }

    try {
      const result =
      await invoke<ProfileConfig>(
        "create_profile",
        {
          name,
        },
      );

      setProfiles(
        result,
      );

      setNewProfileName(
        "",
      );

      setShowProfileModal(
        false,
      );

      await refreshEngineDecision(
        result.selected_profile,
      );

      await refreshMods(
        result.selected_profile,
      );
    } catch (error) {
      setMessage(
        String(error),
      );
    }
  }

  async function selectProfile(
    id: string,
  ) {
    try {
      const result =
      await invoke<ProfileConfig>(
        "select_profile",
        {
          id,
        },
      );

      setProfiles(
        result,
      );

      await Promise.all([
        refreshEngineDecision(
          id,
        ),

        refreshMods(
          id,
        ),

        refreshConflicts(
          id,
        ),
      ]);
    } catch (error) {
      setMessage(
        String(error),
      );
    }
  }

  async function deleteProfile(
    id: string,
  ) {
    try {
      const result =
      await invoke<ProfileConfig>(
        "delete_profile",
        {
          id,
        },
      );

      setProfiles(
        result,
      );

      await Promise.all([
        refreshEngineDecision(
          result.selected_profile,
        ),

        refreshMods(
          result.selected_profile,
        ),

        refreshConflicts(
          result.selected_profile,
        ),
      ]);
    } catch (error) {
      setMessage(
        String(error),
      );
    }
  }

  async function changeEngine(
    engine: EngineKind,
  ) {
    if (!selectedProfile) {
      return;
    }

    try {
      const result =
      await invoke<ProfileConfig>(
        "set_profile_engine",
        {
          id:
          selectedProfile.id,

          engine,
        },
      );

      setProfiles(
        result,
      );

      await refreshEngineDecision(
        selectedProfile.id,
      );
    } catch (error) {
      setMessage(
        String(error),
      );
    }
  }

  const selectedRuntime =
  engineDecision?.selected ??
  "me3";

const selectedEngineInstalled =
selectedRuntime ===
"modengine2"
? engines
?.modengine2
.installed
: engines
?.me3
.installed;

const enabledModCount =
installedMods.filter(
  (mod) =>
  mod.enabled,
).length;

return (
  <div
    className={`app-shell ${
      appSettings.reducedEffects
        ? "reduced-effects"
        : ""
    }`}
  >
  {availableUpdateVersion && (
    <div className="app-update-overlay">
      <div className="app-update-dialog">
        <div className="app-update-title">
          Update Available
        </div>

        <div className="app-update-text">
          Elden Mod Manager {availableUpdateVersion} is available.
        </div>

        <div className="app-update-actions">
          <button
            className="app-update-later"
            disabled={installingUpdate}
            onClick={() => setAvailableUpdateVersion(null)}
          >
            Later
          </button>

          <button
            className="app-update-now"
            disabled={installingUpdate}
            onClick={() => void installAvailableUpdate()}
          >
            {installingUpdate ? "Installing..." : "Update Now"}
          </button>
        </div>
      </div>
    </div>
  )}

  <header
    className="custom-titlebar"
    onMouseDown={async (event) => {
      if (event.button !== 0) return;

      const target = event.target as HTMLElement;

      if (target.closest(".custom-titlebar-controls")) {
        return;
      }

      await getCurrentWindow().startDragging();
    }}
  >
    <div
      className="custom-titlebar-brand"
      data-tauri-drag-region
    >
      <img
        className="custom-titlebar-icon"
        src={appIcon}
        alt=""
        draggable={false}
      />

      <span
        className="custom-titlebar-title"
        data-tauri-drag-region
      >
        Elden Mod Manager
      </span>
    </div>

    <div className="custom-titlebar-controls">
      <button
        className="custom-titlebar-button"
        aria-label="Minimize"
        title="Minimize"
        onClick={() =>
          getCurrentWindow().minimize()
        }
      >
        —
      </button>

      <button
        className="custom-titlebar-button"
        aria-label="Maximize"
        title="Maximize"
        onClick={() =>
          getCurrentWindow().toggleMaximize()
        }
      >
        □
      </button>

      <button
        className="custom-titlebar-button custom-titlebar-close"
        aria-label="Close"
        title="Close"
        onClick={() =>
          getCurrentWindow().close()
        }
      >
        ×
      </button>
    </div>
  </header>

  <aside className="sidebar">
  <div className="brand">
  <div className="brand-mark">
  ER
  </div>

  <div>
  <div className="brand-title">
  Elden Mod Manager
  </div>

  <div className="brand-subtitle">
  Community Mod Launcher
  </div>
  </div>
  </div>

  <nav className="nav">
  {(
    [
      [
        "dashboard",
        "Dashboard",
        "⌂",
      ],
      [
        "browse",
        "Browse Mods",
        "◇",
      ],
      [
        "mods",
        "My Mods",
        "◆",
      ],
      [
        "game_settings",
        "Game Settings",
        "⚙",
      ],
      [
        "profiles",
        "Profiles",
        "♙",
      ],
      [
        "downloads",
        "Downloads",
        "⇩",
      ],
      [
        "settings",
        "Settings",
        "☰",
      ],
    ] as [
      Page,
      string,
      string,
    ][]
  ).map(
    ([
      id,
      label,
      icon,
    ]) => (
      <NavButton
        key={id}
        label={label}
        icon={icon}
        active={
          page === id
        }
        onClick={async () => {
          setPage(id);

          if (
            (id === "mods" ||
             id === "browse") &&
            selectedProfile
          ) {
            await Promise.all([
              refreshMods(
                selectedProfile.id,
              ),
              refreshEngineDecision(
                selectedProfile.id,
              ),
              refreshConflicts(
                selectedProfile.id,
              ),
            ]);
          }
        }}
      />
    ),
  )}
  </nav>

  <div className="sidebar-bottom">
  <div className="mini-status">
  <span
  className={`status-dot ${
    installation?.found
    ? "online"
    : ""
  }`}
  />

  <div>
  <strong>
  Elden Ring
  </strong>

  <span>
  {installation?.found
    ? "Detected"
    : "Not detected"}
    </span>
    </div>
    </div>

    <div className="version">
    Development Build
    </div>
    </div>
    </aside>

    <main className="main">
    <DownloadProgress />
    {message && (
      <div className="notification">
      <span>
      {message}
      </span>

      <button
      onClick={() =>
        setMessage(
          null,
        )
      }
      >
      ×
      </button>
      </div>
    )}

    {loading ? (
      <div className="loading-screen">
      <div className="spinner" />

      <span>
      Loading Elden Mod Manager…
      </span>
      </div>
    ) : page ===
    "dashboard" ? (
      <Dashboard
      installation={
        installation
      }
      platform={
        platform
      }
      selectedProfile={
        selectedProfile
      }
      engines={
        engines
      }
      engineDecision={
        engineDecision
      }
      enabledModCount={
        enabledModCount
      }
      launching={
        launching
      }
      launchingModded={
        launchingModded
      }
      installingEngine={
        installingEngine
      }
      selectedEngineInstalled={
        !!selectedEngineInstalled
      }
      onLaunchVanilla={
        launchVanilla
      }
      onOpenModdedLaunch={() =>
        setShowModdedLaunchModal(
          true,
        )
      }
      onChangeEngine={
        changeEngine
      }
      onInstallMe3={
        installOrUpdateMe3
      }
      onOpenProfiles={() =>
        setPage(
          "profiles",
        )
      }
      onOpenMods={() =>
        setPage(
          "mods",
        )
      }
      onRedetectGame={
        redetectEldenRing
      }
      redetectingGame={
        redetectingGame
      }
      />
    ) : page ===
    "browse" ? (
      <BrowseModsPage
        selectedProfileId={
          selectedProfile?.id ?? null
        }
        installedMods={
          installedMods
        }
        onModsChanged={async () => {
          if (!selectedProfile) {
            return;
          }

          await Promise.all([
            refreshMods(
              selectedProfile.id,
            ),
            refreshEngineDecision(
              selectedProfile.id,
            ),
            refreshConflicts(
              selectedProfile.id,
            ),
          ]);
        }}
      />
    ) : page ===
    "mods" ? (
      <ModsPage
      selectedProfile={
        selectedProfile
      }
      mods={
        installedMods
      }
      conflictReport={
        conflictReport
      }
      settings={
        appSettings
      }
      loading={
        loadingMods
      }
      importing={
        importingMod
      }
      onInstall={
        installModFromFile
      }
      onToggle={
        toggleMod
      }
      onMove={
        moveInstalledMod
      }
      onReorder={
        reorderInstalledMods
      }
      onUninstall={
        uninstallInstalledMod
      }
      onBulkUninstall={
        uninstallInstalledMods
      }
      />
    ) : page ===
    "game_settings" ? (
      <GameSettingsPage />
    ) : page ===
    "profiles" ? (
      <ProfilesPage
      profiles={
        profiles
      }
      selectedProfile={
        selectedProfile
      }
      engineDecision={
        engineDecision
      }
      onSelect={
        selectProfile
      }
      onDelete={
        deleteProfile
      }
      onChangeEngine={
        changeEngine
      }
      onCreate={() =>
        setShowProfileModal(
          true,
        )
      }
      onModpackImported={
        loadEverything
      }
      />
    ) : page ===
    "downloads" ? (
      <DownloadsPage />
    ) : page ===
    "settings" ? (
      <SettingsPage
        settings={
          appSettings
        }
        engines={
          engines
        }
        engineDecision={
          engineDecision
        }
        onChange={
          updateAppSetting
        }
        onReset={
          resetAppSettings
        }
      />
    ) : (
      <ComingSoonPage
      page={
        page
      }
      />
    )}
    </main>

    {showProfileModal && (
      <div
      className="modal-backdrop"
      onMouseDown={() =>
        setShowProfileModal(
          false,
        )
      }
      >
      <div
      className="modal"
      onMouseDown={(
        event,
      ) =>
      event.stopPropagation()
      }
      >
      <div className="eyebrow">
      NEW PROFILE
      </div>

      <h2>
      Create Profile
      </h2>

      <p>
      Mods installed in this
      profile remain isolated
      from your other profiles.
      </p>

      <input
      autoFocus
      value={
        newProfileName
      }
      placeholder="Profile name"
      onChange={(
        event,
      ) =>
      setNewProfileName(
        event.target
        .value,
      )
      }
      onKeyDown={(
        event,
      ) => {
        if (
          event.key ===
          "Enter"
        ) {
          createProfile();
        }
      }}
      />

      <div className="modal-actions">
      <button
      className="button secondary"
      onClick={() =>
        setShowProfileModal(
          false,
        )
      }
      >
      Cancel
      </button>

      <button
      className="button primary"
      onClick={
        createProfile
      }
      >
      Create Profile
      </button>
      </div>
      </div>
      </div>
    )}

    {pendingArchive && (
      <div
      className="modal-backdrop"
      onMouseDown={() => {
        if (!importingMod) {
          setPendingArchive(
            null,
          );

          setSelectedVariantId(
            null,
          );
        }
      }}
      >
      <div
      className="modal"
      onMouseDown={(
        event,
      ) =>
      event.stopPropagation()
      }
      >
      <div className="eyebrow">
      MOD OPTIONS DETECTED
      </div>

      <h2>
      Choose a Variant
      </h2>

      <p>
      This archive contains
      multiple installable
      versions. Choose the one
      you want in this profile.
      </p>

      <div className="decision-box">
      <span>
      ARCHIVE
      </span>

      <strong>
      {pendingArchive.display_name}
      </strong>

      <p>
      {pendingArchive
        .variants
        .length}{" "}
        variants detected
        </p>
        </div>

        <div className="profile-list">
        {pendingArchive
          .variants
          .map(
            (variant) => (
              <button
              key={
                variant.id
              }
              className={`profile-item ${
                selectedVariantId ===
                variant.id
                ? "active"
                : ""
              }`}
              disabled={
                importingMod
              }
              onClick={() =>
                setSelectedVariantId(
                  variant.id,
                )
              }
              >
              <div>
              <strong>
              {variant.name}
              </strong>

              <span>
              {variant.relative_path}
              </span>
              </div>

              {selectedVariantId ===
                variant.id && (
                  <span className="active-tag">
                  SELECTED
                  </span>
                )}
                </button>
            ),
          )}
          </div>

          <div className="modal-actions">
          <button
          className="button secondary"
          disabled={
            importingMod
          }
          onClick={() => {
            setPendingArchive(
              null,
            );

            setSelectedVariantId(
              null,
            );
          }}
          >
          Cancel
          </button>

          <button
          className="button primary"
          disabled={
            importingMod ||
            !selectedVariant
          }
          onClick={
            confirmVariantInstall
          }
          >
          {importingMod
            ? "Installing…"
            : "Install Selected Variant"}
            </button>
            </div>
            </div>
            </div>
    )}

    {showModdedLaunchModal && (
      <div
      className="modal-backdrop"
      onMouseDown={() =>
        setShowModdedLaunchModal(
          false,
        )
      }
      >
      <div
      className="modal"
      onMouseDown={(
        event,
      ) =>
      event.stopPropagation()
      }
      >
      <div className="eyebrow">
      MODDED SESSION
      </div>

      <h2>
      Launch Modded
      </h2>

      <p>
      Elden Ring will launch
      through me3 using the{" "}
      <strong>
      {selectedProfile
        ?.name ??
        "selected"}
        </strong>{" "}
        profile.
        </p>

        <div className="decision-box">
        <span>
        ENABLED MODS
        </span>

        <strong>
        {enabledModCount}
        </strong>

        <p>
        Only enabled mods
        from this profile
        will be added to the
        generated me3
        configuration.
        </p>
        </div>

        <div className="decision-box">
        <span>
        OFFICIAL MATCHMAKING
        </span>

        <strong>
        Disabled
        </strong>

        <p>
        This modded session
        will not use Elden
        Ring's official
        matchmaking.
        </p>
        </div>

        <div className="modal-actions">
        <button
        className="button secondary"
        disabled={
          launchingModded
        }
        onClick={() =>
          setShowModdedLaunchModal(
            false,
          )
        }
        >
        Cancel
        </button>

        <button
        className="button primary"
        disabled={
          launchingModded
        }
        onClick={
          launchModded
        }
        >
        {launchingModded
          ? "Launching…"
          : "Launch Modded"}
          </button>
          </div>
          </div>
          </div>
    )}
    </div>
);
}

function NavButton({
  label,
  icon,
  active,
  onClick,
}: {
  label: string;
  icon: string;
  active: boolean;
  onClick: () => void | Promise<void>;
}) {
  return (
    <button
      className={`nav-button ${
        active
          ? "active"
          : ""
      }`}
      onClick={
        onClick
      }
    >
      <span className="nav-line" />

      <span
        className="nav-icon"
        aria-hidden="true"
      >
        {icon}
      </span>

      <span>
        {label}
      </span>
    </button>
  );
}

function Dashboard({
  installation,
  platform,
  selectedProfile,
  engines,
  engineDecision,
  enabledModCount,
  launching,
  launchingModded,
  installingEngine,
  selectedEngineInstalled,
  onLaunchVanilla,
  onOpenModdedLaunch,
  onChangeEngine,
  onInstallMe3,
  onOpenProfiles,
  onOpenMods,
  onRedetectGame,
  redetectingGame,
}: {
  installation: EldenRingInstallation | null;
  platform: PlatformInfo | null;
  selectedProfile: Profile | null;
  engines: EngineOverview | null;
  engineDecision: EngineDecision | null;
  enabledModCount: number;
  launching: boolean;
  launchingModded: boolean;
  installingEngine: string | null;
  selectedEngineInstalled: boolean;
  onLaunchVanilla: () => void;
  onOpenModdedLaunch: () => void;
  onChangeEngine: (engine: EngineKind) => void;
  onInstallMe3: () => void;
  onOpenProfiles: () => void;
  onOpenMods: () => void;
  onRedetectGame: () => void;
  redetectingGame: boolean;
}) {
  return (
    <>
    <header className="page-header">
    <div>
    <div className="eyebrow">
    ELDEN RING
    </div>

    <h1>
    Dashboard
    </h1>

    <p>
    Manage isolated mod
    profiles and launch
    Elden Ring your way.
    </p>
    </div>

    <div className="platform-pill">
    {platform
      ?.display_name ??
      "Unknown Platform"}

      {platform
        ?.uses_proton && (
          <span>
          PROTON
          </span>
        )}
        </div>
        </header>

        <section className="hero-card">
        <div className="hero-content">
        <div className="eyebrow">
        ACTIVE PROFILE
        </div>

        <h2>
        {selectedProfile
          ?.name ??
          "Default"}
          </h2>

          <p>
          {enabledModCount} enabled mod(s)
          </p>

          <button
          className="text-button"
          onClick={
            onOpenMods
          }
          >
          Manage mods →
          </button>

          <button
          className="text-button"
          onClick={
            onOpenProfiles
          }
          >
          Manage profiles →
          </button>
          </div>

          <div className="launch-area">
          <button
          className="launch-button vanilla"
          disabled={
            !installation?.found ||
            launching
          }
          onClick={
            onLaunchVanilla
          }
          >
          <span>
          LAUNCH
          </span>

          <strong>
          {launching
            ? "Starting…"
            : "Vanilla"}
            </strong>
            </button>

            <button
            className="launch-button modded"
            disabled={
              !installation?.found ||
              !selectedEngineInstalled ||
              launchingModded
            }
            onClick={
              onOpenModdedLaunch
            }
            >
            <span>
            LAUNCH
            </span>

            <strong>
            {launchingModded
              ? "Starting…"
              : "Modded"}
              </strong>

              {!selectedEngineInstalled ? (
                <small>
                ENGINE REQUIRED
                </small>
              ) : (
                <small>
                {enabledModCount} MOD(S) ENABLED
                </small>
              )}
              </button>
              </div>
              </section>

              <section className="section">
              <div className="section-heading">
              <div>
              <div className="eyebrow">
              MODDING ENGINES
              </div>

              <h2>
              Runtime Backends
              </h2>
              </div>

              <p>
              The manager translates
              your unified profile
              into whichever runtime
              your mods require.
              </p>
              </div>

              <div className="engine-grid">
              {engines && (
                <>
                <EngineCard
                engine={
                  engines.me3
                }
                badge="RECOMMENDED"
                installing={
                  installingEngine ===
                  "me3"
                }
                onSetup={
                  onInstallMe3
                }
                />

                <EngineCard
                engine={
                  engines.modengine2
                }
                badge="LEGACY"
                installing={
                  false
                }
                onSetup={() => {}}
                />
                </>
              )}
              </div>
              </section>

              <section className="two-column">
              <div className="panel">
              <div className="eyebrow">
              PROFILE ENGINE
              </div>

              <h2>
              Compatibility Mode
              </h2>

              <p className="muted">
              Automatic chooses the
              best runtime for the
              enabled mods.
              </p>

              <label className="field-label">
              Runtime preference
              </label>

              <select
              value={
                selectedProfile
                ?.engine ??
                "auto"
              }
              onChange={(
                event,
              ) =>
              onChangeEngine(
                event.target
                .value as EngineKind,
              )
              }
              >
              <option value="auto">
              Automatic (Recommended)
              </option>

              <option value="me3">
              me3
              </option>

              <option value="modengine2">
              Mod Engine 2
              </option>
              </select>

              <div className="decision-box">
              <span>
              CURRENT DECISION
              </span>

              <strong>
              {engineDisplayName(
                engineDecision
                ?.selected ??
                "auto",
              )}
              </strong>

              <p>
              {engineDecision
                ?.reason ??
                "No runtime decision available."}
                </p>
                </div>
                </div>

                <div className="panel">
                <div className="eyebrow">
                INSTALLATION
                </div>

                <h2>
                Game Status
                </h2>

                <button
                  className="text-button"
                  disabled={
                    redetectingGame
                  }
                  onClick={
                    onRedetectGame
                  }
                  style={{
                    marginBottom: "14px",
                  }}
                >
                  {redetectingGame
                    ? "↻ Detecting…"
                    : "↻ Redetect Game"}
                </button>

                <StatusRow
                label="Elden Ring"
                value={
                  installation?.found
                  ? "Detected"
                  : "Not Found"
                }
                good={
                  !!installation?.found
                }
                />

                <StatusRow
                label="Platform"
                value={
                  platform
                  ?.display_name ??
                  "Unknown"
                }
                />

                <StatusRow
                label="Compatibility"
                value={
                  platform
                  ?.uses_proton
                  ? "Steam / Proton"
                  : "Native"
                }
                />

                <div className="path-box">
                <span>
                GAME DIRECTORY
                </span>

                <code>
                {shortPath(
                  installation
                  ?.game_path ??
                  null,
                )}
                </code>
                </div>
                </div>
                </section>
                </>
  );
}

type NexusModUpdate = {
  local_mod_id: string;

  nexus_mod_id: number;

  installed_file_id: number;
  installed_version: string;

  latest_file_id: number | null;
  latest_version: string | null;
  latest_file_name: string | null;
  latest_uploaded_timestamp: number | null;

  update_available: boolean;
  manual_check_required: boolean;

  reason: string;
};

function ModsPage({
  selectedProfile,
  mods,
  conflictReport,
  settings,
  loading,
  importing,
  onInstall,
  onToggle,
  onMove,
  onReorder,
  onUninstall,
  onBulkUninstall,
}: {
  selectedProfile: Profile | null;
  mods: InstalledMod[];
  conflictReport: ConflictReport | null;
  settings: AppSettings;
  loading: boolean;
  importing: boolean;
  onInstall: () => void;
  onToggle: (mod: InstalledMod) => void;
  onMove: (
    mod: InstalledMod,
    direction: "up" | "down",
  ) => void;
  onReorder: (
    modIds: string[],
  ) => Promise<void>;
  onUninstall: (mod: InstalledMod) => void;
  onBulkUninstall: (
    modIds: string[],
  ) => Promise<void>;
}) {
  const [
    nexusUpdates,
    setNexusUpdates,
  ] =
  useState<NexusModUpdate[]>([]);

  const [
    checkingUpdates,
    setCheckingUpdates,
  ] =
  useState(false);

  const [
    updateCheckError,
    setUpdateCheckError,
  ] =
  useState<string | null>(
    null,
  );

  async function checkForNexusUpdates() {
    if (!selectedProfile) {
      return;
    }

    setCheckingUpdates(
      true,
    );

    setUpdateCheckError(
      null,
    );

    try {
      const result =
        await invoke<
          NexusModUpdate[]
        >(
          "check_nexus_mod_updates",
          {
            profileId:
              selectedProfile.id,
          },
        );

            setNexusUpdates(
        result,
      );
    } catch (error) {
      console.error(
        error,
      );

      setUpdateCheckError(
        String(
          error,
        ),
      );
    } finally {
      setCheckingUpdates(
        false,
      );
    }
  }

  function nexusUpdateFor(
    modId: string,
  ) {
    return (
      nexusUpdates.find(
        (update) =>
          update.local_mod_id ===
          modId,
      ) ?? null
    );
  }

  const [
    expandedMods,
    setExpandedMods,
  ] =
  useState<Set<string>>(
    () =>
      settings.expandModsByDefault
        ? new Set(
            mods.map(
              (mod) => mod.id,
            ),
          )
        : new Set(),
  );

  useEffect(() => {
    setNexusUpdates(
      [],
    );

    setUpdateCheckError(
      null,
    );

    if (
      selectedProfile &&
      mods.length > 0
    ) {
      void checkForNexusUpdates();
    }
  }, [
    selectedProfile?.id,
  ]);

  useEffect(() => {
    if (
      settings.expandModsByDefault
    ) {
      setExpandedMods(
        new Set(
          mods.map(
            (mod) => mod.id,
          ),
        ),
      );
    } else {
      setExpandedMods(
        new Set(),
      );
    }
  }, [
    settings.expandModsByDefault,
  ]);

  const [
    selectionMode,
    setSelectionMode,
  ] =
  useState(false);

  const [
    selectedModIds,
    setSelectedModIds,
  ] =
  useState<Set<string>>(
    new Set(),
  );

  const [
    bulkDeleting,
    setBulkDeleting,
  ] =
  useState(false);

  const [
    draggedModId,
    setDraggedModId,
  ] =
  useState<string | null>(
    null,
  );

  const [
    dragOverModId,
    setDragOverModId,
  ] =
  useState<string | null>(
    null,
  );

  const conflictsByMod =
  useMemo(() => {
    const lookup =
    new Map<
      string,
      ModConflict[]
    >();

    for (
      const conflict of
      conflictReport?.conflicts ?? []
    ) {
      for (
        const modId of
        conflict.mod_ids
      ) {
        const existing =
          lookup.get(
            modId,
          );

        if (existing) {
          existing.push(
            conflict,
          );
        } else {
          lookup.set(
            modId,
            [conflict],
          );
        }
      }
    }

    return lookup;
  }, [
    conflictReport,
  ]);

  function conflictsForMod(
    modId: string,
  ) {
    return (
      conflictsByMod.get(
        modId,
      ) ?? []
    );
  }

  function beginModDrag(
    modId: string,
    event: React.DragEvent<HTMLDivElement>,
  ) {
    if (
      selectionMode ||
      bulkDeleting
    ) {
      event.preventDefault();
      return;
    }

    setDraggedModId(
      modId,
    );

    event.dataTransfer.effectAllowed =
      "move";

    event.dataTransfer.setData(
      "text/plain",
      modId,
    );
  }

  function dragOverMod(
    modId: string,
    event: React.DragEvent<HTMLDivElement>,
  ) {
    if (
      !draggedModId ||
      draggedModId === modId
    ) {
      return;
    }

    event.preventDefault();

    event.dataTransfer.dropEffect =
      "move";

    setDragOverModId(
      modId,
    );
  }

  async function dropMod(
    targetModId: string,
    event: React.DragEvent<HTMLDivElement>,
  ) {
    event.preventDefault();

    const sourceModId =
      draggedModId ??
      event.dataTransfer.getData(
        "text/plain",
      );

    setDraggedModId(
      null,
    );

    setDragOverModId(
      null,
    );

    if (
      !sourceModId ||
      sourceModId === targetModId
    ) {
      return;
    }

    const currentIds =
      mods.map(
        (mod) =>
          mod.id,
      );

    const sourceIndex =
      currentIds.indexOf(
        sourceModId,
      );

    const targetIndex =
      currentIds.indexOf(
        targetModId,
      );

    if (
      sourceIndex === -1 ||
      targetIndex === -1
    ) {
      return;
    }

    const targetRect =
      event.currentTarget
        .getBoundingClientRect();

    const placeAfter =
      event.clientY >
      targetRect.top +
      targetRect.height / 2;

    const next =
      [...currentIds];

    next.splice(
      sourceIndex,
      1,
    );

    let insertionIndex =
      next.indexOf(
        targetModId,
      );

    if (placeAfter) {
      insertionIndex += 1;
    }

    next.splice(
      insertionIndex,
      0,
      sourceModId,
    );

    if (
      next.every(
        (id, index) =>
          id === currentIds[index],
      )
    ) {
      return;
    }

    await onReorder(
      next,
    );
  }

  function finishModDrag() {
    setDraggedModId(
      null,
    );

    setDragOverModId(
      null,
    );
  }

  function contentLabel(
    mod: InstalledMod,
  ) {
    const hasAssets =
    mod.contents.some(
      (content) =>
      content.content_type ===
      "assets",
    );

    const hasNative =
    mod.contents.some(
      (content) =>
      content.content_type ===
      "native_dll",
    );

    if (hasAssets && hasNative) {
      return "Mixed asset + native mod";
    }

    if (hasNative) {
      return "Native DLL mod";
    }

    if (hasAssets) {
      return "Loose-file asset mod";
    }

    return "Mod";
  }

  function runtimeLabel(
    mod: InstalledMod,
  ) {
    const hasNative =
    mod.contents.some(
      (content) =>
      content.content_type ===
      "native_dll",
    );

    const hasAssets =
    mod.contents.some(
      (content) =>
      content.content_type ===
      "assets",
    );

    if (hasNative && hasAssets) {
      return "me3 package + native";
    }

    if (hasNative) {
      return "me3 native";
    }

    return "me3 package";
  }

  function toggleExpanded(
    modId: string,
  ) {
    setExpandedMods(
      (current) => {
        const next =
          new Set(
            current,
          );

        if (next.has(modId)) {
          next.delete(
            modId,
          );
        } else {
          next.add(
            modId,
          );
        }

        return next;
      },
    );
  }

  function toggleSelected(
    modId: string,
  ) {
    setSelectedModIds(
      (current) => {
        const next =
          new Set(
            current,
          );

        if (next.has(modId)) {
          next.delete(
            modId,
          );
        } else {
          next.add(
            modId,
          );
        }

        return next;
      },
    );
  }

  function leaveSelectionMode() {
    setSelectionMode(
      false,
    );

    setSelectedModIds(
      new Set(),
    );
  }

  function requestUninstall(
    mod: InstalledMod,
  ) {
    if (
      settings.confirmSingleUninstall &&
      !window.confirm(
        `Uninstall "${mod.name}" from this profile?`,
      )
    ) {
      return;
    }

    onUninstall(
      mod,
    );
  }

  async function deleteSelectedMods() {
    const ids =
      Array.from(
        selectedModIds,
      );

    if (ids.length === 0) {
      return;
    }

    const confirmed =
      !settings.confirmBulkUninstall ||
      window.confirm(
        `Uninstall ${ids.length} selected mod${
          ids.length === 1
            ? ""
            : "s"
        } from this profile?`,
      );

    if (!confirmed) {
      return;
    }

    setBulkDeleting(
      true,
    );

    try {
      await onBulkUninstall(
        ids,
      );

      leaveSelectionMode();
    } finally {
      setBulkDeleting(
        false,
      );
    }
  }

  return (
    <>
      <header className="page-header">
        <div>
          <div className="eyebrow">
            ACTIVE PROFILE
          </div>

          <h1>
            My Mods
          </h1>

          <p>
            Mods installed into{" "}
            <strong>
              {selectedProfile
                ?.name ??
                "Default"}
            </strong>
            .
          </p>
        </div>

        <div className="mods-header-actions">
          {selectionMode ? (
            <>
              <button
                className="button secondary"
                disabled={
                  bulkDeleting
                }
                onClick={
                  leaveSelectionMode
                }
              >
                Cancel Selection
              </button>

              <button
                className="button danger"
                disabled={
                  bulkDeleting ||
                  selectedModIds.size === 0
                }
                onClick={
                  deleteSelectedMods
                }
              >
                {bulkDeleting
                  ? "Deleting…"
                  : `Delete Selected (${selectedModIds.size})`}
              </button>
            </>
          ) : (
            <button
              className="button secondary"
              disabled={
                mods.length === 0
              }
              onClick={() =>
                setSelectionMode(
                  true,
                )
              }
            >
              Select Mods
            </button>
          )}

          <button
            className="button secondary"
            disabled={
              checkingUpdates ||
              !selectedProfile ||
              mods.length === 0 ||
              bulkDeleting
            }
            onClick={
              checkForNexusUpdates
            }
          >
            {checkingUpdates
              ? "Checking Updates…"
              : "Check Updates"}
          </button>

          <button
            className="button primary"
            disabled={
              importing ||
              !selectedProfile ||
              bulkDeleting
            }
            onClick={
              onInstall
            }
          >
            {importing
              ? "Analyzing…"
              : "+ Install Mod From File"}
          </button>
        </div>
      </header>

      {updateCheckError && (
        <section className="panel">
          <div className="eyebrow">
            NEXUS UPDATE CHECK
          </div>

          <p className="muted">
            {updateCheckError}
          </p>
        </section>
      )}

      {conflictReport &&
       conflictReport.conflict_count > 0 && (
        <section
          className="panel conflict-summary"
        >
          <div className="eyebrow">
            CONFLICT DETECTOR
          </div>

          <h2>
            ⚠{" "}
            {conflictReport.conflict_count}{" "}
            conflict
            {conflictReport.conflict_count === 1
              ? ""
              : "s"}{" "}
            detected
          </h2>

          <p className="muted">
            Asset conflicts are resolved by
            load order. The lower mod in the
            list loads later and wins. Native
            DLL conflicts still require manual
            review.
          </p>
        </section>
      )}

      {loading ? (
        <div className="panel">
          <p className="muted">
            Loading mods…
          </p>
        </div>
      ) : mods.length === 0 ? (
        <div className="panel">
          <div className="eyebrow">
            NO MODS INSTALLED
          </div>

          <h2>
            This profile is clean.
          </h2>

          <p className="muted">
            Choose a downloaded Elden
            Ring ZIP, RAR, or 7Z mod
            archive. The manager will
            inspect its structure before
            installing anything.
          </p>

          <button
            className="button primary"
            disabled={
              importing
            }
            onClick={
              onInstall
            }
          >
            {importing
              ? "Analyzing…"
              : "Install Mod From File"}
          </button>
        </div>
      ) : (
        <div className="mods-list">
          {mods.map(
            (
              mod,
              modIndex,
            ) => {
              const modConflicts =
                conflictsForMod(
                  mod.id,
                );

              const nexusUpdate =
                nexusUpdateFor(
                  mod.id,
                );

              const expanded =
                expandedMods.has(
                  mod.id,
                );

              const selected =
                selectedModIds.has(
                  mod.id,
                );

              const nativeConflicts =
                modConflicts.filter(
                  (conflict) =>
                    conflict.conflict_type ===
                    "native",
                );

              const winningAssetConflicts =
                modConflicts.filter(
                  (conflict) =>
                    conflict.conflict_type ===
                    "asset" &&
                    conflict.winner_mod_id ===
                    mod.id,
                );

              const overriddenAssetConflicts =
                modConflicts.filter(
                  (conflict) =>
                    conflict.conflict_type ===
                    "asset" &&
                    conflict.winner_mod_id !==
                    null &&
                    conflict.winner_mod_id !==
                    mod.id,
                );

              const overriddenByNames =
                Array.from(
                  new Set(
                    overriddenAssetConflicts
                      .map(
                        (conflict) =>
                          conflict.winner_mod_name,
                      )
                      .filter(
                        (
                          name,
                        ): name is string =>
                          !!name,
                      ),
                  ),
                );

              const overriddenNames =
                Array.from(
                  new Set(
                    winningAssetConflicts
                      .flatMap(
                        (conflict) =>
                          conflict
                            .overridden_mod_names,
                      ),
                  ),
                );

              return (
                <div
                  draggable={
                    !selectionMode &&
                    !bulkDeleting
                  }
                  onDragStart={(
                    event,
                  ) =>
                    beginModDrag(
                      mod.id,
                      event,
                    )
                  }
                  onDragOver={(
                    event,
                  ) =>
                    dragOverMod(
                      mod.id,
                      event,
                    )
                  }
                  onDrop={(
                    event,
                  ) =>
                    dropMod(
                      mod.id,
                      event,
                    )
                  }
                  onDragEnd={
                    finishModDrag
                  }
                  className={`panel mod-card mod-card-collapsible ${
                    draggedModId === mod.id
                      ? "mod-card-dragging "
                      : ""
                  }${
                    dragOverModId === mod.id &&
                    draggedModId !== mod.id
                      ? "mod-card-drag-over "
                      : ""
                  }${
                    !mod.enabled
                      ? "mod-card-disabled"
                      : modConflicts.length > 0
                        ? "mod-card-conflict"
                        : "mod-card-enabled"
                  }`}
                  key={
                    mod.id
                  }
                >
                  <div className="mod-card-compact">
                    {selectionMode && (
                      <label className="mod-select-control">
                        <input
                          type="checkbox"
                          checked={
                            selected
                          }
                          disabled={
                            bulkDeleting
                          }
                          onChange={() =>
                            toggleSelected(
                              mod.id,
                            )
                          }
                        />

                        <span>
                          Select
                        </span>
                      </label>
                    )}

                    {!selectionMode && (
                      <span
                        className="mod-drag-handle"
                        title="Drag to change load order"
                        aria-hidden="true"
                      >
                        ⠿
                      </span>
                    )}

                    <button
                      className="mod-expand-button"
                      type="button"
                      title={
                        expanded
                          ? "Hide mod details"
                          : "Show mod details"
                      }
                      aria-expanded={
                        expanded
                      }
                      onClick={() =>
                        toggleExpanded(
                          mod.id,
                        )
                      }
                    >
                      {expanded
                        ? "▾"
                        : "▸"}
                    </button>

                    <div className="mod-name-update-group">
                      <h2 className="mod-compact-name">
                        {mod.name}
                      </h2>

                      {nexusUpdate?.update_available && (
                        <span
                          className="mod-update-badge mod-update-badge-available"
                          title={
                            nexusUpdate.reason
                          }
                        >
                          UPDATE AVAILABLE
                        </span>

                      
                      )}

                      {nexusUpdate?.update_available &&
                       nexusUpdate.latest_file_id !== null &&
                       selectedProfile && (
                        <NexusUpdateButton
                          profileId={
                            selectedProfile.id
                          }
                          localModId={
                            mod.id
                          }
                          nexusModId={
                            nexusUpdate.nexus_mod_id
                          }
                          nexusFileId={
                            nexusUpdate.latest_file_id
                          }
                          nexusVersion={
                            nexusUpdate.latest_version
                          }
                          onUpdated={async () => {
                            await checkForNexusUpdates();
                          }}
                        />
                      )}

                      

                      {nexusUpdate?.manual_check_required && (
                        <span
                          className="mod-update-badge mod-update-badge-manual"
                          title={
                            nexusUpdate.reason
                          }
                        >
                          CHECK NEXUS
                        </span>
                      )}
                    </div>

                    <div className="mod-quick-actions">
                      <button
                        className="button secondary"
                        disabled={
                          bulkDeleting
                        }
                        onClick={() =>
                          onToggle(
                            mod,
                          )
                        }
                      >
                        {mod.enabled
                          ? "Disable"
                          : "Enable"}
                      </button>

                      <button
                        className="button danger"
                        disabled={
                          bulkDeleting
                        }
                        onClick={() =>
                          requestUninstall(
                            mod,
                          )
                        }
                      >
                        Uninstall
                      </button>
                    </div>
                  </div>

                  {expanded && (
                    <div className="mod-card-details">
                      <div className="mod-detail-heading">
                        <div>
                          <div className="eyebrow">
                            {mod.enabled
                              ? modConflicts.length > 0
                                ? "ENABLED • CONFLICT"
                                : "ENABLED"
                              : "DISABLED"}
                          </div>

                          <p className="muted">
                            {contentLabel(
                              mod,
                            )}
                          </p>
                        </div>
                      </div>

                      <StatusRow
                        label="Status"
                        value={
                          mod.enabled
                            ? modConflicts.length > 0
                              ? "Conflict"
                              : "Enabled"
                            : "Disabled"
                        }
                        good={
                          mod.enabled &&
                          modConflicts.length === 0
                        }
                      />

                      <StatusRow
                        label="Runtime"
                        value={
                          runtimeLabel(
                            mod,
                          )
                        }
                      />

                      {nexusUpdate && (
                        <>
                          <StatusRow
                            label="Nexus"
                            value={
                              nexusUpdate.update_available
                                ? "Update Available"
                                : nexusUpdate.manual_check_required
                                  ? "Manual Check"
                                  : "Up to Date"
                            }
                            good={
                              !nexusUpdate.update_available &&
                              !nexusUpdate.manual_check_required
                            }
                          />

                          <StatusRow
                            label="Installed Version"
                            value={
                              nexusUpdate.installed_version ||
                              `File ${nexusUpdate.installed_file_id}`
                            }
                          />

                          {nexusUpdate.latest_version && (
                            <StatusRow
                              label="Latest Version"
                              value={
                                nexusUpdate.latest_version
                              }
                              good={
                                nexusUpdate.update_available
                              }
                            />
                          )}

                          {nexusUpdate.reason && (
                            <div className="decision-box">
                              <span>
                                NEXUS UPDATE STATUS
                              </span>

                              <strong>
                                {nexusUpdate.reason}
                              </strong>
                            </div>
                          )}
                        </>
                      )}

                      {settings.showRuntimeDetails && (
                        <div className="debug-detail-grid">
                          <div className="engine-path">
                            <span>
                              MOD ID
                            </span>

                            <code>
                              {mod.id}
                            </code>
                          </div>

                          <div className="engine-path">
                            <span>
                              CONTENT ENTRIES
                            </span>

                            <code>
                              {mod.contents.length}
                            </code>
                          </div>
                        </div>
                      )}

                      <div className="load-order-row">
                        <div className="load-order-label">
                          <span>
                            LOAD ORDER
                          </span>

                          <strong>
                            {String(
                              modIndex + 1,
                            ).padStart(
                              2,
                              "0",
                            )}
                          </strong>
                        </div>

                        <div className="load-order-controls">
                          <button
                            className="load-order-button"
                            title="Move earlier"
                            disabled={
                              modIndex === 0 ||
                              bulkDeleting
                            }
                            onClick={() =>
                              onMove(
                                mod,
                                "up",
                              )
                            }
                          >
                            ↑
                          </button>

                          <button
                            className="load-order-button"
                            title="Move later"
                            disabled={
                              modIndex ===
                              mods.length - 1 ||
                              bulkDeleting
                            }
                            onClick={() =>
                              onMove(
                                mod,
                                "down",
                              )
                            }
                          >
                            ↓
                          </button>
                        </div>
                      </div>

                      {mod.enabled &&
                       winningAssetConflicts.length > 0 && (
                        <div className="decision-box conflict-box conflict-winner-box">
                          <span>
                            WINNING ASSET CONFLICT
                          </span>

                          <strong>
                            Winning{" "}
                            {winningAssetConflicts.length}{" "}
                            conflicting asset
                            {winningAssetConflicts.length === 1
                              ? ""
                              : "s"}
                          </strong>

                          <p>
                            This mod loads later and
                            overrides{" "}
                            {overriddenNames.length}{" "}
                            earlier mod
                            {overriddenNames.length === 1
                              ? ""
                              : "s"}
                            {overriddenNames.length > 0
                              ? `: ${overriddenNames.join(", ")}`
                              : "."}
                          </p>
                        </div>
                      )}

                      {mod.enabled &&
                       overriddenAssetConflicts.length > 0 && (
                        <div className="decision-box conflict-box">
                          <span>
                            OVERRIDDEN ASSET CONFLICT
                          </span>

                          <strong>
                            Overridden{" "}
                            {overriddenAssetConflicts.length}{" "}
                            conflicting asset
                            {overriddenAssetConflicts.length === 1
                              ? ""
                              : "s"}
                          </strong>

                          <p>
                            Overridden by{" "}
                            {overriddenByNames.join(
                              ", ",
                            )}
                            .
                          </p>
                        </div>
                      )}

                      {mod.enabled &&
                       nativeConflicts.length > 0 && (
                        <div className="decision-box conflict-box">
                          <span>
                            NATIVE CONFLICT
                          </span>

                          <strong>
                            Manual review recommended
                          </strong>

                          <p>
                            {nativeConflicts.length} native
                            DLL conflict
                            {nativeConflicts.length === 1
                              ? ""
                              : "s"}{" "}
                            detected. Load order is not
                            treated as a safe resolution
                            for native modules.
                          </p>
                        </div>
                      )}

                      {settings.verboseConflicts &&
                       modConflicts.length > 0 && (
                        <div className="debug-conflict-summary">
                          <span>
                            VERBOSE CONFLICT INFO
                          </span>

                          <strong>
                            {winningAssetConflicts.length} won ·{" "}
                            {overriddenAssetConflicts.length} overridden ·{" "}
                            {nativeConflicts.length} native
                          </strong>
                        </div>
                      )}

                      {settings.showInternalPaths &&
                       modConflicts.length > 0 && (
                        <div className="debug-path-list">
                          <span>
                            INTERNAL CONFLICT PATHS
                          </span>

                          {modConflicts.map(
                            (
                              conflict,
                              index,
                            ) => (
                              <code
                                key={`${conflict.conflict_type}-${conflict.relative_path}-${index}`}
                              >
                                {conflict.relative_path}
                              </code>
                            ),
                          )}
                        </div>
                      )}

                      <div className="engine-path">
                        <span>
                          MOD ID
                        </span>

                        <code>
                          {mod.id}
                        </code>
                      </div>
                    </div>
                  )}
                </div>
              );
            },
          )}
        </div>
      )}
    </>
  );
}

function EngineCard({
  engine,
  badge,
  installing,
  onSetup,
}: {
  engine: EngineStatus;
  badge: string;
  installing: boolean;
  onSetup: () => void;
}) {
  return (
    <div
    className={`engine-card ${
      engine.preferred
      ? "preferred"
      : ""
    }`}
    >
    <div className="engine-top">
    <div>
    <h3>
    {engine.name}
    </h3>

    <p>
    {engine.description}
    </p>
    </div>

    <span className="engine-badge">
    {badge}
    </span>
    </div>

    <div className="engine-status">
    <span
    className={`status-dot ${
      engine.installed
      ? "online"
      : ""
    }`}
    />

    <strong>
    {installing
      ? engine.update_available
      ? "Updating…"
      : "Installing…"
      : engine.installed
      ? engine.update_available
      ? "Update Available"
      : "Installed"
      : "Not Installed"}
      </strong>
      </div>

      {engine.installed && (
        <>
        <div className="engine-path">
        <span>
        INSTALLED VERSION
        </span>

        <code>
        {engine.installed_version
          ? `v${engine.installed_version}`
          : "Unknown"}
          </code>
          </div>

          <div className="engine-path">
          <span>
          LATEST VERSION
          </span>

          <code>
          {engine.latest_version
            ? `v${engine.latest_version}`
            : "Unable to check"}
            </code>
            </div>
            </>
      )}

      <div className="engine-path">
      <span>
      MANAGED LOCATION
      </span>

      <code>
      {shortPath(
        engine.engine_path,
      )}
      </code>
      </div>

      {!engine.installed ? (
        <button
        className="button primary"
        disabled={
          installing
        }
        onClick={
          onSetup
        }
        >
        {installing
          ? "Installing…"
          : `Set Up ${engine.name}`}
          </button>
      ) : engine.update_available ? (
        <button
        className="button primary"
        disabled={
          installing
        }
        onClick={
          onSetup
        }
        >
        {installing
          ? "Updating…"
          : `Update ${engine.name}`}
          </button>
      ) : (
        <button
        className="button installed"
        disabled
        >
        ✓ Up to Date
        </button>
      )}
      </div>
  );
}

function StatusRow({
  label,
  value,
  good,
}: {
  label: string;
  value: string;
  good?: boolean;
}) {
  return (
    <div className="status-row">
    <span>
    {label}
    </span>

    <strong
    className={
      good
      ? "good-text"
      : ""
    }
    >
    {value}
    </strong>
    </div>
  );
}

function ProfilesPage({
  profiles,
  selectedProfile,
  engineDecision,
  onSelect,
  onDelete,
  onChangeEngine,
  onCreate,
  onModpackImported,
}: {
  profiles: ProfileConfig;
  selectedProfile: Profile | null;
  engineDecision: EngineDecision | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
  onChangeEngine: (engine: EngineKind) => void;
  onCreate: () => void;
  onModpackImported: () => Promise<void>;
}) {
  const [
    modpackBusy,
    setModpackBusy,
  ] = useState(false);

  const [
    modpackMessage,
    setModpackMessage,
  ] = useState<string | null>(null);

  useEffect(() => {
    let removeListener:
      (() => void) | undefined;

    listen<NexusDownloadProgressEvent>(
      "nexus-download-progress",
      (event) => {
        const progress =
          event.payload;

        if (
          progress.state ===
          "awaiting_authorization"
        ) {
          setModpackMessage(
            "Waiting for Nexus authorization — click Slow Download on the Nexus page to continue.",
          );

          return;
        }

        if (
          progress.state ===
          "connecting"
        ) {
          setModpackMessage(
            `Authorized. Connecting to Nexus for ${progress.file_name}…`,
          );

          return;
        }

        if (
          progress.state ===
          "downloading"
        ) {
          const percent =
            progress.percent !== null
              ? ` ${Math.round(
                  progress.percent,
                )}%`
              : "";

          setModpackMessage(
            `Downloading ${progress.file_name}…${percent}`,
          );

          return;
        }

        if (
          progress.state ===
          "complete"
        ) {
          setModpackMessage(
            `Downloaded ${progress.file_name}. Continuing modpack installation…`,
          );
        }
      },
    ).then(
      (unlisten) => {
        removeListener =
          unlisten;
      },
    );

    return () => {
      removeListener?.();
    };
  }, []);


  async function exportSelectedModpack() {
    if (!selectedProfile) {
      return;
    }

    setModpackBusy(true);
    setModpackMessage(null);

    try {
      const path =
        await invoke<string | null>(
          "export_modpack",
          {
            profileId:
              selectedProfile.id,
          },
        );

      if (path) {
        setModpackMessage(
          `Modpack exported to ${path}`,
        );
      }
    } catch (error) {
      console.error(error);
      setModpackMessage(
        `Modpack export failed: ${String(error)}`,
      );
    } finally {
      setModpackBusy(false);
    }
  }

  async function importModpack() {
    setModpackBusy(true);
    setModpackMessage(
      "Installing modpack…",
    );

    try {
      const result =
        await invoke<ModpackImportResult | null>(
          "import_modpack",
        );

      if (!result) {
        setModpackMessage(null);
        return;
      }

      await onModpackImported();

      const details: string[] = [
        `${result.installed_count} mod${
          result.installed_count === 1
            ? ""
            : "s"
        } installed`,
      ];

      if (
        result.skipped_manual.length > 0
      ) {
        details.push(
          `${result.skipped_manual.length} manual mod${
            result.skipped_manual.length === 1
              ? ""
              : "s"
          } need to be installed separately`,
        );
      }

      if (result.failures.length > 0) {
        details.push(
          `${result.failures.length} failed`,
        );
      }

      setModpackMessage(
        `${result.profile_name}: ${details.join(" • ")}`,
      );
    } catch (error) {
      console.error(error);
      setModpackMessage(
        `Modpack import failed: ${String(error)}`,
      );
    } finally {
      setModpackBusy(false);
    }
  }

  return (
    <>
    <header className="page-header">
    <div>
    <div className="eyebrow">
    LOADOUTS
    </div>

    <h1>
    Profiles
    </h1>

    <p>
    Keep different mod
    setups completely
    separated.
    </p>
    </div>

    <div className="modal-actions">
    <button
    className="button secondary"
    disabled={
      modpackBusy
    }
    onClick={
      importModpack
    }
    >
    {modpackBusy
      ? "Working…"
      : "Import Modpack"}
    </button>

    <button
    className="button secondary"
    disabled={
      modpackBusy ||
      !selectedProfile
    }
    onClick={
      exportSelectedModpack
    }
    >
    Export Modpack
    </button>

    <button
    className="button primary"
    disabled={
      modpackBusy
    }
    onClick={
      onCreate
    }
    >
    + New Profile
    </button>
    </div>
    </header>

    {modpackMessage && (
      <div className="decision-box">
      <span>
      MODPACK
      </span>

      <p>
      {modpackMessage}
      </p>
      </div>
    )}

    <div className="profile-layout">
    <div className="profile-list">
    {profiles.profiles.map(
      (profile) => (
        <button
        key={
          profile.id
        }
        className={`profile-item ${
          profile.id ===
          profiles.selected_profile
          ? "active"
          : ""
        }`}
        onClick={() =>
          onSelect(
            profile.id,
          )
        }
        >
        <div>
        <strong>
        {profile.name}
        </strong>

        <span>
        {engineDisplayName(
          profile.engine,
        )}
        </span>
        </div>

        {profile.id ===
          profiles.selected_profile && (
            <span className="active-tag">
            ACTIVE
            </span>
          )}
          </button>
      ),
    )}
    </div>

    <div className="panel profile-detail">
    <div className="eyebrow">
    SELECTED PROFILE
    </div>

    <h2>
    {selectedProfile
      ?.name ??
      "No profile"}
      </h2>

      <label className="field-label">
      Runtime preference
      </label>

      <select
      value={
        selectedProfile
        ?.engine ??
        "auto"
      }
      onChange={(
        event,
      ) =>
      onChangeEngine(
        event.target
        .value as EngineKind,
      )
      }
      >
      <option value="auto">
      Automatic (Recommended)
      </option>

      <option value="me3">
      me3
      </option>

      <option value="modengine2">
      Mod Engine 2
      </option>
      </select>

      <div className="decision-box">
      <span>
      RUNTIME DECISION
      </span>

      <strong>
      {engineDisplayName(
        engineDecision
        ?.selected ??
        "auto",
      )}
      </strong>

      <p>
      {engineDecision
        ?.reason ??
        "No decision available."}
        </p>
        </div>

        {selectedProfile &&
          selectedProfile.id !==
          "default" && (
            <button
            className="button danger"
            onClick={() =>
              onDelete(
                selectedProfile.id,
              )
            }
            >
            Delete Profile
            </button>
          )}
          </div>
          </div>
          </>
  );
}

function SettingToggle({
  title,
  description,
  checked,
  onChange,
}: {
  title: string;
  description: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="setting-row">
      <div className="setting-copy">
        <strong>
          {title}
        </strong>

        <span>
          {description}
        </span>
      </div>

      <input
        className="setting-checkbox"
        type="checkbox"
        checked={
          checked
        }
        onChange={(
          event,
        ) =>
          onChange(
            event.target.checked,
          )
        }
      />
    </label>
  );
}

function SettingsPage({
  settings,
  engines,
  engineDecision,
  onChange,
  onReset,
}: {
  settings: AppSettings;
  engines: EngineOverview | null;
  engineDecision: EngineDecision | null;
  onChange: (
    key: keyof AppSettings,
    value: boolean,
  ) => void;
  onReset: () => void;
}) {
  return (
    <>
      <header className="page-header">
        <div>
          <div className="eyebrow">
            APPLICATION
          </div>

          <h1>
            Settings
          </h1>

          <p>
            Configure mod management,
            debugging, and interface behavior.
          </p>
        </div>
      </header>

      <div className="settings-grid">
        <section className="panel settings-panel">
          <div className="eyebrow">
            MOD BEHAVIOR
          </div>

          <h2>
            Mod Management
          </h2>

          <p className="muted">
            Control confirmations and how mod
            information is displayed.
          </p>

          <div className="settings-list">
            <SettingToggle
              title="Confirm single uninstall"
              description="Ask before uninstalling one mod."
              checked={
                settings.confirmSingleUninstall
              }
              onChange={
                (value) =>
                  onChange(
                    "confirmSingleUninstall",
                    value,
                  )
              }
            />

            <SettingToggle
              title="Confirm bulk uninstall"
              description="Ask before deleting a selected group of mods."
              checked={
                settings.confirmBulkUninstall
              }
              onChange={
                (value) =>
                  onChange(
                    "confirmBulkUninstall",
                    value,
                  )
              }
            />

            <SettingToggle
              title="Expand mod details by default"
              description="Open mod details automatically in My Mods."
              checked={
                settings.expandModsByDefault
              }
              onChange={
                (value) =>
                  onChange(
                    "expandModsByDefault",
                    value,
                  )
              }
            />
          </div>
        </section>

        <section className="panel settings-panel">
          <div className="eyebrow">
            APPEARANCE
          </div>

          <h2>
            Performance
          </h2>

          <p className="muted">
            Control heavier visual effects on
            the mod list.
          </p>

          <div className="settings-list">
            <SettingToggle
              title="Reduced visual effects"
              description="Use the lightweight My Mods rendering that avoids WebKit lag."
              checked={
                settings.reducedEffects
              }
              onChange={
                (value) =>
                  onChange(
                    "reducedEffects",
                    value,
                  )
              }
            />
          </div>
        </section>

        <section className="panel settings-panel">
          <div className="eyebrow">
            DEBUGGING
          </div>

          <h2>
            Technical Details
          </h2>

          <p className="muted">
            Reveal technical information that
            stays hidden during normal use.
          </p>

          <div className="settings-list">
            <SettingToggle
              title="Show internal asset paths"
              description="Show exact conflicting asset and DLL paths inside expanded mod cards."
              checked={
                settings.showInternalPaths
              }
              onChange={
                (value) =>
                  onChange(
                    "showInternalPaths",
                    value,
                  )
              }
            />

            <SettingToggle
              title="Show runtime details"
              description="Show internal mod IDs and content entry information."
              checked={
                settings.showRuntimeDetails
              }
              onChange={
                (value) =>
                  onChange(
                    "showRuntimeDetails",
                    value,
                  )
              }
            />

            <SettingToggle
              title="Verbose conflict information"
              description="Show won, overridden, and native conflict counts."
              checked={
                settings.verboseConflicts
              }
              onChange={
                (value) =>
                  onChange(
                    "verboseConflicts",
                    value,
                  )
              }
            />
          </div>
        </section>

        <section className="panel settings-panel">
          <div className="eyebrow">
            RUNTIME
          </div>

          <h2>
            Engine Status
          </h2>

          <StatusRow
            label="Selected runtime"
            value={
              engineDisplayName(
                engineDecision?.selected ??
                "auto",
              )
            }
          />

          <StatusRow
            label="me3"
            value={
              engines?.me3.installed
                ? engines.me3.installed_version
                  ? `v${engines.me3.installed_version}`
                  : "Installed"
                : "Not Installed"
            }
            good={
              !!engines?.me3.installed
            }
          />

          <StatusRow
            label="Mod Engine 2"
            value={
              engines?.modengine2.installed
                ? engines.modengine2.installed_version
                  ? `v${engines.modengine2.installed_version}`
                  : "Installed"
                : "Not Installed"
            }
            good={
              !!engines?.modengine2.installed
            }
          />
        </section>
      </div>

      <section className="panel settings-reset-panel">
        <div>
          <div className="eyebrow">
            RESET
          </div>

          <h2>
            Restore Defaults
          </h2>

          <p className="muted">
            Reset interface settings without
            touching profiles or installed mods.
          </p>
        </div>

        <button
          className="button secondary"
          onClick={
            onReset
          }
        >
          Reset Settings
        </button>
      </section>
    </>
  );
}

function ComingSoonPage({
  page,
}: {
  page: Page;
}) {
  const names:
  Record<Page, string> =
  {
    dashboard:
    "Dashboard",

    browse:
    "Browse Mods",

    mods:
    "My Mods",
    game_settings: "Game Settings",

    profiles:
    "Profiles",

    downloads:
    "Downloads",

    settings:
    "Settings",
  };

  return (
    <div className="coming-soon">
    <div className="eyebrow">
    UNDER CONSTRUCTION
    </div>

    <h1>
    {names[page]}
    </h1>

    <p>
    This section will come
    online as we build the
    mod installation pipeline.
    </p>
    </div>
  );
}

export default App;
