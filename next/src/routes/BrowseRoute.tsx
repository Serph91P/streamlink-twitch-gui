import { useQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";

import type { AppBackend } from "../api/backend";
import { ChannelCard } from "../components/channel/ChannelCard";
import { StreamCard } from "../components/channel/StreamCard";
import { GameCard } from "../components/game/GameCard";
import type { RouteName } from "../components/layout/AppLayout";
import type {
  FollowedChannel,
  TwitchPage,
  TwitchSearchChannel,
  TwitchStream,
} from "../domain/twitch";
import type { Settings } from "../domain/settings";
import { SettingsPanel } from "../features/settings/SettingsPanel";
import { PlaybackPanel } from "../features/playback/PlaybackPanel";

function Header({
  eyebrow,
  title,
  description,
}: {
  eyebrow: string;
  title: string;
  description: string;
}) {
  return (
    <header className="route-header">
      <p className="eyebrow">{eyebrow}</p>
      <h1>{title}</h1>
      <p>{description}</p>
    </header>
  );
}

function QueryState({
  pending,
  error,
  empty,
  onRetry,
}: {
  pending: boolean;
  error: Error | null;
  empty: boolean;
  onRetry: () => void;
}) {
  if (pending)
    return (
      <p role="status" className="state-panel">
        Tuning the live feed...
      </p>
    );
  if (error)
    return (
      <div role="alert" className="state-panel">
        <strong>Could not load Twitch</strong>
        <p>{error.message}</p>
        <button onClick={onRetry}>Try again</button>
      </div>
    );
  if (empty)
    return (
      <div className="state-panel">
        <strong>No live channels found</strong>
        <p>The signal is quiet. Refresh or explore another section.</p>
        <button onClick={onRetry}>Refresh</button>
      </div>
    );
  return null;
}

function LiveRoute({ backend }: { backend: AppBackend }) {
  const query = useQuery({
    queryKey: ["twitch", "streams"],
    queryFn: ({ signal }) => backend.streams(undefined, undefined, signal),
  });
  return (
    <>
      <Header
        eyebrow="On air"
        title="Live now"
        description="Current broadcasts, ordered by the Twitch live signal."
      />
      <QueryState
        pending={query.isPending}
        error={query.error}
        empty={query.data?.items.length === 0}
        onRetry={() => void query.refetch()}
      />
      {query.data?.items.length ? (
        <section className="stream-grid" aria-label="Live streams">
          {query.data.items.map((stream) => (
            <StreamCard key={stream.id} stream={stream} />
          ))}
        </section>
      ) : null}
    </>
  );
}

function FollowingRoute({
  backend,
  channelsOnly = false,
}: {
  backend: AppBackend;
  channelsOnly?: boolean;
}) {
  const session = useQuery({
    queryKey: ["twitch", "session"],
    queryFn: ({ signal }) => backend.getSession(signal),
  });
  const userId =
    session.data?.status === "authenticated" ? session.data.user.id : "";
  const query = useQuery<TwitchPage<TwitchStream | FollowedChannel>>({
    queryKey: [
      "twitch",
      channelsOnly ? "followed-channels" : "followed-streams",
      userId,
    ],
    enabled: Boolean(userId),
    queryFn: async ({ signal }) =>
      channelsOnly
        ? backend.followedChannels(userId, undefined, signal)
        : backend.followedStreams(userId, undefined, signal),
  });
  const title = channelsOnly ? "Followed channels" : "Following live";
  if (!session.isPending && !userId)
    return (
      <>
        <Header
          eyebrow="Your lineup"
          title={title}
          description="Your personal Twitch desk."
        />
        <div className="state-panel">
          <strong>Sign in to see your follows</strong>
          <p>Authentication stays in the secure Rust core.</p>
        </div>
      </>
    );
  return (
    <>
      <Header
        eyebrow="Your lineup"
        title={title}
        description="Your personal Twitch desk."
      />
      <QueryState
        pending={session.isPending || query.isPending}
        error={session.error || query.error}
        empty={Boolean(query.data && query.data.items.length === 0)}
        onRetry={() => void query.refetch()}
      />
      {query.data && !channelsOnly ? (
        <section className="stream-grid">
          {query.data.items.map((item) =>
            "id" in item ? <StreamCard key={item.id} stream={item} /> : null,
          )}
        </section>
      ) : null}
      {query.data && channelsOnly ? (
        <section className="channel-list">
          {query.data.items.map((item) =>
            "broadcasterId" in item ? (
              <article key={item.broadcasterId}>
                <strong>{item.broadcasterName}</strong>
                <span>@{item.broadcasterLogin}</span>
              </article>
            ) : null,
          )}
        </section>
      ) : null}
    </>
  );
}

function CategoriesRoute({ backend }: { backend: AppBackend }) {
  const query = useQuery({
    queryKey: ["twitch", "top-games"],
    queryFn: ({ signal }) => backend.topGames(undefined, signal),
  });
  return (
    <>
      <Header
        eyebrow="Directory"
        title="Top categories"
        description="The busiest rooms across Twitch right now."
      />
      <QueryState
        pending={query.isPending}
        error={query.error}
        empty={query.data?.items.length === 0}
        onRetry={() => void query.refetch()}
      />
      {query.data?.items.length ? (
        <section className="game-grid">
          {query.data.items.map((game) => (
            <GameCard key={game.id} game={game} />
          ))}
        </section>
      ) : null}
    </>
  );
}

function SearchRoute({
  backend,
  settings,
}: {
  backend: AppBackend;
  settings: Settings;
}) {
  const [input, setInput] = useState("");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<TwitchSearchChannel>();
  const results = useQuery({
    queryKey: ["twitch", "search", search],
    enabled: Boolean(search),
    queryFn: ({ signal }) => backend.searchChannels(search, undefined, signal),
  });
  const categoryResults = useQuery({
    queryKey: ["twitch", "search-categories", search],
    enabled: Boolean(search),
    queryFn: ({ signal }) =>
      backend.searchCategories(search, undefined, signal),
  });
  const detail = useQuery({
    queryKey: ["twitch", "channel", selected?.id],
    enabled: Boolean(selected),
    queryFn: ({ signal }) => backend.streams(selected?.id, undefined, signal),
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    setSelected(undefined);
    setSearch(input.trim());
  }
  if (selected)
    return (
      <>
        <button className="back-button" onClick={() => setSelected(undefined)}>
          Back to search
        </button>
        <Header
          eyebrow={selected.isLive ? "Live channel" : "Channel detail"}
          title={selected.displayName}
          description={
            selected.title || "Channel status and playback controls."
          }
        />
        <QueryState
          pending={detail.isPending}
          error={detail.error}
          empty={false}
          onRetry={() => void detail.refetch()}
        />
        {detail.data?.items.length === 0 ? (
          <div className="state-panel">
            <strong>{selected.displayName} is offline</strong>
            <p>
              Search remains available while you wait for the next broadcast.
            </p>
            <button onClick={() => void detail.refetch()}>Try again</button>
          </div>
        ) : null}
        {detail.data?.items.map((stream) => (
          <div key={stream.id} className="channel-detail">
            <StreamCard stream={stream} />
            <PlaybackPanel
              backend={backend}
              login={stream.userLogin}
              settings={settings}
            />
          </div>
        ))}
      </>
    );
  return (
    <>
      <Header
        eyebrow="Find a signal"
        title="Search Twitch"
        description="Look up channels and broadcasts without leaving the keyboard."
      />
      <form role="search" className="search-form" onSubmit={submit}>
        <label htmlFor="channel-search">Channel name</label>
        <div>
          <input
            id="channel-search"
            type="search"
            value={input}
            onChange={(event) => setInput(event.target.value)}
            placeholder="Search channels"
          />
          <button type="submit">Search</button>
        </div>
      </form>
      {results.isFetching || categoryResults.isFetching ? (
        <p role="status" className="state-panel">
          Searching Twitch...
        </p>
      ) : null}
      {results.error ? (
        <QueryState
          pending={false}
          error={results.error}
          empty={false}
          onRetry={() => void results.refetch()}
        />
      ) : null}
      {categoryResults.error ? (
        <QueryState
          pending={false}
          error={categoryResults.error}
          empty={false}
          onRetry={() => void categoryResults.refetch()}
        />
      ) : null}
      {search && results.data?.items.length === 0 ? (
        <div className="state-panel">
          <strong>No channels match &quot;{search}&quot;</strong>
        </div>
      ) : null}
      {results.data?.items.length ? <h2>Channels</h2> : null}
      <section className="channel-grid" aria-label="Channel results">
        {results.data?.items.map((channel) => (
          <ChannelCard
            key={channel.id}
            channel={channel}
            onOpen={() => setSelected(channel)}
          />
        ))}
      </section>
      {categoryResults.data?.items.length ? <h2>Categories</h2> : null}
      <section className="game-grid" aria-label="Category results">
        {categoryResults.data?.items.map((game) => (
          <GameCard key={game.id} game={game} />
        ))}
      </section>
    </>
  );
}

export function BrowseRoute({
  route,
  backend,
  settings,
  onSettingsSaved,
}: {
  route: RouteName;
  backend: AppBackend;
  settings: Settings;
  onSettingsSaved: (settings: Settings) => void;
  onNavigate: (route: RouteName) => void;
}) {
  if (route === "live") return <LiveRoute backend={backend} />;
  if (route === "following") return <FollowingRoute backend={backend} />;
  if (route === "channels")
    return <FollowingRoute backend={backend} channelsOnly />;
  if (route === "categories") return <CategoriesRoute backend={backend} />;
  if (route === "search")
    return <SearchRoute backend={backend} settings={settings} />;
  return (
    <SettingsPanel
      backend={backend}
      settings={settings}
      onSaved={onSettingsSaved}
    />
  );
}
