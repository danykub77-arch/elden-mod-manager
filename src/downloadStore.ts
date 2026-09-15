import {
    useSyncExternalStore,
} from "react";

import {
    invoke,
} from "@tauri-apps/api/core";

import {
    listen,
    type UnlistenFn,
} from "@tauri-apps/api/event";

export type DownloadStatus =
| "queued"
| "waiting_authorization"
| "connecting"
| "downloading"
| "cancelling"
| "cancelled"
| "complete"
| "error";

export type DownloadItem = {
    id: string;

    modId: number;
    fileId: number;

    fileName: string;

    downloadedBytes: number;
    totalBytes: number | null;

    percent: number | null;

    status: DownloadStatus;

    archivePath: string | null;
    error: string | null;

    startedAt: number;
    updatedAt: number;

    speedBytesPerSecond: number | null;
    etaSeconds: number | null;
};

type ProgressEvent = {
    mod_id: number;
    file_id: number;

    file_name: string;

    downloaded_bytes: number;
    total_bytes: number | null;

    percent: number | null;

    state: string;
};

type CompleteEvent = {
    mod_id: number;
    file_id: number;
    archive_path: string;
};

type ErrorEvent = {
    mod_id: number | null;
    file_id: number | null;
    message: string;
};

type NexusAccountStatus = {
    connected: boolean;
    username: string | null;
    is_premium: boolean | null;
};

const STORAGE_KEY =
"elden-mod-manager.download-history.v2";

const OLD_STORAGE_KEY =
"elden-mod-manager.download-history.v1";

type Sample = {
    bytes: number;
    time: number;
};

const samples =
new Map<string, Sample>();

function downloadId(
    modId: number,
    fileId: number,
) {
    return `${modId}:${fileId}`;
}

function normalizeStoredItem(
    value: Partial<DownloadItem>,
): DownloadItem | null {
    if (
        typeof value.modId !== "number" ||
        typeof value.fileId !== "number"
    ) {
        return null;
    }

    const id =
    typeof value.id === "string"
    ? value.id
    : downloadId(
        value.modId,
        value.fileId,
    );

    return {
        id,

        modId:
        value.modId,

        fileId:
        value.fileId,

        fileName:
        typeof value.fileName === "string"
        ? value.fileName
        : "Nexus download",

        downloadedBytes:
        typeof value.downloadedBytes === "number"
        ? value.downloadedBytes
        : 0,

        totalBytes:
        typeof value.totalBytes === "number"
        ? value.totalBytes
        : null,

        percent:
        typeof value.percent === "number"
        ? value.percent
        : null,

        status:
        value.status ??
        "error",

        archivePath:
        typeof value.archivePath === "string"
        ? value.archivePath
        : null,

        error:
        typeof value.error === "string"
        ? value.error
        : null,

        startedAt:
        typeof value.startedAt === "number"
        ? value.startedAt
        : Date.now(),

        updatedAt:
        typeof value.updatedAt === "number"
        ? value.updatedAt
        : Date.now(),

        speedBytesPerSecond:
        null,

        etaSeconds:
        null,
    };
}

function loadStoredItems(): DownloadItem[] {
    try {
        let raw =
        localStorage.getItem(
            STORAGE_KEY,
        );

        if (!raw) {
            raw =
            localStorage.getItem(
                OLD_STORAGE_KEY,
            );
        }

        if (!raw) {
            return [];
        }

        const parsed =
        JSON.parse(
            raw,
        );

        if (!Array.isArray(parsed)) {
            return [];
        }

        return parsed
        .map(
            normalizeStoredItem,
        )
        .filter(
            (
                item,
            ): item is DownloadItem =>
            item !== null,
        );
    } catch {
        return [];
    }
}

let items: DownloadItem[] =
loadStoredItems();

let initialized =
false;

let initializing:
Promise<void> | null =
null;

const subscribers =
new Set<() => void>();

function persist() {
    try {
        localStorage.setItem(
            STORAGE_KEY,
            JSON.stringify(
                items.slice(
                    0,
                    100,
                ),
            ),
        );
    } catch {
        // Download history persistence is non-critical.
    }
}

