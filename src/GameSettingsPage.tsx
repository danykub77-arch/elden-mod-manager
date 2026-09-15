import {
  useEffect,
  useState,
} from "react";

import {
  invoke,
} from "@tauri-apps/api/core";

import SaveGameSettingsSections from "./SaveGameSettingsSections";

type GameGraphicsSettings = {
  config_found: boolean;
  config_path: string;

  screen_mode: string;

  window_width: number;
  window_height: number;

  fullscreen_width: number;
  fullscreen_height: number;

  borderless_width: number;
  borderless_height: number;

  auto_detect: string;
  quality_setting: string;

  texture_quality: string;
  antialiasing: string;
  ssao: string;
  depth_of_field: string;
  motion_blur: string;
  shadow_quality: string;
  lighting_quality: string;
  effects_quality: string;
  reflection_quality: string;
  water_surface_quality: string;
  shade_quality: string;
  volumetric_effect_quality: string;
  raytracing_quality: string;
  gi_data_quality: string;
  grass_quality: string;

  backup_available: boolean;
};

type SelectOption = {
  value: string;
  label: string;
};

const QUALITY_4: SelectOption[] = [
  {
    value: "LOW",
    label: "Low",
  },
  {
    value: "MEDIUM",
    label: "Medium",
  },
  {
    value: "HIGH",
    label: "High",
  },
  {
    value: "MAX",
    label: "Maximum",
  },
];

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="game-setting-row">
      <div className="game-setting-label">
        <strong>
          {label}
        </strong>

        {description && (
          <span>
            {description}
          </span>
        )}
      </div>

      <div className="game-setting-control">
        {children}
      </div>
    </div>
  );
}

function SettingSelect({
  value,
  options,
  onChange,
}: {
  value: string;
  options: SelectOption[];
  onChange: (
    value: string,
  ) => void;
}) {
  return (
    <select
      value={value}
      onChange={(
        event,
      ) =>
        onChange(
          event.target.value,
        )
      }
    >
      {options.map(
        (option) => (
          <option
            key={
              option.value
            }
            value={
              option.value
            }
          >
            {option.label}
          </option>
        ),
      )}
    </select>
  );
}

