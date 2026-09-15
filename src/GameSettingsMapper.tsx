import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type BaselineStatus = {
  baseline_exists: boolean;
};

type SettingChange = {
  key: string;
  label: string;
  before: number;
  after: number;
};

type SettingsComparison = {
  baseline_exists: boolean;
  change_count: number;
  changes: SettingChange[];
};

export default function GameSettingsMapper() {
  const [baselineExists, setBaselineExists] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [comparison, setComparison] =
    useState<SettingsComparison | null>(null);

  async function refreshStatus() {
    try {
      const result = await invoke<BaselineStatus>(
        "get_game_settings_baseline_status",
      );

      setBaselineExists(result.baseline_exists);
    } catch {
      // Optional diagnostic feature.
    }
  }

  useEffect(() => {
    refreshStatus();
  }, []);

  async function captureBaseline() {
    setBusy(true);
    setError(null);
    setMessage(null);
    setComparison(null);

    try {
      const result = await invoke<BaselineStatus>(
        "capture_game_settings_baseline",
      );

      setBaselineExists(result.baseline_exists);

      setMessage(
        "Baseline captured. Change one setting in Elden Ring, let the game save it, then compare.",
      );
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }

  async function compareBaseline() {
    setBusy(true);
    setError(null);
    setMessage(null);

    try {
      const result = await invoke<SettingsComparison>(
        "compare_game_settings_baseline",
      );

      setComparison(result);
      setBaselineExists(result.baseline_exists);

      if (!result.baseline_exists) {
        setMessage("Capture a baseline first.");
      } else if (result.change_count === 0) {
        setMessage(
          "No setting changes detected. Elden Ring may not have written the new value yet.",
        );
      } else if (result.change_count === 1) {
        setMessage(
          "Exactly one setting changed. This is a clean mapping result.",
        );
      } else {
        setMessage(
          `${result.change_count} settings changed. Change only one option for a clean mapping.`,
        );
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }

  async function clearBaseline() {
    setBusy(true);
    setError(null);
    setMessage(null);
    setComparison(null);

    try {
      await invoke("clear_game_settings_baseline");

      setBaselineExists(false);
      setMessage("Baseline cleared.");
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel settings-mapper-panel">
      <div className="eyebrow">
        SETTINGS MAPPER
      </div>

      <h2>
        Verify Setting Values
      </h2>

      <p className="muted">
        Compare two read-only snapshots of Elden Ring settings.
        This tool does not modify your save.
      </p>

      <div className="settings-mapper-steps">
        <div>
          <b>1</b>
          <span>Capture the current settings.</span>
        </div>

        <div>
          <b>2</b>
          <span>
            Change exactly one setting in Elden Ring and let
            the game save it.
          </span>
        </div>

        <div>
          <b>3</b>
          <span>Compare it with the baseline.</span>
        </div>
      </div>

      <div className="settings-mapper-actions">
        <button
          className="button secondary"
          disabled={busy}
          onClick={captureBaseline}
        >
          {baselineExists
            ? "Replace Baseline"
            : "Capture Baseline"}
        </button>

        <button
          className="button primary"
          disabled={busy || !baselineExists}
          onClick={compareBaseline}
        >
          Compare With Baseline
        </button>

        <button
          className="button secondary"
          disabled={busy || !baselineExists}
          onClick={clearBaseline}
        >
          Clear
        </button>
      </div>

      {baselineExists && (
        <div className="settings-mapper-baseline">
          Baseline Ready
        </div>
      )}

      {message && (
        <div className="settings-mapper-message">
          {message}
        </div>
      )}

      {error && (
        <div className="settings-mapper-message error">
          {error}
        </div>
      )}

      {comparison && comparison.changes.length > 0 && (
        <div className="settings-mapper-results">
          {comparison.changes.map((change) => (
            <div
              className="settings-mapper-result"
              key={change.key}
            >
              <div className="settings-mapper-name">
                <strong>{change.label}</strong>
                <span>{change.key}</span>
              </div>

              <div className="settings-mapper-diff">
                <b>{change.before}</b>
                <span>-&gt;</span>
                <b>{change.after}</b>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