function notify() {
    persist();

    for (
        const subscriber
        of subscribers
    ) {
        subscriber();
    }
}

function setItems(
    updater:
    | DownloadItem[]
    | ((
        current: DownloadItem[],
    ) => DownloadItem[]),
) {
    items =
    typeof updater === "function"
    ? updater(items)
    : updater;

    notify();
}

function upsert(
    next: DownloadItem,
) {
    setItems(
        (current) => [
            next,

             ...current.filter(
                 (item) =>
                 item.id !==
                 next.id,
             ),
        ],
    );
}

function calculateTransfer(
    id: string,
    downloadedBytes: number,
    totalBytes: number | null,
    now: number,
) {
    const previous =
    samples.get(
        id,
    );

    samples.set(
        id,
        {
            bytes:
            downloadedBytes,

            time:
            now,
        },
    );

    if (!previous) {
        return {
            speed:
            null,

            eta:
            null,
        };
    }

    const elapsed =
    (
        now -
        previous.time
    ) / 1000;

    const delta =
    downloadedBytes -
    previous.bytes;

    if (
        elapsed <= 0 ||
        delta <= 0
    ) {
        return {
            speed:
            null,

            eta:
            null,
        };
    }

    const instantSpeed =
    delta /
    elapsed;

    const old =
    items.find(
        (item) =>
        item.id ===
        id,
    );

    const speed =
    old?.speedBytesPerSecond
    ? (
        old.speedBytesPerSecond *
        0.7
    ) +
    (
        instantSpeed *
        0.3
    )
    : instantSpeed;

    const remaining =
    totalBytes === null
    ? null
    : Math.max(
        0,
        totalBytes -
        downloadedBytes,
    );

    const eta =
    remaining === null ||
    speed <= 0
    ? null
    : remaining /
    speed;

    return {
        speed,
        eta,
    };
}

function progressStatus(
    state: string,
): DownloadStatus {
    switch (state) {
        case "queued":
            return "queued";

        case "waiting_authorization":
            return "waiting_authorization";

        case "connecting":
            return "connecting";

        case "cancelling":
            return "cancelling";

        case "cancelled":
            return "cancelled";

        case "complete":
            return "complete";

        default:
            return "downloading";
    }
}

function handleProgress(
    payload: ProgressEvent,
) {
    const id =
    downloadId(
        payload.mod_id,
        payload.file_id,
    );

    const now =
    Date.now();

    const previous =
    items.find(
        (item) =>
        item.id ===
        id,
    );

    const status =
    progressStatus(
        payload.state,
    );

    const transfer =
    status ===
    "downloading"
    ? calculateTransfer(
        id,
        payload.downloaded_bytes,
        payload.total_bytes,
        now,
    )
    : {
        speed:
        previous
        ?.speedBytesPerSecond ??
        null,

        eta:
        previous
        ?.etaSeconds ??
        null,
    };

    upsert({
        id,

        modId:
        payload.mod_id,

        fileId:
        payload.file_id,

        fileName:
        payload.file_name,

        downloadedBytes:
        payload.downloaded_bytes,

        totalBytes:
        payload.total_bytes,

        percent:
        payload.percent,

        status,

        archivePath:
        previous
        ?.archivePath ??
        null,

        error:
        null,

        startedAt:
        previous
        ?.startedAt ??
        now,

        updatedAt:
        now,

        speedBytesPerSecond:
        status ===
        "complete"
        ? null
        : transfer.speed,

        etaSeconds:
        status ===
        "complete"
        ? null
        : displayedEta(
            id,
            transfer.eta,
            now,
        ),
    });
}

