import {
    useMemo,
} from "react";

import {
    cancelDownload,
    clearDownloadHistory,
    type DownloadItem,
    type DownloadStatus,
    removeDownload,
    retryDownload,
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
        !Number.isFinite(
            speed,
        ) ||
        speed <= 0
    ) {
        return "Calculating…";
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
        !Number.isFinite(
            seconds,
        ) ||
        seconds < 0
    ) {
        return "ETA —";
    }

    const rounded =
    Math.ceil(
        seconds,
    );

    if (rounded < 60) {
        return `ETA ${rounded}s`;
    }

    const minutes =
    Math.floor(
        rounded / 60,
    );

    const remaining =
    rounded % 60;

    if (minutes < 60) {
        return `ETA ${minutes}m ${remaining}s`;
    }

    const hours =
    Math.floor(
        minutes / 60,
    );

    const remainingMinutes =
    minutes % 60;

    return `ETA ${hours}h ${remainingMinutes}m`;
}

function statusText(
    status: DownloadStatus,
) {
    switch (status) {
        case "queued":
            return "QUEUED";

        case "waiting_authorization":
            return "WAITING FOR NEXUS";

        case "connecting":
            return "CONNECTING";

        case "downloading":
            return "DOWNLOADING";

        case "cancelling":
            return "CANCELLING";

        case "cancelled":
            return "CANCELLED";

        case "complete":
            return "COMPLETE";

        case "error":
            return "FAILED";
    }
}

function isActive(
    item: DownloadItem,
) {
    return (
        item.status ===
        "queued" ||
        item.status ===
        "waiting_authorization" ||
        item.status ===
        "connecting" ||
        item.status ===
        "downloading" ||
        item.status ===
        "cancelling"
    );
}

export default function DownloadsPage() {
    const items =
    useDownloads();

    const active =
    useMemo(
        () =>
        items.filter(
            isActive,
        ),
        [
            items,
        ],
    );

    const completed =
    useMemo(
        () =>
        items.filter(
            (item) =>
            item.status ===
            "complete",
        ),
        [
            items,
        ],
    );

    const failed =
    useMemo(
        () =>
        items.filter(
            (item) =>
            item.status ===
            "error" ||
            item.status ===
            "cancelled",
        ),
        [
            items,
        ],
    );

    const history =
    useMemo(
        () =>
        items.filter(
            (item) =>
            !isActive(
                item,
            ),
        ),
        [
            items,
        ],
    );

    return (
        <div className="downloads-page">
        <header className="downloads-header">
        <div>
        <div className="eyebrow">
        DOWNLOAD MANAGER
        </div>

        <h1>
        Downloads
        </h1>

        <p className="muted">
        Downloads continue to be tracked
        while you move around the manager.
            </p>
            </div>

            {history.length > 0 && (
                <button
                className="button secondary"
                onClick={
                    clearDownloadHistory
                }
                >
                Clear History
                </button>
            )}
            </header>

            <div className="downloads-summary">
            <SummaryCard
            label="ACTIVE"
            value={
                active.length
            }
            />

            <SummaryCard
            label="COMPLETED"
            value={
                completed.length
            }
            />

            <SummaryCard
            label="FAILED"
            value={
                failed.length
            }
            />
            </div>

            <DownloadSection
            title="Active Downloads"
            eyebrow="CURRENT"
            items={
                active
            }
            empty="No active downloads. Start an install from Browse Mods."
            />

            <DownloadSection
            title="Download History"
            eyebrow="RECENT"
            items={
                history
            }
            empty="Completed, cancelled, and failed downloads will appear here."
            />
            </div>
    );
}

function SummaryCard({
    label,
    value,
}: {
    label: string;
    value: number;
}) {
    return (
        <div className="downloads-stat">
        <span>
        {label}
        </span>

        <strong>
        {value}
        </strong>
        </div>
    );
}

function DownloadSection({
    title,
    eyebrow,
    items,
    empty,
}: {
    title: string;
    eyebrow: string;
    items: DownloadItem[];
    empty: string;
}) {
    return (
        <section className="downloads-section">
        <div className="downloads-section-title">
        <div>
        <div className="eyebrow">
        {eyebrow}
        </div>

        <h2>
        {title}
        </h2>
        </div>

        <span className="downloads-count">
        {items.length}
        </span>
        </div>

        {items.length ===
            0 ? (
                <div className="downloads-empty">
                {empty}
                </div>
            ) : (
                <div className="downloads-list">
                {items.map(
                    (item) => (
                        <DownloadCard
                        key={
                            item.id
                        }
                        item={
                            item
                        }
                        />
                    ),
                )}
                </div>
            )}
            </section>
    );
}

