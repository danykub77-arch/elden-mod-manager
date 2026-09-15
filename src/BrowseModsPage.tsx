import {
  FormEvent,
  useEffect,
  useState,
} from "react";

import {
  invoke,
} from "@tauri-apps/api/core";

import NexusInstallButton from "./NexusInstallButton";

type NexusAccountStatus = {
  connected: boolean;
  username: string | null;
  is_premium: boolean | null;
};

type NexusMod = {
  mod_id: number;

  name: string;
  summary: string;
  version: string;
  author: string;

  endorsements: number;
  downloads: number;

  created_at: string | null;
  updated_at: string | null;

  picture_url: string | null;
  category: string | null;

  nexus_url: string;
};

type NexusTag = {
  id: number;
  name: string;
};

type NexusBrowseResult = {
  mods: NexusMod[];

  page: number;
  page_size: number;

  total_count: number;
  total_pages: number;

  has_previous: boolean;
  has_next: boolean;
};

type BrowseSort =
| "popular"
| "downloads"
| "new"
| "updated"
| "name"
| "relevance";

const PAGE_SIZE = 24;

function formatNumber(
  value: number,
) {
  return new Intl.NumberFormat()
  .format(value);
}

function formatDate(
  value: string | null,
) {
  if (!value) {
    return "Unknown";
  }

  const date =
  new Date(value);

  if (
    Number.isNaN(
      date.getTime(),
    )
  ) {
    return "Unknown";
  }

  return date.toLocaleDateString(
    undefined,
    {
      year: "numeric",
      month: "short",
      day: "numeric",
    },
  );
}

type BrowseModsPageProps = {
  selectedProfileId: string | null;

  installedMods: {
    id: string;
    name: string;
    version: string | null;
    enabled: boolean;
  }[];

  onModsChanged: () => void | Promise<void>;
};

