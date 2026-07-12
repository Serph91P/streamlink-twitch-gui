import type { TwitchStream } from "../../domain/twitch";

function safeTwitchImage(url: string): string | undefined {
  try {
    const parsed = new URL(
      url.replace("{width}", "640").replace("{height}", "360"),
    );
    return parsed.protocol === "https:" &&
      (parsed.hostname === "static-cdn.jtvnw.net" ||
        parsed.hostname.endsWith(".jtvnw.net"))
      ? parsed.href
      : undefined;
  } catch {
    return undefined;
  }
}

export function StreamCard({
  stream,
  onOpen,
}: {
  stream: TwitchStream;
  onOpen?: () => void;
}) {
  return (
    <article className="stream-card">
      <a
        href={`https://www.twitch.tv/${encodeURIComponent(stream.userLogin)}`}
        onClick={
          onOpen
            ? (event) => {
                event.preventDefault();
                onOpen();
              }
            : undefined
        }
      >
        <div className="stream-visual">
          {safeTwitchImage(stream.thumbnailUrl) ? (
            <img src={safeTwitchImage(stream.thumbnailUrl)} alt="" />
          ) : (
            <span className="image-fallback" />
          )}
          <span className="live-badge">Live</span>
          <span className="viewer-count">
            {new Intl.NumberFormat(undefined, { notation: "compact" }).format(
              stream.viewerCount,
            )}{" "}
            watching
          </span>
        </div>
        <div className="stream-copy">
          <p className="eyebrow">{stream.gameName || "Uncategorized"}</p>
          <h3>{stream.userName}</h3>
          <p>{stream.title}</p>
        </div>
      </a>
    </article>
  );
}
