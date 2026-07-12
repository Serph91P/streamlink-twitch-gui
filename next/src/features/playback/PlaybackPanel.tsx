import { useQuery } from "@tanstack/react-query";

import type { PlaybackBackend } from "../../api/backend";
import { PlaybackStatus } from "./PlaybackStatus";
import { QualityPicker } from "./QualityPicker";
import { usePlayback } from "./usePlayback";

function Controls({
  backend,
  url,
  variants,
}: {
  backend: PlaybackBackend;
  url: string;
  variants: Awaited<ReturnType<PlaybackBackend["inspectStreams"]>>["variants"];
}) {
  const playback = usePlayback(backend, url, variants);
  return (
    <section className="playback-panel" aria-labelledby="playback-heading">
      <div>
        <p className="eyebrow">External player</p>
        <h2 id="playback-heading">Choose the broadcast signal</h2>
      </div>
      <QualityPicker
        variants={variants}
        selected={playback.selected}
        onChange={(name) => void playback.select(name)}
      />
      <div className="playback-actions">
        {playback.status === "running" ? (
          <button onClick={() => void playback.stop()}>Stop playback</button>
        ) : (
          <button
            className="primary-action"
            onClick={() => void playback.play()}
            disabled={!playback.selected || playback.status === "launching"}
          >
            Launch stream
          </button>
        )}
      </div>
      <PlaybackStatus
        status={playback.status}
        diagnostics={playback.diagnostics}
      />
    </section>
  );
}

export function PlaybackPanel({
  backend,
  login,
}: {
  backend: PlaybackBackend;
  login: string;
}) {
  const url = `https://www.twitch.tv/${encodeURIComponent(login)}`;
  const capabilities = useQuery({
    queryKey: ["streamlink", "inspect", login],
    queryFn: ({ signal }) => backend.inspectStreams(url, signal),
  });
  if (capabilities.isPending)
    return (
      <p role="status" className="state-panel">
        Inspecting available qualities...
      </p>
    );
  if (capabilities.error)
    return (
      <div role="alert" className="state-panel">
        <strong>Could not inspect this stream</strong>
        <p>{capabilities.error.message}</p>
        <button onClick={() => void capabilities.refetch()}>
          Inspect again
        </button>
      </div>
    );
  if (capabilities.data.variants.length === 0)
    return (
      <div className="state-panel">
        <strong>No playable qualities were advertised</strong>
        <p>Check Streamlink and try inspecting the channel again.</p>
      </div>
    );
  return (
    <Controls
      backend={backend}
      url={url}
      variants={capabilities.data.variants}
    />
  );
}