export default function BrowseModsPage({
  selectedProfileId,
  installedMods,
  onModsChanged,
}: BrowseModsPageProps) {
  // These are wired in from App so Browse Mods always has
  // the currently selected profile and live installed-mod state.
  void selectedProfileId;
  void installedMods;
  void onModsChanged;
  const [
    account,
    setAccount,
  ] =
  useState<NexusAccountStatus | null>(
    null,
  );

  const [
    apiKey,
    setApiKey,
  ] =
  useState("");

  const [tags, setTags] =
  useState<NexusTag[]>([]);

  const [
    selectedTags,
    setSelectedTags,
  ] =
  useState<string[]>([]);

  const [
    tagSearch,
    setTagSearch,
  ] =
  useState("");

  const [
    filtersOpen,
    setFiltersOpen,
  ] =
  useState(false);

  const [
    loadingTags,
    setLoadingTags,
  ] =
  useState(false);

  const [
    result,
    setResult,
  ] =
  useState<NexusBrowseResult | null>(
    null,
  );

  const [
    searchInput,
    setSearchInput,
  ] =
  useState("");

  const [
    activeSearch,
    setActiveSearch,
  ] =
  useState("");

  const [
    sort,
    setSort,
  ] =
  useState<BrowseSort>(
    "popular",
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
    connecting,
    setConnecting,
  ] =
  useState(false);

  const [
    error,
    setError,
  ] =
  useState<string | null>(
    null,
  );

  async function loadMods(
    nextPage: number,
    nextSort: BrowseSort = sort,
    nextSearch: string = activeSearch,
    nextTags: string[] = selectedTags,
  ) {
    setLoadingMods(true);
    setError(null);

    try {
      const response =
      await invoke<NexusBrowseResult>(
        "browse_nexus_mods",
        {
          search:
          nextSearch,

          sort:
          nextSort,

          page:
          nextPage,

          pageSize:
          PAGE_SIZE,

          tags:
          nextTags,
        },
      );

      setResult(
        response,
      );
    } catch (reason) {
      console.error(
        reason,
      );

      setError(
        String(reason),
      );

      setResult(
        null,
      );
    } finally {
      setLoadingMods(
        false,
      );
    }
  }

  async function loadTags() {
    setLoadingTags(true);

    try {
      const response =
      await invoke<NexusTag[]>(
        "get_nexus_tags",
      );

      setTags(
        response,
      );
    } catch (reason) {
      console.error(
        "Could not load Nexus tags:",
        reason,
      );

      setTags([]);

      setError(
        `Could not load Nexus tags: ${String(
          reason,
        )}`,
      );
    } finally {
      setLoadingTags(false);
    }
  }

  async function toggleTag(
    tagName: string,
  ) {
    const alreadySelected =
    selectedTags.includes(
      tagName,
    );

    const nextTags =
    alreadySelected
    ? selectedTags.filter(
        (tag) =>
        tag !== tagName,
      )
    : [
        ...selectedTags,
        tagName,
      ];

    setSelectedTags(
      nextTags,
    );

    await loadMods(
      1,
      sort,
      activeSearch,
      nextTags,
    );
  }

  async function clearTags() {
    setSelectedTags([]);

    await loadMods(
      1,
      sort,
      activeSearch,
      [],
    );
  }

  async function loadAccount() {
    setLoading(
      true,
    );

    try {
      const response =
      await invoke<NexusAccountStatus>(
        "get_nexus_account",
      );

      setAccount(
        response,
      );

      if (
        response.connected
      ) {
        await Promise.all([
          loadTags(),

          loadMods(
            1,
            "popular",
            "",
            [],
          ),
        ]);
      }
    } catch (reason) {
      console.error(
        reason,
      );

      setError(
        String(reason),
      );
    } finally {
      setLoading(
        false,
      );
    }
  }

  async function connect() {
    const key =
    apiKey.trim();

    if (!key) {
      setError(
        "Enter your Nexus Mods API key.",
      );

      return;
    }

    setConnecting(
      true,
    );

    setError(
      null,
    );

    try {
      const response =
      await invoke<NexusAccountStatus>(
        "connect_nexus",
        {
          apiKey:
          key,
        },
      );

      setAccount(
        response,
      );

      setApiKey(
        "",
      );

      setActiveSearch(
        "",
      );

      setSearchInput(
        "",
      );

      setSort(
        "popular",
      );

      setSelectedTags([]);
      setTagSearch("");

      await Promise.all([
        loadTags(),

        loadMods(
          1,
          "popular",
          "",
          [],
        ),
      ]);
    } catch (reason) {
      console.error(
        reason,
      );

      setError(
        String(reason),
      );
    } finally {
      setConnecting(
        false,
      );
    }
  }

  async function disconnect() {
    try {
      const response =
      await invoke<NexusAccountStatus>(
        "disconnect_nexus",
      );

      setAccount(
        response,
      );

      setResult(
        null,
      );

      setTags([]);
      setSelectedTags([]);
      setTagSearch("");
      setFiltersOpen(false);

      setSearchInput(
        "",
      );

      setActiveSearch(
        "",
      );

      setError(
        null,
      );

      setSort(
        "popular",
      );
    } catch (reason) {
      console.error(
        reason,
      );

      setError(
        String(reason),
      );
    }
  }

  async function openMod(
    modId: number,
  ) {
    try {
      await invoke(
        "open_nexus_mod",
        {
          modId,
        },
      );
    } catch (reason) {
      console.error(
        reason,
      );

      setError(
        String(reason),
      );
    }
  }

  async function changeSort(
    nextSort: BrowseSort,
  ) {
    if (
      nextSort === sort &&
      result
    ) {
      return;
    }

    setSort(
      nextSort,
    );

    await loadMods(
      1,
      nextSort,
      activeSearch,
    );
  }

  async function submitSearch(
    event: FormEvent,
  ) {
    event.preventDefault();

    const query =
    searchInput.trim();

    setActiveSearch(
      query,
    );

    let nextSort =
    sort;

    if (
      query &&
      sort === "popular"
    ) {
      nextSort =
      "relevance";

      setSort(
        "relevance",
      );
    }

    if (
      !query &&
      sort === "relevance"
    ) {
      nextSort =
      "popular";

        setSort(
          "popular",
        );
    }

    await loadMods(
      1,
      nextSort,
      query,
    );
  }

  async function clearSearch() {
    setSearchInput(
      "",
    );

    setActiveSearch(
      "",
    );

    const nextSort =
    sort === "relevance"
    ? "popular"
    : sort;

    if (
      nextSort !== sort
    ) {
      setSort(
        nextSort,
      );
    }

    await loadMods(
      1,
      nextSort,
      "",
    );
  }

  useEffect(
    () => {
      loadAccount();
    },
    [],
  );

  if (loading) {
    return (
      <div className="browse-loading">
      <div className="spinner" />

      <span>
      Connecting to Nexus Mods...
      </span>
      </div>
    );
  }

  if (!account?.connected) {
    return (
      <>
      <header className="page-header">
      <div>
      <div className="eyebrow">
      NEXUS MODS
      </div>

      <h1>
      Browse Mods
      </h1>

      <p>
      Browse Elden Ring mods
      directly from Nexus Mods.
      </p>
      </div>
      </header>

      <section className="nexus-connect-card">
      <div className="eyebrow">
      DEVELOPMENT CONNECTION
      </div>

      <h2>
      Connect Nexus Mods
      </h2>

      <p className="muted">
      During development,
      Elden Mod Manager uses your
      personal Nexus Mods API key.
      Public releases will use a
      registered Nexus application
      instead.
      </p>

      <label className="field-label">
      Personal API key
      </label>

      <div className="nexus-connect-row">
      <input
      type="password"
      value={apiKey}
      placeholder="Paste API key"
      autoComplete="off"
      onChange={(
        event,
      ) =>
      setApiKey(
        event.target.value,
      )
      }
      onKeyDown={(
        event,
      ) => {
        if (
          event.key ===
          "Enter"
        ) {
          connect();
        }
      }}
      />

      <button
      className="button primary"
      disabled={
        connecting ||
        !apiKey.trim()
      }
      onClick={
        connect
      }
      >
      {connecting
        ? "Connecting..."
        : "Connect"}
        </button>
        </div>

        {error && (
          <div className="nexus-error">
          {error}
          </div>
        )}

        <p className="nexus-small-note">
        Age-restricted mod listings
        are excluded from the
        launcher.
        </p>
        </section>
        </>
    );
  }

  const visibleTags =
  tags.filter(
    (tag) =>
    tag.name
      .toLowerCase()
      .includes(
        tagSearch
          .trim()
          .toLowerCase(),
      ),
  );

  const mods =
  result?.mods ?? [];

  return (
    <>
    <header className="page-header browse-header">
    <div>
    <div className="eyebrow">
    NEXUS MODS
    </div>

    <h1>
    Browse Mods
    </h1>

    <p>
    Search and discover Elden
    Ring mods without leaving
    the manager.
    </p>
    </div>

    <div className="nexus-account-pill">
    <div>
    <span>
    CONNECTED AS
    </span>

    <strong>
    {account.username ??
      "Nexus User"}
      </strong>
      </div>

      {account.is_premium && (
        <span className="nexus-premium">
        PREMIUM
        </span>
      )}

      <button
      className="text-button"
      onClick={
        disconnect
      }
      >
      Disconnect
      </button>
      </div>
      </header>

      <section className="browse-toolbar">
      <form
      className="nexus-connect-row"
      onSubmit={
        submitSearch
      }
      >
      <input
      className="browse-search"
      value={
        searchInput
      }
      placeholder="Search Elden Ring mods..."
      disabled={
        loadingMods
      }
      onChange={(
        event,
      ) =>
      setSearchInput(
        event.target.value,
      )
      }
      />

      <button
      className="button primary"
      type="submit"
      disabled={
        loadingMods
      }
      >
      Search
      </button>

      {activeSearch && (
        <button
        className="button secondary"
        type="button"
        disabled={
          loadingMods
        }
        onClick={
          clearSearch
        }
        >
        Clear
        </button>
      )}
      </form>

      <button
      type="button"
      className={`button secondary browse-filter-toggle ${
        filtersOpen
        ? "active"
        : ""
      }`}
      onClick={async () => {
        const opening =
          !filtersOpen;

        setFiltersOpen(
          opening,
        );

        if (
          opening &&
          tags.length === 0 &&
          !loadingTags
        ) {
          await loadTags();
        }
      }}
      >
      <span>
      Filters
      </span>

      {selectedTags.length > 0 && (
        <strong className="browse-filter-count">
        {selectedTags.length}
        </strong>
      )}
      </button>
      </section>

      {filtersOpen && (
        <section className="nexus-filter-panel">
        <div className="nexus-filter-header">
        <div>
        <div className="eyebrow">
        NEXUS FILTERS
        </div>

        <h2>
        Tags
        </h2>

        <p className="muted">
        Tags are loaded directly from
        Nexus Mods.
        </p>
        </div>

        {selectedTags.length > 0 && (
          <button
          type="button"
          className="text-button"
          disabled={
            loadingMods
          }
          onClick={
            clearTags
          }
          >
          Clear All
          </button>
        )}
        </div>

        <input
        className="nexus-tag-search"
        value={
          tagSearch
        }
        placeholder="Search tags..."
        onChange={(
          event,
        ) =>
        setTagSearch(
          event.target.value,
        )
        }
        />

        {selectedTags.length > 0 && (
          <div className="nexus-selected-tags">
          {selectedTags.map(
            (tag) => (
              <button
              type="button"
              key={
                tag
              }
              className="nexus-tag-chip selected"
              disabled={
                loadingMods
              }
              onClick={() =>
                toggleTag(
                  tag,
                )
              }
              >
              <span>
              {tag}
              </span>

              <span className="nexus-tag-remove">
              ×
              </span>
              </button>
            ),
          )}
          </div>
        )}

        <div className="nexus-tag-list">
        {loadingTags ? (
          <div className="nexus-filter-status">
          Loading Nexus tags...
          </div>
        ) : visibleTags.length === 0 ? (
          <div className="nexus-filter-status">
          No tags found.
          </div>
        ) : (
          visibleTags.map(
            (tag) => {
              const checked =
              selectedTags.includes(
                tag.name,
              );

              return (
                <label
                key={
                  tag.id
                }
                className={`nexus-tag-option ${
                  checked
                  ? "selected"
                  : ""
                }`}
                >
                <input
                  type="checkbox"
                  checked={
                    checked
                  }
                  disabled={
                    loadingMods
                  }
                  onChange={() =>
                    toggleTag(
                      tag.name,
                    )
                  }
                />

                <span>
                {tag.name}
                </span>
                </label>
              );
            },
          )
        )}
        </div>
        </section>
      )}

      <section className="browse-toolbar">
      <div className="browse-feed-tabs">
      {activeSearch && (
        <button
        className={
          sort === "relevance"
          ? "active"
          : ""
        }
        disabled={
          loadingMods
        }
        onClick={() =>
          changeSort(
            "relevance",
          )
        }
        >
        Relevance
        </button>
      )}

      <button
      className={
        sort === "popular"
        ? "active"
        : ""
      }
      disabled={
        loadingMods
      }
      onClick={() =>
        changeSort(
          "popular",
        )
      }
      >
      Popular
      </button>

      <button
      className={
        sort === "downloads"
        ? "active"
        : ""
      }
      disabled={
        loadingMods
      }
      onClick={() =>
        changeSort(
          "downloads",
        )
      }
      >
      Downloads
      </button>

      <button
      className={
        sort === "new"
        ? "active"
        : ""
      }
      disabled={
        loadingMods
      }
      onClick={() =>
        changeSort(
          "new",
        )
      }
      >
      New
      </button>

      <button
      className={
        sort === "updated"
        ? "active"
        : ""
      }
      disabled={
        loadingMods
      }
      onClick={() =>
        changeSort(
          "updated",
        )
      }
      >
      Updated
      </button>

      <button
      className={
        sort === "name"
        ? "active"
        : ""
      }
      disabled={
        loadingMods
      }
      onClick={() =>
        changeSort(
          "name",
        )
      }
      >
      A–Z
      </button>
      </div>

      <button
      className="button secondary"
      disabled={
        loadingMods
      }
      onClick={() =>
        loadMods(
          result?.page ?? 1,
        )
      }
      >
      {loadingMods
        ? "Refreshing..."
        : "Refresh"}
        </button>
        </section>

        {selectedTags.length > 0 && (
          <div className="browse-active-filter-chips">
          {selectedTags.map(
            (tag) => (
              <button
              type="button"
              key={
                tag
              }
              className="nexus-tag-chip selected"
              disabled={
                loadingMods
              }
              onClick={() =>
                toggleTag(
                  tag,
                )
              }
              >
              {tag}
              <span className="nexus-tag-remove">
              ×
              </span>
              </button>
            ),
          )}
          </div>
        )}

        {activeSearch && (
          <div className="nexus-small-note browse-note">
          Search results for{" "}
          <strong>
          “{activeSearch}”
          </strong>
          </div>
        )}

        {result && (
          <div className="nexus-small-note browse-note">
          {formatNumber(
            result.total_count,
          )}{" "}
          mods found
          {result.total_pages > 0 && (
            <>
            {" "}• Page{" "}
            {result.page} of{" "}
            {formatNumber(
              result.total_pages,
            )}
            </>
          )}
          </div>
        )}

        {error && (
          <div className="nexus-error">
          {error}
          </div>
        )}

        {loadingMods ? (
          <div className="browse-loading">
          <div className="spinner" />

          <span>
          Loading mods...
          </span>
          </div>
        ) : mods.length === 0 ? (
          <div className="browse-empty">
          <h2>
          No mods found
          </h2>

          <p>
          Try changing your search
          or sort mode.
          </p>
          </div>
        ) : (
          <div className="browse-mod-grid">
          {mods.map(
            (mod) => (
              <article
              key={
                mod.mod_id
              }
              className="browse-mod-card"
              >
              <button
              className="browse-mod-image"
              onClick={() =>
                openMod(
                  mod.mod_id,
                )
              }
              >
              {mod.picture_url ? (
                <img
                src={
                  mod.picture_url
                }
                alt=""
                />
              ) : (
                <div className="browse-mod-placeholder">
                ER
                </div>
              )}
              </button>

              <div className="browse-mod-body">
              <div className="browse-mod-topline">
              {mod.category ? (
                <span>
                {mod.category}
                </span>
              ) : (
                <span>
                MOD {mod.mod_id}
                </span>
              )}

              {mod.version && (
                <span>
                v{mod.version}
                </span>
              )}
              </div>

              <button
              className="browse-mod-title"
              onClick={() =>
                openMod(
                  mod.mod_id,
                )
              }
              >
              {mod.name}
              </button>

              <div className="browse-mod-author">
              by{" "}
              {mod.author ||
                "Unknown"}
                </div>

                <p>
                {mod.summary ||
                  "No summary provided."}
                  </p>

                  <div className="browse-mod-stats">
                  <span>
                  {formatNumber(
                    mod.downloads,
                  )}{" "}
                  downloads
                  </span>

                  <span>
                  {formatNumber(
                    mod.endorsements,
                  )}{" "}
                  endorsements
                  </span>

                  <span>
                  Updated{" "}
                  {formatDate(
                    mod.updated_at,
                  )}
                  </span>
                  </div>

                  <div
                  style={{
                    display: "grid",
                    gap: "8px",
                  }}
                  >
                  <NexusInstallButton
                  modId={
                    mod.mod_id
                  }
                  modName={
                    mod.name
                  }
                  isPremium={
                    Boolean(
                      account.is_premium,
                    )
                  }
                  />

                  <button
                  className="button secondary browse-open-button"
                  onClick={() =>
                    openMod(
                      mod.mod_id,
                    )
                  }
                  >
                  View on Nexus
                  </button>
                  </div>
                  </div>
                  </article>
            ),
          )}
          </div>
        )}

        {result &&
          result.total_pages > 1 && (
            <section className="browse-toolbar">
            <button
            className="button secondary"
            disabled={
              loadingMods ||
              !result.has_previous
            }
            onClick={() =>
              loadMods(
                result.page - 1,
              )
            }
            >
            ← Previous
            </button>

            <div className="nexus-small-note">
            Page{" "}
            <strong>
            {result.page}
            </strong>{" "}
            of{" "}
            <strong>
            {formatNumber(
              result.total_pages,
            )}
            </strong>
            </div>

            <button
            className="button secondary"
            disabled={
              loadingMods ||
              !result.has_next
            }
            onClick={() =>
              loadMods(
                result.page + 1,
              )
            }
            >
            Next →
            </button>
            </section>
          )}

          <div className="nexus-small-note browse-note">
          Showing up to {PAGE_SIZE} mods
          per page. Search and pagination
          are handled by Nexus rather than
          filtering a ten-mod local feed.
          </div>
          </>
  );
}
