export function PlaybackStatus({
  status,
  diagnostics,
}: {
  status: string;
  diagnostics: string[];
}) {
  return (
    <div
      className={`playback-status playback-status-${status}`}
      aria-live="polite"
    >
      <strong>{status === "running" ? "Playing externally" : status}</strong>
      {diagnostics.length > 0 ? (
        <details open={status === "error"}>
          <summary>Playback diagnostics</summary>
          <pre>{diagnostics.join("\n")}</pre>
        </details>
      ) : null}
    </div>
  );
}
