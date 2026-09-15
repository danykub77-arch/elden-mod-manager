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

type NexusFile = {
    file_id: number;

    name: string;
    file_name: string;

    version: string;

    category_name: string;

    is_primary: boolean;

    size_bytes: number | null;

    uploaded_timestamp:
    number | null;

    description: string;

    supported_archive: boolean;
};

type DownloadProgress = {
    mod_id: number;
    file_id: number;

    file_name: string;

    downloaded_bytes: number;

    total_bytes:
    number | null;

    percent:
    number | null;

    state: string;
};

type DownloadComplete = {
    mod_id: number;
    file_id: number;

    archive_path: string;
};

type DownloadError = {
    mod_id:
    number | null;

    file_id:
    number | null;

    message: string;
};

type ProfileConfig = {
    selected_profile:
    string | null;
};

type ModArchiveVariant = {
    id: string;
    name: string;

    relative_path: string;
};

type ModArchiveAnalysis = {
    archive_path: string;

    display_name: string;

    variants:
    ModArchiveVariant[];
};

type PendingVariantChoice = {
    analysis:
    ModArchiveAnalysis;

    profileId: string;
};

type Props = {
    modId: number;

    modName: string;

    isPremium: boolean;
};

type InstalledModResult = {
    id: string;
};

function formatBytes(
    bytes: number | null,
) {
    if (
        bytes === null ||
        bytes <= 0
    ) {
        return "Unknown size";
    }

    const units = [
        "B",
        "KB",
        "MB",
        "GB",
    ];

    let value =
    bytes;

    let index =
    0;

    while (
        value >= 1024 &&
        index <
        units.length - 1
    ) {
        value /=
        1024;

        index +=
        1;
    }

    return `${value.toFixed(
        index === 0
        ? 0
        : 1,
    )} ${units[index]}`;
}

