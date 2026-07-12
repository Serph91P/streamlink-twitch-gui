import type { StreamCodec } from "../../domain/stream";

export function CodecPicker({
  value,
  onChange,
}: {
  value: StreamCodec | "auto";
  onChange: (value: StreamCodec | "auto") => void;
}) {
  return (
    <label>
      Preferred codec
      <select
        value={value}
        onChange={(event) =>
          onChange(event.target.value as StreamCodec | "auto")
        }
      >
        <option value="auto">Automatic</option>
        <option value="h264">H.264</option>
        <option value="h265">HEVC</option>
        <option value="av1">AV1</option>
      </select>
    </label>
  );
}