function DownloadCard({
    item,
}: {
    item: DownloadItem;
}) {
    const percent =
    item.percent ===
    null
    ? null
    : Math.max(
        0,
        Math.min(
            100,
            item.percent,
        ),
    );

    const active =
    isActive(
        item,
    );

    const canCancel =
    item.status ===
    "waiting_authorization" ||
    item.status ===
    "connecting" ||
    item.status ===
    "downloading";

    const canRetry =
    (
        item.status ===
        "error" ||
        item.status ===
        "cancelled"
    ) &&
    item.modId > 0 &&
    item.fileId > 0;

    return (
        <article
        className={`download-card ${item.status}`}
        >
        <div className="download-card-row">
        <div className="download-file-icon">
        {item.status ===
            "complete"
            ? "✓"
            : item.status ===
            "error"
            ? "!"
            : item.status ===
            "cancelled"
            ? "×"
            : item.status ===
            "queued"
            ? "…"
            : "↓"}
            </div>

            <div className="download-card-info">
            <strong>
            {item.fileName}
            </strong>

            <div className="download-meta">
            {item.modId >
                0 && (
                    <span>
                    Nexus Mod #
                    {item.modId}
                    </span>
                )}

                {item.fileId >
                    0 && (
                        <span>
                        File #
                        {item.fileId}
                        </span>
                    )}

                    <span>
                    {new Date(
                        item.updatedAt,
                    ).toLocaleString()}
                    </span>
                    </div>
                    </div>

                    <span
                    className={`download-status ${item.status}`}
                    >
                    {statusText(
                        item.status,
                    )}
                    </span>

                    {percent !==
                        null && (
                            <strong className="download-percent">
                            {Math.round(
                                percent,
                            )}
                            %
                            </strong>
                        )}
                        </div>

                        {active && (
                            <>
                            <div className="download-page-track">
                            <div
                            className={`download-page-bar ${
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

                            <div className="download-transfer">
                            <span>
                            {item.status ===
                                "waiting_authorization"
                                ? "Complete the Nexus Mods download authorization in your browser."
                                : item.status ===
                                "queued"
                                ? "Waiting for an available download slot…"
                                : item.status ===
                                "connecting"
                                ? "Connecting to Nexus…"
                                : item.status ===
                                "cancelling"
                                ? "Cancelling download…"
                                : `${formatBytes(
                                    item.downloadedBytes,
                                )}${
                                    item.totalBytes !==
                                    null
                                    ? ` / ${formatBytes(
                                        item.totalBytes,
                                    )}`
                                    : ""
                                }`}
                                </span>

                                {item.status ===
                                    "downloading" && (
                                        <span>
                                        {formatSpeed(
                                            item.speedBytesPerSecond,
                                        )}
                                        {" · "}
                                        {formatEta(
                                            item.etaSeconds,
                                        )}
                                        </span>
                                    )}
                                    </div>
                                    </>
                        )}

                        {item.status ===
                            "complete" && (
                                <div className="download-result success">
                                Download complete
                                </div>
                            )}

                            {item.status ===
                                "cancelled" && (
                                    <div className="download-result failure">
                                    Download cancelled
                                    </div>
                                )}

                                {item.status ===
                                    "error" &&
                                    item.error && (
                                        <div className="download-result failure">
                                        {item.error}
                                        </div>
                                    )}

                                    <div className="modal-actions">
                                    {canCancel && (
                                        <button
                                        className="button secondary"
                                        onClick={() =>
                                            void cancelDownload(
                                                item,
                                            )
                                        }
                                        >
                                        Cancel
                                        </button>
                                    )}

                                    {canRetry && (
                                        <button
                                        className="button primary"
                                        onClick={() =>
                                            void retryDownload(
                                                item,
                                            )
                                        }
                                        >
                                        Retry Download
                                        </button>
                                    )}

                                    {!active && (
                                        <button
                                        className="text-button"
                                        onClick={() =>
                                            removeDownload(
                                                item.id,
                                            )
                                        }
                                        >
                                        Remove
                                        </button>
                                    )}
                                    </div>
                                    </article>
    );
}