function handleComplete(
    payload: CompleteEvent,
) {
    const id =
    downloadId(
        payload.mod_id,
        payload.file_id,
    );

    const now =
    Date.now();

    const previous =
    items.find(
        (item) =>
        item.id ===
        id,
    );

    if (!previous) {
        upsert({
            id,

            modId:
            payload.mod_id,

            fileId:
            payload.file_id,

            fileName:
            "Nexus download",

            downloadedBytes:
            0,

            totalBytes:
            null,

            percent:
            100,

            status:
            "complete",

            archivePath:
            payload.archive_path,

            error:
            null,

            startedAt:
            now,

            updatedAt:
            now,

            speedBytesPerSecond:
            null,

            etaSeconds:
            null,
        });

        return;
    }

    upsert({
        ...previous,

        percent:
        100,

        status:
        "complete",

        archivePath:
        payload.archive_path,

        error:
        null,

        updatedAt:
        now,

        speedBytesPerSecond:
        null,

        etaSeconds:
        null,
    });

    samples.delete(
        id,
    );
}

function handleError(
    payload: ErrorEvent,
) {
    /*
     * A cancellation already has a dedicated state.
     * Free-user NXM downloads may emit the normal
     * error event after the cancellation bubbles up,
     * so don't turn CANCELLED back into FAILED.
     */
    if (
        payload.message
        .toLowerCase()
        .includes(
            "cancelled",
        )
    ) {
        const matching =
        items.find(
            (item) =>
            (
                payload.mod_id ===
                null ||
                item.modId ===
                payload.mod_id
            ) &&
            (
                payload.file_id ===
                null ||
                item.fileId ===
                payload.file_id
            ),
        );

        if (
            matching?.status ===
            "cancelled"
        ) {
            return;
        }
    }

    if (
        payload.mod_id ===
        null ||
        payload.file_id ===
        null
    ) {
        const now =
        Date.now();

        upsert({
            id:
            `error:${now}`,

            modId:
            payload.mod_id ??
            0,

            fileId:
            payload.file_id ??
            0,

            fileName:
            "Nexus download",

            downloadedBytes:
            0,

            totalBytes:
            null,

            percent:
            null,

            status:
            "error",

            archivePath:
            null,

            error:
            payload.message,

            startedAt:
            now,

            updatedAt:
            now,

            speedBytesPerSecond:
            null,

            etaSeconds:
            null,
        });

        return;
    }

    const id =
    downloadId(
        payload.mod_id,
        payload.file_id,
    );

    const previous =
    items.find(
        (item) =>
        item.id ===
        id,
    );

    if (!previous) {
        const now =
        Date.now();

        upsert({
            id,

            modId:
            payload.mod_id,

            fileId:
            payload.file_id,

            fileName:
            "Nexus download",

            downloadedBytes:
            0,

            totalBytes:
            null,

            percent:
            null,

            status:
            "error",

            archivePath:
            null,

            error:
            payload.message,

            startedAt:
            now,

            updatedAt:
            now,

            speedBytesPerSecond:
            null,

            etaSeconds:
            null,
        });

        return;
    }

    upsert({
        ...previous,

        status:
        "error",

        error:
        payload.message,

        updatedAt:
        Date.now(),

           speedBytesPerSecond:
           null,

           etaSeconds:
           null,
    });

    samples.delete(
        id,
    );
}

const ETA_DISPLAY_INTERVAL_MS = 5_000;

const etaDisplaySnapshots = new Map<
    string,
    {
        eta: number | null;
        updatedAt: number;
    }
>();

function displayedEta(
    id: string,
    calculatedEta: number | null,
    now: number,
): number | null {
    const previous =
        etaDisplaySnapshots.get(
            id,
        );

    if (
        !previous ||
        now - previous.updatedAt >=
            ETA_DISPLAY_INTERVAL_MS
    ) {
        etaDisplaySnapshots.set(
            id,
            {
                eta:
                    calculatedEta,
                updatedAt:
                    now,
            },
        );

        return calculatedEta;
    }

    return previous.eta;
}


