import {
    useEffect,
    useMemo,
    useState,
} from "react";

import {
    cancelDownload,
    useDownloads,
} from "./downloadStore";

function formatBytes(
    bytes: number | null,
) {
    if (
        bytes === null ||
        !Number.isFinite(
            bytes,
        )
    ) {
        return "Unknown";
    }

    const units = [
        "B",
        "KB",
        "MB",
        "GB",
    ];

    let value =
    bytes;

    let unit =
    0;

    while (
        value >= 1024 &&
        unit <
        units.length - 1
    ) {
        value /=
        1024;

        unit +=
        1;
    }

    return `${value.toFixed(
        unit === 0
        ? 0
        : 1,
    )} ${units[unit]}`;
}

function formatSpeed(
    speed: number | null,
) {
    if (
        speed === null ||
        speed <= 0
    ) {
        return null;
    }

    return `${formatBytes(
        speed,
    )}/s`;
}

function formatEta(
    seconds: number | null,
) {
    if (
        seconds === null ||
        seconds < 0
    ) {
        return null;
    }

    const rounded =
    Math.ceil(
        seconds,
    );

    if (rounded < 60) {
        return `${rounded}s`;
    }

    const minutes =
    Math.floor(
        rounded / 60,
    );

    const remaining =
    rounded % 60;

    return `${minutes}m ${remaining}s`;
}

export default function DownloadProgress() {
    const items =
    useDownloads();

    const [
        hiddenId,
        setHiddenId,
    ] =
    useState<string | null>(
        null,
    );

    const download =
    useMemo(
        () =>
        items.find(
            (item) =>
            item.status ===
            "downloading" ||
            item.status ===
            "connecting" ||
            item.status ===
            "waiting_authorization" ||
            item.status ===
            "cancelling",
        ) ??
        items.find(
            (item) =>
            item.status ===
            "error" ||
            item.status ===
            "complete",
        ) ??
        null,
        [
            items,
        ],
    );

    useEffect(
        () => {
            if (
                !download ||
                download.status !== "complete"
            ) {
                return;
            }

            const completedId =
                download.id;

            const timer =
                window.setTimeout(
                    () => {
                        setHiddenId(
                            completedId,
                        );
                    },
                    3500,
                );

            return () => {
                window.clearTimeout(
                    timer,
                );
            };
        },
        [
            download?.id,
            download?.status,
        ],
    );

    if (
        !download ||
        download.id ===
        hiddenId
    ) {
        return null;
    }

    const percent =
    download.percent ===
    null
    ? null
    : Math.max(
        0,
        Math.min(
            100,
            download.percent,
        ),
    );

    const status =
    download.status ===
    "connecting"
    ? "Connecting to Nexus"
    : download.status ===
    "waiting_authorization"
    ? "Waiting for Nexus"
    : download.status ===
    "cancelling"
    ? "Cancelling download"
    : download.status ===
    "complete"
    ? "Download complete"
    : download.status ===
    "error"
    ? "Download failed"
    : "Downloading";

    const speed =
    formatSpeed(
        download.speedBytesPerSecond,
    );

    const eta =
    formatEta(
        download.etaSeconds,
    );

    return (
        <div
        className={`global-download ${download.status}`}
        >
        <div className="global-download-top">
        <div className="global-download-info">
        <span className="global-download-status">
        {status}
        </span>

        <strong>
        {download.fileName}
        </strong>
        </div>

        <div className="global-download-right">
        {percent !==
            null && (
                <span className="global-download-percent">
                {Math.round(
                    percent,
                )}
                %
                </span>
            )}

            <button
            className="global-download-close"
            onClick={() =>
                setHiddenId(
                    download.id,
                )
            }
            aria-label="Hide download progress"
            >
            ×
            </button>
            </div>
            </div>

            <div className="global-download-track">
            <div
            className={`global-download-bar ${
                percent ===
                null
                ? "indeterminate"
                : ""
            }`}
            style={
                percent ===
                null
                ? undefined
                : {
                    width:
                    `${percent}%`,
                }
            }
            />
            </div>

            <div className="global-download-bottom">
            {download.status ===
                "error" ? (
                    <span>
                    {download.error}
                    </span>
                ) : (
                    <>
                    <span>
                    {formatBytes(
                        download.downloadedBytes,
                    )}

                    {download.totalBytes !==
                        null
                        ? ` / ${formatBytes(
                            download.totalBytes,
                        )}`
                        : ""}
                        </span>

                        <span>
                        {speed
                            ? `${speed}${
                                eta
                                ? ` · ETA ${eta}`
                                : ""
                            }`
                            : "Nexus Mods"}
                            </span>
                            </>
                )}
                </div>

                {(
                    download.status ===
                    "connecting" ||
                    download.status ===
                    "downloading" ||
                    download.status ===
                    "waiting_authorization"
                ) && (
                    <div className="modal-actions">
                    <button
                    className="text-button"
                    onClick={() =>
                        void cancelDownload(
                            download,
                        )
                    }
                    >
                    Cancel
                    </button>
                    </div>
                )}
                </div>
    );
}
