import {
  useEffect,
  useRef,
  useState,
} from "react";

import {
  invoke,
} from "@tauri-apps/api/core";

import {
  listen,
} from "@tauri-apps/api/event";

type DownloadProgress = {
  mod_id: number;
  file_id: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  percent: number | null;
  state: string;
};

type DownloadComplete = {
  mod_id: number;
  file_id: number;
  archive_path: string;
};

type DownloadError = {
  mod_id: number | null;
  file_id: number | null;
  message: string;
};

type NexusAccount = {
  connected: boolean;
  username: string | null;
  is_premium: boolean;
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

type PendingChoice = {
  analysis: ModArchiveAnalysis;
};

type Props = {
  profileId: string;
  localModId: string;

  nexusModId: number;
  nexusFileId: number;

  nexusVersion: string | null;

  onUpdated?: () => void | Promise<void>;
};

function formatBytes(bytes: number | null) {
  if (bytes === null || bytes <= 0) {
    return "";
  }

  const units = [
    "B",
    "KB",
    "MB",
    "GB",
  ];

  let value = bytes;
  let index = 0;

  while (
    value >= 1024 &&
    index < units.length - 1
  ) {
    value /= 1024;
    index += 1;
  }

  return `${value.toFixed(
    index === 0 ? 0 : 1,
  )} ${units[index]}`;
}

export default function NexusUpdateButton({
  profileId,
  localModId,
  nexusModId,
  nexusFileId,
  nexusVersion,
  onUpdated,
}: Props) {
  const processedArchives =
    useRef<Set<string>>(
      new Set(),
    );

  const [
    active,
    setActive,
  ] = useState(false);

  const [
    status,
    setStatus,
  ] = useState<string | null>(
    null,
  );

  const [
    error,
    setError,
  ] = useState<string | null>(
    null,
  );

  const [
    progress,
    setProgress,
  ] = useState<number | null>(
    null,
  );

  const [
    pendingChoice,
    setPendingChoice,
  ] =
    useState<PendingChoice | null>(
      null,
    );

  async function finalizeUpdate(
    analysis: ModArchiveAnalysis,
    variant: ModArchiveVariant,
  ) {
    setError(null);

    setStatus(
      "Installing update...",
    );

    await invoke(
      "replace_nexus_mod_variant",
      {
        profileId,
        oldModId:
          localModId,

        archivePath:
          analysis.archive_path,

        variantPath:
          variant.relative_path,

        variantName:
          variant.name,

        multipleVariants:
          analysis.variants.length > 1,

        nexusModId,

        nexusFileId,

        nexusVersion,
      },
    );

    setPendingChoice(
      null,
    );

    setProgress(
      100,
    );

    setStatus(
      "Done ✓",
    );

    setActive(
      false,
    );

    window.dispatchEvent(
      new CustomEvent(
        "elden-mod-manager:mods-updated",
      ),
    );

    if (onUpdated) {
      await onUpdated();
    }
  }

  async function processArchive(
    archivePath: string,
  ) {
    if (
      processedArchives.current.has(
        archivePath,
      )
    ) {
      return;
    }

    processedArchives.current.add(
      archivePath,
    );

    try {
      setError(
        null,
      );

      setStatus(
        "Analyzing update...",
      );

      const analysis =
        await invoke<ModArchiveAnalysis>(
          "analyze_mod_archive",
          {
            archivePath,
          },
        );

      if (
        analysis.variants.length === 0
      ) {
        throw new Error(
          "The downloaded update does not contain a supported Elden Ring mod.",
        );
      }

      if (
        analysis.variants.length === 1
      ) {
        await finalizeUpdate(
          analysis,
          analysis.variants[0],
        );

        return;
      }

      setPendingChoice({
        analysis,
      });

      setStatus(
        "Choose the update variant.",
      );
    } catch (reason) {
      processedArchives.current.delete(
        archivePath,
      );

      console.error(
        reason,
      );

      setError(
        String(reason),
      );

      setStatus(
        null,
      );

      setActive(
        false,
      );
    }
  }

  useEffect(
    () => {
      if (!active) {
        return;
      }

      let disposed =
        false;

      const unlisteners:
        Array<() => void> =
        [];

      async function attach() {
        const stopProgress =
          await listen<DownloadProgress>(
            "nexus-download-progress",
            (event) => {
              const payload =
                event.payload;

              if (
                payload.mod_id !==
                  nexusModId ||
                payload.file_id !==
                  nexusFileId
              ) {
                return;
              }

              if (
                payload.percent !==
                null
              ) {
                setProgress(
                  Math.max(
                    0,
                    Math.min(
                      100,
                      payload.percent,
                    ),
                  ),
                );
              }

              if (
                payload.state ===
                "connecting"
              ) {
                setStatus(
                  "Connecting to Nexus...",
                );
              } else if (
                payload.state ===
                "downloading"
              ) {
                const downloaded =
                  formatBytes(
                    payload.downloaded_bytes,
                  );

                const total =
                  formatBytes(
                    payload.total_bytes,
                  );

                setStatus(
                  total
                    ? `Downloading ${downloaded} / ${total}`
                    : `Downloading ${downloaded}`,
                );
              }
            },
          );

        if (disposed) {
          stopProgress();
        } else {
          unlisteners.push(
            stopProgress,
          );
        }

        const stopComplete =
          await listen<DownloadComplete>(
            "nexus-download-complete",
            (event) => {
              const payload =
                event.payload;

              if (
                payload.mod_id !==
                  nexusModId ||
                payload.file_id !==
                  nexusFileId
              ) {
                return;
              }

              setProgress(
                100,
              );

              void processArchive(
                payload.archive_path,
              );
            },
          );

        if (disposed) {
          stopComplete();
        } else {
          unlisteners.push(
            stopComplete,
          );
        }

        const stopError =
          await listen<DownloadError>(
            "nexus-download-error",
            (event) => {
              const payload =
                event.payload;

              if (
                payload.mod_id !==
                  null &&
                payload.mod_id !==
                  nexusModId
              ) {
                return;
              }

              if (
                payload.file_id !==
                  null &&
                payload.file_id !==
                  nexusFileId
              ) {
                return;
              }

              setError(
                payload.message,
              );

              setStatus(
                null,
              );

              setActive(
                false,
              );
            },
          );

        if (disposed) {
          stopError();
        } else {
          unlisteners.push(
            stopError,
          );
        }
      }

      void attach();

      return () => {
        disposed =
          true;

        for (
          const unlisten
          of unlisteners
        ) {
          unlisten();
        }
      };
    },
    [
      active,
      nexusModId,
      nexusFileId,
    ],
  );

  async function beginUpdate() {
    setError(
      null,
    );

    setPendingChoice(
      null,
    );

    setProgress(
      0,
    );

    setActive(
      true,
    );

    try {
      const account =
        await invoke<NexusAccount>(
          "get_nexus_account",
        );

      if (
        !account.connected
      ) {
        throw new Error(
          "Connect your Nexus Mods account before updating.",
        );
      }

      if (
        account.is_premium
      ) {
        setStatus(
          "Starting Premium download...",
        );

        const archivePath =
          await invoke<string>(
            "download_nexus_mod_file",
            {
              modId:
                nexusModId,

              fileId:
                nexusFileId,
            },
          );

        /*
         * Premium downloads return the completed
         * archive path directly. Process it here.
         *
         * The completion event may also arrive, but
         * processArchive() is guarded against handling
         * the same archive twice.
         */
        await processArchive(
          archivePath,
        );
      } else {
        setStatus(
          "Waiting for Nexus authorization...",
        );

        await invoke(
          "open_nexus_download_authorization",
          {
            modId:
              nexusModId,

            fileId:
              nexusFileId,
          },
        );
      }
    } catch (reason) {
      console.error(
        reason,
      );

      setError(
        String(reason),
      );

      setStatus(
        null,
      );

      setActive(
        false,
      );
    }
  }

  return (
    <div
      className="nexus-update-control"
      onClick={(event) =>
        event.stopPropagation()
      }
    >
      <button
        type="button"
        className="mod-update-real-button"
        disabled={active}
        onClick={() =>
          void beginUpdate()
        }
      >
        {active
          ? "UPDATING..."
          : "UPDATE"}
      </button>

      {status && (
        <div className="nexus-update-status">
          {status}
        </div>
      )}

      {active &&
       progress !== null && (
        <div className="nexus-update-progress">
          <div
            className="nexus-update-progress-fill"
            style={{
              width:
                `${progress}%`,
            }}
          />
        </div>
      )}

      {pendingChoice && (
        <div className="nexus-update-variants">
          <strong>
            Choose update variant
          </strong>

          {pendingChoice
            .analysis
            .variants
            .map(
              (variant) => (
                <button
                  key={
                    variant.id
                  }
                  type="button"
                  className="button secondary"
                  onClick={() =>
                    void finalizeUpdate(
                      pendingChoice.analysis,
                      variant,
                    )
                  }
                >
                  {variant.name}
                </button>
              ),
            )}
        </div>
      )}

      {error && (
        <div className="nexus-error">
          {error}
        </div>
      )}
    </div>
  );
}