export async function initializeDownloadStore() {
    if (initialized) {
        return;
    }

    if (initializing) {
        return initializing;
    }

    initializing =
    (async () => {
        const listeners:
        UnlistenFn[] = [];

        listeners.push(
            await listen<ProgressEvent>(
                "nexus-download-progress",
                (event) => {
                    handleProgress(
                        event.payload,
                    );
                },
            ),
        );

        listeners.push(
            await listen<CompleteEvent>(
                "nexus-download-complete",
                (event) => {
                    handleComplete(
                        event.payload,
                    );
                },
            ),
        );

        listeners.push(
            await listen<ErrorEvent>(
                "nexus-download-error",
                (event) => {
                    handleError(
                        event.payload,
                    );
                },
            ),
        );

        /*
         * Keep these listeners alive for the lifetime
         * of the app. They are deliberately global.
         */
        void listeners;

        initialized =
        true;
    })();

    try {
        await initializing;
    } finally {
        initializing =
        null;
    }
}

function subscribe(
    listener: () => void,
) {
    subscribers.add(
        listener,
    );

    return () => {
        subscribers.delete(
            listener,
        );
    };
}

function snapshot() {
    return items;
}

export function useDownloads() {
    return useSyncExternalStore(
        subscribe,
        snapshot,
        snapshot,
    );
}

export function clearDownloadHistory() {
    setItems(
        (current) =>
        current.filter(
            (item) =>
            item.status ===
            "queued" ||
            item.status ===
            "waiting_authorization" ||
            item.status ===
            "connecting" ||
            item.status ===
            "downloading" ||
            item.status ===
            "cancelling",
        ),
    );
}

export function removeDownload(
    id: string,
) {
    setItems(
        (current) =>
        current.filter(
            (item) =>
            item.id !==
            id,
        ),
    );
}

export async function cancelDownload(
    item: DownloadItem,
) {
    if (
        item.status !==
        "connecting" &&
        item.status !==
        "downloading" &&
        item.status !==
        "waiting_authorization"
    ) {
        return;
    }

    upsert({
        ...item,

        status:
        "cancelling",

        updatedAt:
        Date.now(),
    });

    try {
        await invoke(
            "cancel_nexus_download",
            {
                modId:
                item.modId,

                fileId:
                item.fileId,
            },
        );
    } catch (reason) {
        upsert({
            ...item,

            status:
            "error",

            error:
            String(
                reason,
            ),

            updatedAt:
            Date.now(),

               speedBytesPerSecond:
               null,

               etaSeconds:
               null,
        });
    }
}

export async function retryDownload(
    item: DownloadItem,
) {
    if (
        item.modId <= 0 ||
        item.fileId <= 0
    ) {
        return;
    }

    const now =
    Date.now();

    upsert({
        ...item,

        downloadedBytes:
        0,

        percent:
        0,

        status:
        "connecting",

        archivePath:
        null,

        error:
        null,

        startedAt:
        now,

        updatedAt:
        now,

        speedBytesPerSecond:
        null,

        etaSeconds:
        null,
    });

    samples.delete(
        item.id,
    );

    try {
        const account =
        await invoke<NexusAccountStatus>(
            "get_nexus_account",
        );

        if (!account.connected) {
            throw new Error(
                "Connect Nexus Mods before retrying this download.",
            );
        }

        if (
            account.is_premium
        ) {
            const archivePath =
            await invoke<string>(
                "download_nexus_mod_file",
                {
                    modId:
                    item.modId,

                    fileId:
                    item.fileId,
                },
            );

            handleComplete({
                mod_id:
                item.modId,

                file_id:
                item.fileId,

                archive_path:
                archivePath,
            });

            return;
        }

        upsert({
            ...item,

            downloadedBytes:
            0,

            percent:
            0,

            status:
            "waiting_authorization",

            archivePath:
            null,

            error:
            null,

            startedAt:
            now,

            updatedAt:
            Date.now(),

               speedBytesPerSecond:
               null,

               etaSeconds:
               null,
        });

        await invoke(
            "open_nexus_download_authorization",
            {
                modId:
                item.modId,

                fileId:
                item.fileId,
            },
        );
    } catch (reason) {
        handleError({
            mod_id:
            item.modId,

            file_id:
            item.fileId,

            message:
            String(
                reason,
            ),
        });
    }
}
