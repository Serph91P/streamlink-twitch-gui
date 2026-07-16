import { useState } from "react";

import type { PlaybackBackend } from "../../api/backend";
import type { StreamCodec, StreamVariant } from "../../domain/stream";
import { selectBestVariant } from "./QualityPicker";

export function usePlayback(
  backend: PlaybackBackend,
  url: string,
  variants: StreamVariant[],
  constraints: { preferredCodec?: StreamCodec; maximumHeight?: number } = {},
) {
  const [selected, setSelected] = useState(
    () =>
      selectBestVariant(variants, {
        preferredCodec: constraints.preferredCodec,
        maximumHeight: constraints.maximumHeight,
      })?.name,
  );
  const [status, setStatus] = useState<
    "idle" | "launching" | "running" | "stopping" | "error"
  >("idle");
  const [diagnostics, setDiagnostics] = useState<string[]>([]);

  async function launch(name = selected) {
    if (!name) return;
    setStatus("launching");
    try {
      const codec = variants.find((variant) => variant.name === name)?.codec;
      const result = await backend.launchStream({
        url,
        variantName: name,
        codecs: codec ? [codec] : [],
      });
      if (result.status !== "running") {
        throw new Error(
          result.diagnostics[0] ?? "Playback exited before it could start",
        );
      }
      setDiagnostics(result.diagnostics);
      setStatus("running");
    } catch (error) {
      setDiagnostics([
        error instanceof Error ? error.message : "Playback could not start",
      ]);
      setStatus("error");
    }
  }

  async function stop() {
    setStatus("stopping");
    try {
      const result = await backend.stopStream();
      setDiagnostics(result.diagnostics);
      setStatus("idle");
    } catch (error) {
      setDiagnostics([
        error instanceof Error ? error.message : "Playback could not stop",
      ]);
      setStatus("error");
    }
  }

  async function select(name: string) {
    const wasRunning = status === "running";
    setSelected(name);
    if (wasRunning) {
      await backend.stopStream();
      await launch(name);
    }
  }

  return { selected, status, diagnostics, play: launch, stop, select };
}
