import type { StreamCodec, StreamVariant } from "../../domain/stream";

const codecNames: Record<StreamCodec, string> = {
  h264: "H.264",
  h265: "HEVC",
  av1: "AV1",
  unknown: "Unknown codec",
};

export function selectBestVariant(
  variants: StreamVariant[],
  constraints: { preferredCodec?: StreamCodec; maximumHeight?: number } = {},
): StreamVariant | undefined {
  const withinHeight = variants.filter(
    (variant) =>
      constraints.maximumHeight === undefined ||
      variant.resolution === undefined ||
      variant.resolution.height <= constraints.maximumHeight,
  );
  const preferred = constraints.preferredCodec
    ? withinHeight.filter(
        (variant) => variant.codec === constraints.preferredCodec,
      )
    : withinHeight;
  const candidates = preferred.length > 0 ? preferred : withinHeight;
  return [...candidates].sort((left, right) => {
    if (left.aliases.includes("best")) return -1;
    if (right.aliases.includes("best")) return 1;
    return (
      (right.resolution?.height ?? 0) - (left.resolution?.height ?? 0) ||
      (right.fps ?? 0) - (left.fps ?? 0) ||
      (right.bitrateKbps ?? 0) - (left.bitrateKbps ?? 0)
    );
  })[0];
}

export function QualityPicker({
  variants,
  selected,
  onChange,
}: {
  variants: StreamVariant[];
  selected?: string;
  onChange: (name: string) => void;
}) {
  const groups = variants.reduce<Map<string, StreamVariant[]>>(
    (result, variant) => {
      const key = variant.resolution
        ? `${variant.resolution.height}p`
        : "Other";
      result.set(key, [...(result.get(key) ?? []), variant]);
      return result;
    },
    new Map(),
  );
  const selectedVariant = variants.find((variant) => variant.name === selected);
  return (
    <div className="quality-picker">
      {[...groups].map(([resolution, options]) => (
        <fieldset key={resolution}>
          <legend>{resolution}</legend>
          {options.map((variant) => (
            <label key={variant.name} className="quality-option">
              <input
                type="radio"
                name="quality"
                checked={variant.name === selected}
                onChange={() => onChange(variant.name)}
              />
              <span>{codecNames[variant.codec ?? "unknown"]}</span>
              <small>{variant.fps ? `${variant.fps} fps` : variant.name}</small>
            </label>
          ))}
        </fieldset>
      ))}
      {selectedVariant?.codec === "h265" || selectedVariant?.codec === "av1" ? (
        <p role="note" className="compatibility-note">
          {codecNames[selectedVariant.codec]} needs support in your external
          player and hardware decoder.
        </p>
      ) : null}
    </div>
  );
}
