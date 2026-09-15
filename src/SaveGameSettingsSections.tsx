import {
  type ReactNode,
  useEffect,
  useMemo,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";

type SaveGameSettings = {
  save_found: boolean;
  save_path: string;
  backup_available: boolean;
  camera_speed: number;
  controller_vibration: number;
  brightness: number;
  music_volume: number;
  sound_effects_volume: number;
  voice_volume: number;
  master_volume: number;
  display_blood: number;
  subtitles: number;
  hud: number;
  camera_x_axis: number;
  camera_y_axis: number;
  toggle_auto_lockon: number;
  camera_auto_wall_recovery: number;
  reset_camera_y_axis: number;
  cinematic_effects: number;
  camera_auto_rotation: number;
  perform_matchmaking: number;
  manual_attack_aim: number;
  autotarget: number;
  launchsettings: number;
  send_summon_sign: number;
  hdr: number;
  hdr_adjust_brightness: number;
  hdr_maximum_brightness: number;
  hdr_adjust_saturation: number;
  is_raytracing_on: number;
  mark_new_items: number;
  show_recent_tabs: number;
  show_tutorials: number;
};

type EditableSaveSettings = Omit<
  SaveGameSettings,
  "save_found" | "save_path" | "backup_available" | "hdr_maximum_brightness"
>;

const EDITABLE_KEYS: (keyof EditableSaveSettings)[] = [
  "camera_speed",
  "controller_vibration",
  "brightness",
  "music_volume",
  "sound_effects_volume",
  "voice_volume",
  "master_volume",
  "display_blood",
  "subtitles",
  "hud",
  "camera_x_axis",
  "camera_y_axis",
  "toggle_auto_lockon",
  "camera_auto_wall_recovery",
  "reset_camera_y_axis",
  "cinematic_effects",
  "camera_auto_rotation",
  "perform_matchmaking",
  "manual_attack_aim",
  "autotarget",
  "launchsettings",
  "send_summon_sign",
  "hdr",
  "hdr_adjust_brightness",
  "hdr_adjust_saturation",
  "is_raytracing_on",
  "mark_new_items",
  "show_recent_tabs",
  "show_tutorials",
];

function editableFrom(settings: SaveGameSettings): EditableSaveSettings {
  const result = {} as EditableSaveSettings;
  for (const key of EDITABLE_KEYS) {
    result[key] = settings[key];
  }
  return result;
}

function SettingPanel({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="panel save-settings-panel">
      <div className="eyebrow">{eyebrow}</div>
      <h2>{title}</h2>
      {children}
    </section>
  );
}

function RangeSetting({
  label,
  value,
  disabled,
  onChange,
}: {
  label: string;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <div className="save-setting-row save-range-row">
      <div className="save-setting-info"><strong>{label}</strong></div>
      <div className="save-range-control">
        <input
          type="range"
          min={0}
          max={10}
          step={1}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(Number(event.target.value))}
        />
        <span className="save-setting-value">{value}</span>
      </div>
    </div>
  );
}

function SelectSetting({
  label,
  value,
  disabled,
  options,
  onChange,
}: {
  label: string;
  value: number;
  disabled: boolean;
  options: { value: number; label: string }[];
  onChange: (value: number) => void;
}) {
  const known = options.some((option) => option.value === value);
  return (
    <div className="save-setting-row">
      <div className="save-setting-info"><strong>{label}</strong></div>
      <select
        className="save-setting-select"
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.target.value))}
      >
        {!known && <option value={value}>Unknown ({value})</option>}
        {options.map((option) => (
          <option key={option.value} value={option.value}>{option.label}</option>
        ))}
      </select>
    </div>
  );
}

function ReadOnlySetting({ label, value }: { label: string; value: number }) {
  return (
    <div className="save-setting-row">
      <div className="save-setting-info"><strong>{label}</strong></div>
      <span className="save-setting-value">{value}</span>
    </div>
  );
}