export default function NexusInstallButton({
    modId,
    modName,
    isPremium,
}: Props) {
    const [
        files,
        setFiles,
    ] =
    useState<NexusFile[] | null>(
        null,
    );

    const installMenuRef =
    useRef<HTMLDivElement | null>(
        null,
    );

    useEffect(
        () => {
            if (files === null) {
                return;
            }

            function handlePointerDown(
                event: PointerEvent,
            ) {
                const target =
                event.target;

                if (
                    target instanceof Node &&
                    installMenuRef.current &&
                    !installMenuRef.current.contains(
                        target,
                    )
                ) {
                    setFiles(
                        null,
                    );
                }
            }

            function handleKeyDown(
                event: KeyboardEvent,
            ) {
                if (
                    event.key === "Escape"
                ) {
                    setFiles(
                        null,
                    );
                }
            }

            document.addEventListener(
                "pointerdown",
                handlePointerDown,
            );

            document.addEventListener(
                "keydown",
                handleKeyDown,
            );

            return () => {
                document.removeEventListener(
                    "pointerdown",
                    handlePointerDown,
                );

                document.removeEventListener(
                    "keydown",
                    handleKeyDown,
                );
            };
        },
        [
            files,
        ],
    );

    const [
        loadingFiles,
        setLoadingFiles,
    ] =
    useState(false);

    const [
        activeFileId,
        setActiveFileId,
    ] =
    useState<number | null>(
        null,
    );

    const [
        status,
        setStatus,
    ] =
    useState<string | null>(
        null,
    );

    const [
        progress,
        setProgress,
    ] =
    useState<number | null>(
        null,
    );

    const [
        error,
        setError,
    ] =
    useState<string | null>(
        null,
    );

    const [
        pendingVariants,
        setPendingVariants,
    ] =
    useState<
    PendingVariantChoice |
    null
    >(
        null,
    );

    const [
        installedOnProfile,
        setInstalledOnProfile,
    ] =
    useState(
        false,
    );

    const [
        checkingInstalled,
        setCheckingInstalled,
    ] =
    useState(
        true,
    );

    async function getSelectedProfile() {
        const profiles =
        await invoke<ProfileConfig>(
            "get_profiles",
        );

        if (
            !profiles
            .selected_profile
        ) {
            throw new Error(
                "Select a mod profile before installing a Nexus mod.",
            );
        }

        return profiles
        .selected_profile;
    }

    async function refreshInstalledState() {
        setCheckingInstalled(
            true,
        );

        try {
            const profiles =
            await invoke<ProfileConfig>(
                "get_profiles",
            );

            if (
                !profiles
                .selected_profile
            ) {
                setInstalledOnProfile(
                    false,
                );

                return;
            }

            const installed =
            await invoke<boolean>(
                "is_nexus_mod_installed",
                {
                    profileId:
                    profiles
                    .selected_profile,

                    nexusModId:
                    modId,
                },
            );

            setInstalledOnProfile(
                installed,
            );
        } catch (reason) {
            console.error(
                "Could not check Nexus install state:",
                reason,
            );

            setInstalledOnProfile(
                false,
            );
        } finally {
            setCheckingInstalled(
                false,
            );
        }
    }

    useEffect(
        () => {
            void refreshInstalledState();
        },
        [
            modId,
        ],
    );

    async function installVariant(
        analysis:
        ModArchiveAnalysis,

        profileId: string,

        variant:
        ModArchiveVariant,
    ) {
        setError(
            null,
        );

        setStatus(
            `Installing ${modName}...`,
        );

        const installed =
        await invoke<InstalledModResult>(
            "import_mod_variant",
            {
                profileId,

                archivePath:
                analysis.archive_path,

                variantPath:
                variant.relative_path,

                variantName:
                variant.name,

                multipleVariants:
                analysis
                .variants
                .length > 1,
            },
        );

        if (
            activeFileId === null
        ) {
            throw new Error(
                "The Nexus file ID was lost before installation completed.",
            );
        }

        const installedFile =
        files?.find(
            (file) =>
            file.file_id ===
            activeFileId,
        );

        await invoke(
            "record_nexus_mod_install",
            {
                profileId,

                installedModId:
                installed.id,

                nexusModId:
                modId,

                nexusFileId:
                activeFileId,

                nexusVersion:
                installedFile
                ?.version ||
                null,
            },
        );

        setInstalledOnProfile(
            true,
        );

        setPendingVariants(
            null,
        );

        setProgress(
            100,
        );

        setStatus(
            "Done ✓",
        );

        setActiveFileId(
            null,
        );
    }

    async function processDownloadedArchive(
        archivePath: string,
    ) {
        try {
            setError(
                null,
            );

            setStatus(
                "Extracting & analyzing archive...",
            );

            const profileId =
            await getSelectedProfile();

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
                    "The downloaded archive does not contain a supported Elden Ring mod.",
                );
            }

            if (
                analysis
                .variants
                .length === 1
            ) {
                await installVariant(
                    analysis,

                    profileId,

                    analysis
                    .variants[0],
                );

                return;
            }

            setPendingVariants({
                analysis,
                profileId,
            });

            setStatus(
                "Choose the version you want to install.",
            );
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

            setActiveFileId(
                null,
            );
        }
    }

    useEffect(
        () => {
            if (
                activeFileId === null
            ) {
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
                    (
                        event,
                    ) => {
                        const payload =
                        event.payload;

                        if (
                            payload.mod_id !==
                            modId ||
                            payload.file_id !==
                            activeFileId
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
                            setStatus(
                                `Downloading ${formatBytes(
                                    payload.downloaded_bytes,
                                )}${
                                    payload.total_bytes
                                    ? ` / ${formatBytes(
                                        payload.total_bytes,
                                    )}`
                                    : ""
                                }`,
                            );
                        }
                    },
                );

                if (
                    disposed
                ) {
                    stopProgress();
                } else {
                    unlisteners.push(
                        stopProgress,
                    );
                }

                const stopComplete =
                await listen<DownloadComplete>(
                    "nexus-download-complete",
                    (
                        event,
                    ) => {
                        const payload =
                        event.payload;

                        if (
                            payload.mod_id !==
                            modId ||
                            payload.file_id !==
                            activeFileId
                        ) {
                            return;
                        }

                        setProgress(
                            100,
                        );

                        void processDownloadedArchive(
                            payload.archive_path,
                        );
                    },
                );

                if (
                    disposed
                ) {
                    stopComplete();
                } else {
                    unlisteners.push(
                        stopComplete,
                    );
                }

                const stopError =
                await listen<DownloadError>(
                    "nexus-download-error",
                    (
                        event,
                    ) => {
                        const payload =
                        event.payload;

                        if (
                            payload.mod_id !==
                            null &&
                            payload.mod_id !==
                            modId
                        ) {
                            return;
                        }

                        if (
                            payload.file_id !==
                            null &&
                            payload.file_id !==
                            activeFileId
                        ) {
                            return;
                        }

                        setError(
                            payload.message,
                        );

                        setStatus(
                            null,
                        );

                        setActiveFileId(
                            null,
                        );
                    },
                );

                if (
                    disposed
                ) {
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
            activeFileId,
            modId,
        ],
    );

    async function loadFiles() {
        if (
            files !== null
        ) {
            setFiles(
                null,
            );

            return;
        }

        setLoadingFiles(
            true,
        );

        setError(
            null,
        );

        try {
            const result =
            await invoke<NexusFile[]>(
                "get_nexus_mod_files",
                {
                    modId,
                },
            );

            setFiles(
                result,
            );

            if (
                result.length === 0
            ) {
                setError(
                    "Nexus did not return any downloadable files for this mod.",
                );
            }
        } catch (reason) {
            console.error(
                reason,
            );

            setError(
                String(reason),
            );
        } finally {
            setLoadingFiles(
                false,
            );
        }
    }

    async function beginDownload(
        file: NexusFile,
    ) {
        if (
            !file.supported_archive
        ) {
            return;
        }

        setActiveFileId(
            file.file_id,
        );

        setProgress(
            0,
        );

        setError(
            null,
        );

        setPendingVariants(
            null,
        );

        try {
            /*
             * Check before starting so we don't
             * download a file and only then discover
             * that there is nowhere to install it.
             */
            await getSelectedProfile();

            if (
                isPremium
            ) {
                setStatus(
                    "Starting Premium download...",
                );

                const archivePath =
                await invoke<string>(
                    "download_nexus_mod_file",
                    {
                        modId,

                        fileId:
                        file.file_id,
                    },
                );

                await processDownloadedArchive(
                    archivePath,
                );
            } else {
                setStatus(
                    "Waiting for Nexus authorization...",
                );

                await invoke(
                    "open_nexus_download_authorization",
                    {
                        modId,

                        fileId:
                        file.file_id,
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

            setActiveFileId(
                null,
            );
        }
    }

    return (
        <div
        ref={
            installMenuRef
        }
        style={{
            display:
            "grid",

            gap:
            "8px",

            width:
            "100%",
        }}
        >
        <button
        className="button primary browse-open-button"
        disabled={
            loadingFiles ||
            activeFileId !== null ||
            checkingInstalled ||
            installedOnProfile
        }
        onClick={
            loadFiles
        }
        >
        {checkingInstalled
            ? "Checking..."
            : installedOnProfile
            ? "✓ Already installed on this profile"
            : loadingFiles
            ? "Loading files..."
            : files
            ? "Close Files"
            : "Install"}
            </button>

            {files && (
                <div
                style={{
                    display:
                    "grid",

                    gap:
                    "8px",

                    padding:
                    "10px",

                    border:
                    "1px solid rgba(203, 170, 93, 0.25)",

                       borderRadius:
                       "8px",

                       background:
                       "rgba(0, 0, 0, 0.24)",
                }}
                >
                {files.map(
                    (
                        file,
                    ) => (
                        <div
                        key={
                            file.file_id
                        }
                        style={{
                            display:
                            "grid",

                            gap:
                            "6px",

                            padding:
                            "8px",

                            border:
                            "1px solid rgba(255,255,255,0.08)",

                          borderRadius:
                          "6px",
                        }}
                        >
                        <div
                        style={{
                            display:
                            "flex",

                            alignItems:
                            "center",

                            justifyContent:
                            "space-between",

                            gap:
                            "8px",
                        }}
                        >
                        <strong
                        style={{
                            fontSize:
                            "13px",
                        }}
                        >
                        {file.name ||
                            file.file_name}
                            </strong>

                            <span
                            style={{
                                fontSize:
                                "10px",

                                opacity:
                                0.7,
                            }}
                            >
                            {file.category_name ||
                                "FILE"}
                                </span>
                                </div>

                                <div
                                style={{
                                    fontSize:
                                    "11px",

                                    opacity:
                                    0.65,
                                }}
                                >
                                {formatBytes(
                                    file.size_bytes,
                                )}

                                {file.version
                                    ? ` • v${file.version}`
                                    : ""}

                                    {file.is_primary
                                        ? " • Primary"
                                        : ""}
                                        </div>

                                        <button
                                        className="button secondary"
                                        disabled={
                                            !file.supported_archive ||
                                            activeFileId !== null
                                        }
                                        onClick={() =>
                                            beginDownload(
                                                file,
                                            )
                                        }
                                        >
                                        {!file.supported_archive
                                            ? "Unsupported Archive"
                                            : isPremium
                                            ? "Download & Install"
                                            : "Authorize & Install"}
                                            </button>
                                            </div>
                    ),
                )}
                </div>
            )}

            {activeFileId !==
                null && (
                    <div
                    style={{
                        display:
                        "grid",

                        gap:
                        "5px",
                    }}
                    >
                    {status && (
                        <div
                        className="nexus-small-note"
                        >
                        {status}
                        </div>
                    )}

                    {progress !==
                        null && (
                            <div
                            style={{
                                height:
                                "5px",

                                overflow:
                                "hidden",

                                borderRadius:
                                "999px",

                                background:
                                "rgba(255,255,255,0.08)",
                            }}
                            >
                            <div
                            style={{
                                width:
                                `${progress}%`,

                                height:
                                "100%",

                                background:
                                "currentColor",

                                transition:
                                "width 120ms linear",
                            }}
                            />
                            </div>
                        )}
                        </div>
                )}

                {pendingVariants && (
                    <div
                    style={{
                        display:
                        "grid",

                        gap:
                        "7px",

                        padding:
                        "10px",

                        border:
                        "1px solid rgba(203, 170, 93, 0.35)",

                                     borderRadius:
                                     "8px",

                                     background:
                                     "rgba(0,0,0,0.3)",
                    }}
                    >
                    <strong>
                    Choose a variant
                    </strong>

                    {pendingVariants
                        .analysis
                        .variants
                        .map(
                            (
                                variant,
                            ) => (
                                <button
                                key={
                                    variant.id
                                }
                                className="button secondary"
                                onClick={() =>
                                    installVariant(
                                        pendingVariants
                                        .analysis,

                                        pendingVariants
                                        .profileId,

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
                    <div
                    className="nexus-error"
                    style={{
                        margin:
                        0,
                    }}
                    >
                    {error}
                    </div>
                )}
                </div>
    );
}