function GameSettingsPage() {
  const [
    draft,
    setDraft,
  ] =
    useState<GameGraphicsSettings | null>(
      null,
    );

  const [
    loading,
    setLoading,
  ] =
    useState(true);

  const [
    saving,
    setSaving,
  ] =
    useState(false);

  const [
    message,
    setMessage,
  ] =
    useState<string | null>(
      null,
    );

  async function loadSettings() {
    setLoading(
      true,
    );

    setMessage(
      null,
    );

    try {
      const result =
        await invoke<GameGraphicsSettings>(
          "get_game_graphics_settings",
        );

      setDraft(
        result,
      );
    } catch (error) {
      setMessage(
        `Could not read Elden Ring settings: ${String(
          error,
        )}`,
      );
    } finally {
      setLoading(
        false,
      );
    }
  }

  async function saveSettings() {
    if (!draft) {
      return;
    }

    setSaving(
      true,
    );

    setMessage(
      null,
    );

    try {
      const result =
        await invoke<GameGraphicsSettings>(
          "save_game_graphics_settings",
          {
            settings: draft,
          },
        );

      setDraft(
        result,
      );

      setMessage(
        "Settings saved.",
      );
    } catch (error) {
      setMessage(
        `Could not save Elden Ring settings: ${String(
          error,
        )}`,
      );
    } finally {
      setSaving(
        false,
      );
    }
  }

  async function restoreBackup() {
    setSaving(
      true,
    );

    setMessage(
      null,
    );

    try {
      const result =
        await invoke<GameGraphicsSettings>(
          "restore_game_graphics_backup",
        );

      setDraft(
        result,
      );

      setMessage(
        "Graphics configuration backup restored.",
      );
    } catch (error) {
      setMessage(
        `Could not restore backup: ${String(
          error,
        )}`,
      );
    } finally {
      setSaving(
        false,
      );
    }
  }

  useEffect(() => {
    loadSettings();
  }, []);

  function change<
    K extends keyof GameGraphicsSettings
  >(
    key: K,
    value: GameGraphicsSettings[K],
  ) {
    setDraft(
      (current) => {
        if (!current) {
          return current;
        }

        return {
          ...current,
          [key]: value,
          quality_setting:
            key === "quality_setting"
              ? String(value)
              : "CUSTOM",
        };
      },
    );
  }

  function applyPreset(
    quality:
      | "LOW"
      | "MEDIUM"
      | "HIGH"
      | "MAX",
  ) {
    setDraft(
      (current) => {
        if (!current) {
          return current;
        }

        const shader =
          quality === "LOW"
            ? "LOW"
            : quality === "MEDIUM"
              ? "MEDIUM"
              : "HIGH";

        return {
          ...current,

          quality_setting:
            quality,

          texture_quality:
            quality,

          antialiasing:
            quality === "LOW"
              ? "LOW"
              : "HIGH",

          ssao:
            quality === "LOW"
              ? "DISABLE"
              : quality,

          depth_of_field:
            quality === "LOW"
              ? "DISABLE"
              : quality,

          motion_blur:
            quality === "LOW"
              ? "DISABLE"
              : quality === "MAX"
                ? "HIGH"
                : quality,

          shadow_quality:
            quality,

          lighting_quality:
            quality,

          effects_quality:
            quality,

          volumetric_effect_quality:
            quality,

          reflection_quality:
            quality === "LOW"
              ? "LOW"
              : quality === "MEDIUM"
                ? "LOW"
                : quality,

          water_surface_quality:
            quality === "LOW"
              ? "LOW"
              : "HIGH",

          shade_quality:
            shader,

          gi_data_quality:
            shader,

          grass_quality:
            quality === "LOW"
              ? "MEDIUM"
              : quality === "MEDIUM"
                ? "MEDIUM"
                : quality,

          raytracing_quality:
            "DISABLE",
        };
      },
    );
  }

  if (
    loading &&
    !draft
  ) {
    return (
      <div className="loading-screen">
        <div className="spinner" />

        <span>
          Reading Elden Ring settings…
        </span>
      </div>
    );
  }

  if (!draft) {
    return (
      <>
        <header className="page-header">
          <div>
            <div className="eyebrow">
              ELDEN RING
            </div>

            <h1>
              Game Settings
            </h1>
          </div>
        </header>

        <section className="panel">
          <h2>
            Settings unavailable
          </h2>

          <p className="muted">
            {message ??
              "Elden Ring graphics settings could not be loaded."}
          </p>

          <button
            className="button secondary"
            onClick={
              loadSettings
            }
          >
            Try Again
          </button>
        </section>
      </>
    );
  }

  if (!draft.config_found) {
    return (
      <>
        <header className="page-header">
          <div>
            <div className="eyebrow">
              ELDEN RING
            </div>

            <h1>
              Game Settings
            </h1>

            <p>
              Change the game&apos;s settings without launching it.
            </p>
          </div>
        </header>

        <section className="panel">
          <h2>
            Graphics configuration not found
          </h2>

          <p className="muted">
            Launch Elden Ring normally once so it can create
            GraphicsConfig.xml.
          </p>

          <div className="game-config-path">
            {draft.config_path}
          </div>

          <button
            className="button secondary"
            onClick={
              loadSettings
            }
          >
            Check Again
          </button>
        </section>
      </>
    );
  }

  return (
    <>
      <header className="page-header game-settings-header">
        <div>
          <div className="eyebrow">
            ELDEN RING
          </div>

          <h1>
            Game Settings
          </h1>

          <p>
            Change Elden Ring&apos;s actual graphics configuration.
          </p>
        </div>

        <div className="game-settings-actions">
          <button
            className="button secondary"
            disabled={
              loading ||
              saving
            }
            onClick={
              loadSettings
            }
          >
            Reload From Game
          </button>

          <button
            className="button primary"
            disabled={
              saving
            }
            onClick={
              saveSettings
            }
          >
            {saving
              ? "Saving…"
              : "Save Changes"}
          </button>
        </div>
      </header>

      {message && (
        <div className="game-settings-message">
          {message}
        </div>
      )}

      <SaveGameSettingsSections />

      <section className="panel game-settings-presets">
        <div>
          <div className="eyebrow">
            GRAPHICS
          </div>

          <h2>
            Quality Preset
          </h2>

          <p className="muted">
            Selecting an individual option switches the game to Custom.
          </p>
        </div>

        <div className="game-preset-buttons">
          {[
            "LOW",
            "MEDIUM",
            "HIGH",
            "MAX",
          ].map(
            (quality) => (
              <button
                key={
                  quality
                }
                className={`button ${
                  draft.quality_setting ===
                  quality
                    ? "primary"
                    : "secondary"
                }`}
                onClick={() =>
                  applyPreset(
                    quality as
                      | "LOW"
                      | "MEDIUM"
                      | "HIGH"
                      | "MAX",
                  )
                }
              >
                {quality === "MAX"
                  ? "Maximum"
                  : quality.charAt(0) +
                    quality
                      .slice(1)
                      .toLowerCase()}
              </button>
            ),
          )}
        </div>
      </section>

      <div className="game-settings-grid">
        <section className="panel game-settings-panel">
          <div className="eyebrow">
            DISPLAY
          </div>

          <h2>
            Screen Settings
          </h2>

          <SettingRow
            label="Screen Mode"
          >
            <SettingSelect
              value={
                draft.screen_mode
              }
              options={[
                {
                  value:
                    "FULLSCREEN",
                  label:
                    "Fullscreen",
                },
                {
                  value:
                    "BORDERLESS",
                  label:
                    "Borderless Windowed",
                },
                {
                  value:
                    "WINDOW",
                  label:
                    "Windowed",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "screen_mode",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow
            label="Auto-Detect Best Rendering Settings"
          >
            <SettingSelect
              value={
                draft.auto_detect
              }
              options={[
                {
                  value: "OFF",
                  label: "Off",
                },
                {
                  value: "ON",
                  label: "On",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "auto_detect",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow
            label="Fullscreen Resolution"
          >
            <div className="game-resolution">
              <input
                type="number"
                min="640"
                value={
                  draft.fullscreen_width
                }
                onChange={(
                  event,
                ) =>
                  change(
                    "fullscreen_width",
                    Number(
                      event.target.value,
                    ),
                  )
                }
              />

              <span>×</span>

              <input
                type="number"
                min="360"
                value={
                  draft.fullscreen_height
                }
                onChange={(
                  event,
                ) =>
                  change(
                    "fullscreen_height",
                    Number(
                      event.target.value,
                    ),
                  )
                }
              />
            </div>
          </SettingRow>

          <SettingRow
            label="Borderless Resolution"
          >
            <div className="game-resolution">
              <input
                type="number"
                min="640"
                value={
                  draft.borderless_width
                }
                onChange={(
                  event,
                ) =>
                  change(
                    "borderless_width",
                    Number(
                      event.target.value,
                    ),
                  )
                }
              />

              <span>×</span>

              <input
                type="number"
                min="360"
                value={
                  draft.borderless_height
                }
                onChange={(
                  event,
                ) =>
                  change(
                    "borderless_height",
                    Number(
                      event.target.value,
                    ),
                  )
                }
              />
            </div>
          </SettingRow>

          <SettingRow
            label="Windowed Resolution"
          >
            <div className="game-resolution">
              <input
                type="number"
                min="640"
                value={
                  draft.window_width
                }
                onChange={(
                  event,
                ) =>
                  change(
                    "window_width",
                    Number(
                      event.target.value,
                    ),
                  )
                }
              />

              <span>×</span>

              <input
                type="number"
                min="360"
                value={
                  draft.window_height
                }
                onChange={(
                  event,
                ) =>
                  change(
                    "window_height",
                    Number(
                      event.target.value,
                    ),
                  )
                }
              />
            </div>
          </SettingRow>
        </section>

        <section className="panel game-settings-panel">
          <div className="eyebrow">
            ADVANCED
          </div>

          <h2>
            Quality Settings
          </h2>

          <SettingRow label="Texture Quality">
            <SettingSelect
              value={
                draft.texture_quality
              }
              options={
                QUALITY_4
              }
              onChange={
                (value) =>
                  change(
                    "texture_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Antialiasing Quality">
            <SettingSelect
              value={
                draft.antialiasing
              }
              options={[
                {
                  value:
                    "DISABLE",
                  label:
                    "Off",
                },
                {
                  value:
                    "LOW",
                  label:
                    "Low",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "antialiasing",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="SSAO">
            <SettingSelect
              value={
                draft.ssao
              }
              options={[
                {
                  value:
                    "DISABLE",
                  label:
                    "Off",
                },
                {
                  value:
                    "MEDIUM",
                  label:
                    "Medium",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
                {
                  value:
                    "MAX",
                  label:
                    "Maximum",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "ssao",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Depth of Field">
            <SettingSelect
              value={
                draft.depth_of_field
              }
              options={[
                {
                  value:
                    "DISABLE",
                  label:
                    "Off",
                },
                ...QUALITY_4,
              ]}
              onChange={
                (value) =>
                  change(
                    "depth_of_field",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Motion Blur">
            <SettingSelect
              value={
                draft.motion_blur
              }
              options={[
                {
                  value:
                    "DISABLE",
                  label:
                    "Off",
                },
                {
                  value:
                    "LOW",
                  label:
                    "Low",
                },
                {
                  value:
                    "MEDIUM",
                  label:
                    "Medium",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "motion_blur",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Shadow Quality">
            <SettingSelect
              value={
                draft.shadow_quality
              }
              options={
                QUALITY_4
              }
              onChange={
                (value) =>
                  change(
                    "shadow_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Lighting Quality">
            <SettingSelect
              value={
                draft.lighting_quality
              }
              options={
                QUALITY_4
              }
              onChange={
                (value) =>
                  change(
                    "lighting_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Effects Quality">
            <SettingSelect
              value={
                draft.effects_quality
              }
              options={
                QUALITY_4
              }
              onChange={
                (value) =>
                  change(
                    "effects_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Volumetric Quality">
            <SettingSelect
              value={
                draft.volumetric_effect_quality
              }
              options={
                QUALITY_4
              }
              onChange={
                (value) =>
                  change(
                    "volumetric_effect_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Reflection Quality">
            <SettingSelect
              value={
                draft.reflection_quality
              }
              options={[
                {
                  value:
                    "LOW",
                  label:
                    "Low",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
                {
                  value:
                    "MAX",
                  label:
                    "Maximum",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "reflection_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Water Surface Quality">
            <SettingSelect
              value={
                draft.water_surface_quality
              }
              options={[
                {
                  value:
                    "LOW",
                  label:
                    "Low",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "water_surface_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Shader Quality">
            <SettingSelect
              value={
                draft.shade_quality
              }
              options={[
                {
                  value:
                    "LOW",
                  label:
                    "Low",
                },
                {
                  value:
                    "MEDIUM",
                  label:
                    "Medium",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "shade_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Global Illumination">
            <SettingSelect
              value={
                draft.gi_data_quality
              }
              options={[
                {
                  value:
                    "LOW",
                  label:
                    "Low",
                },
                {
                  value:
                    "MEDIUM",
                  label:
                    "Medium",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "gi_data_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow label="Grass Quality">
            <SettingSelect
              value={
                draft.grass_quality
              }
              options={[
                {
                  value:
                    "MEDIUM",
                  label:
                    "Medium",
                },
                {
                  value:
                    "HIGH",
                  label:
                    "High",
                },
                {
                  value:
                    "MAX",
                  label:
                    "Maximum",
                },
              ]}
              onChange={
                (value) =>
                  change(
                    "grass_quality",
                    value,
                  )
              }
            />
          </SettingRow>

          <SettingRow
            label="Ray Tracing Quality"
            description="Ray tracing can greatly increase GPU load."
          >
            <SettingSelect
              value={
                draft.raytracing_quality
              }
              options={[
                {
                  value:
                    "DISABLE",
                  label:
                    "Off",
                },
                ...QUALITY_4,
              ]}
              onChange={
                (value) =>
                  change(
                    "raytracing_quality",
                    value,
                  )
              }
            />
          </SettingRow>
        </section>
      </div>

      <section className="panel game-settings-recovery">
        <div>
          <div className="eyebrow">
            RECOVERY
          </div>

          <h2>
            Graphics Configuration Backup
          </h2>

          <p className="muted">
            The launcher creates a backup whenever it saves changes.
          </p>

          <div className="game-config-path">
            {draft.config_path}
          </div>
        </div>

        <button
          className="button secondary"
          disabled={
            saving ||
            !draft.backup_available
          }
          onClick={
            restoreBackup
          }
        >
          Restore Backup
        </button>
      </section>
    </>
  );
}

export default GameSettingsPage;