const OFF_ON = [
  { value: 0, label: "Off" },
  { value: 1, label: "On" },
];
const NORMAL_REVERSED = [
  { value: 0, label: "Normal" },
  { value: 1, label: "Reversed" },
];
const BLOOD_OPTIONS = [
  { value: 0, label: "On" },
  { value: 1, label: "Mild" },
  { value: 2, label: "Off" },
];
const HUD_OPTIONS = [
  { value: 0, label: "On" },
  { value: 1, label: "Auto" },
  { value: 2, label: "Off" },
];
const MATCHMAKING_OPTIONS = [
  { value: 0, label: "No Matchmaking" },
  { value: 1, label: "Perform Matchmaking" },
];
const LAUNCH_OPTIONS = [
  { value: 0, label: "Play Online" },
  { value: 1, label: "Play Offline" },
];
const SUMMON_OPTIONS = [
  { value: 0, label: "Disable" },
  { value: 1, label: "Enable" },
];

export default function SaveGameSettingsSections() {
  const [settings, setSettings] = useState<SaveGameSettings | null>(null);
  const [draft, setDraft] = useState<EditableSaveSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  async function loadSettings() {
    setLoading(true);
    setError(null);
    setStatus(null);
    try {
      const result = await invoke<SaveGameSettings>("get_save_game_settings");
      setSettings(result);
      if (result.save_found) setDraft(editableFrom(result));
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadSettings();
  }, []);

  const changed = useMemo(() => {
    if (!settings || !draft) return false;
    return JSON.stringify(editableFrom(settings)) !== JSON.stringify(draft);
  }, [settings, draft]);

  function change(key: keyof EditableSaveSettings, value: number) {
    setDraft((current) => current ? { ...current, [key]: value } : current);
    setStatus(null);
  }

  async function saveChanges() {
    if (!draft) return;
    setSaving(true);
    setError(null);
    setStatus(null);
    try {
      const result = await invoke<SaveGameSettings>("save_save_game_settings", {
        settings: draft,
      });
      setSettings(result);
      setDraft(editableFrom(result));
      setStatus("Settings saved.");
    } catch (caught) {
      setError(String(caught));
    } finally {
      setSaving(false);
    }
  }

  async function restoreBackup() {
    setSaving(true);
    setError(null);
    setStatus(null);
    try {
      const result = await invoke<SaveGameSettings>("restore_save_game_settings_backup");
      setSettings(result);
      setDraft(editableFrom(result));
      setStatus("Backup restored.");
    } catch (caught) {
      setError(String(caught));
    } finally {
      setSaving(false);
    }
  }

  function discardChanges() {
    if (!settings) return;
    setDraft(editableFrom(settings));
    setStatus(null);
  }

  if (loading) {
    return <section className="panel save-settings-loading">Reading Elden Ring system settings…</section>;
  }

  if (!settings || !settings.save_found || !draft) {
    return (
      <section className="panel">
        <div className="eyebrow">SYSTEM SETTINGS</div>
        <h2>ER0000.sl2 Not Found</h2>
        {error && <p className="muted save-settings-error-text">{error}</p>}
        <button className="button secondary" onClick={loadSettings}>Check Again</button>
      </section>
    );
  }

  const disabled = saving;
  const select = (
    label: string,
    key: keyof EditableSaveSettings,
    options: { value: number; label: string }[],
  ) => (
    <SelectSetting
      label={label}
      value={draft[key]}
      disabled={disabled}
      options={options}
      onChange={(value) => change(key, value)}
    />
  );

  return (
    <>
      <section className="panel save-settings-intro">
        <div>
          <div className="eyebrow">ELDEN RING SYSTEM SETTINGS</div>
          <h2>Game Settings</h2>
          <p className="muted">Close Elden Ring before saving changes.</p>
          <div className="save-backup-state">
            {settings.backup_available
              ? "Original backup available"
              : "An original backup will be created before the first save"}
          </div>
        </div>
        <div className="save-settings-actions">
          <button className="button secondary" disabled={saving || !changed} onClick={discardChanges}>Discard</button>
          <button className="button secondary" disabled={saving} onClick={loadSettings}>Reload</button>
          <button className="button secondary" disabled={saving || !settings.backup_available} onClick={restoreBackup}>Restore Backup</button>
          <button className="button primary" disabled={saving || !changed} onClick={saveChanges}>{saving ? "Saving…" : "Save Changes"}</button>
        </div>
      </section>

      {status && <div className="save-settings-status success">{status}</div>}
      {error && <div className="save-settings-status error">{error}</div>}

      <div className="save-settings-grid">
        <SettingPanel eyebrow="GAME" title="Game Options">
          <RangeSetting label="Controller Vibration" value={draft.controller_vibration} disabled={disabled} onChange={(v) => change("controller_vibration", v)} />
          {select("Auto Lock-On", "toggle_auto_lockon", OFF_ON)}
          {select("Manual Attack Aiming", "manual_attack_aim", OFF_ON)}
          {select("Auto-Target", "autotarget", OFF_ON)}
          {select("Mark New Items", "mark_new_items", OFF_ON)}
          {select("Show Recent Tabs", "show_recent_tabs", OFF_ON)}
          {select("Show Tutorials", "show_tutorials", OFF_ON)}
        </SettingPanel>

        <SettingPanel eyebrow="CAMERA" title="Camera Options">
          <RangeSetting label="Camera Speed" value={draft.camera_speed} disabled={disabled} onChange={(v) => change("camera_speed", v)} />
          {select("Camera X Axis", "camera_x_axis", NORMAL_REVERSED)}
          {select("Camera Y Axis", "camera_y_axis", NORMAL_REVERSED)}
          {select("Automatic Wall Recovery", "camera_auto_wall_recovery", OFF_ON)}
          {select("Reset Camera Y Axis", "reset_camera_y_axis", OFF_ON)}
          {select("Cinematic Effects", "cinematic_effects", OFF_ON)}
          {select("Camera Auto-Rotation", "camera_auto_rotation", OFF_ON)}
        </SettingPanel>

        <SettingPanel eyebrow="SOUND & DISPLAY" title="Sound">
          <RangeSetting label="Master Volume" value={draft.master_volume} disabled={disabled} onChange={(v) => change("master_volume", v)} />
          <RangeSetting label="Music" value={draft.music_volume} disabled={disabled} onChange={(v) => change("music_volume", v)} />
          <RangeSetting label="Sound Effects" value={draft.sound_effects_volume} disabled={disabled} onChange={(v) => change("sound_effects_volume", v)} />
          <RangeSetting label="Voice" value={draft.voice_volume} disabled={disabled} onChange={(v) => change("voice_volume", v)} />
        </SettingPanel>

        <SettingPanel eyebrow="SOUND & DISPLAY" title="Display">
          <RangeSetting label="Brightness" value={draft.brightness} disabled={disabled} onChange={(v) => change("brightness", v)} />
          {select("Display Blood", "display_blood", BLOOD_OPTIONS)}
          {select("Subtitles", "subtitles", OFF_ON)}
          {select("HUD", "hud", HUD_OPTIONS)}
          {select("HDR", "hdr", OFF_ON)}
          <RangeSetting label="HDR Brightness" value={draft.hdr_adjust_brightness} disabled={disabled} onChange={(v) => change("hdr_adjust_brightness", v)} />
          <ReadOnlySetting label="HDR Maximum Brightness" value={settings.hdr_maximum_brightness} />
          <RangeSetting label="HDR Saturation" value={draft.hdr_adjust_saturation} disabled={disabled} onChange={(v) => change("hdr_adjust_saturation", v)} />
          {select("Ray Tracing", "is_raytracing_on", OFF_ON)}
        </SettingPanel>

        <SettingPanel eyebrow="NETWORK" title="Network Settings">
          {select("Cross-Region Play", "perform_matchmaking", MATCHMAKING_OPTIONS)}
          {select("Launch Setting", "launchsettings", LAUNCH_OPTIONS)}
          {select("Send Summon Sign", "send_summon_sign", SUMMON_OPTIONS)}
        </SettingPanel>
      </div>
    </>
  );
}
