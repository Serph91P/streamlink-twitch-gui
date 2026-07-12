import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { StreamVariant } from "../../domain/stream";
import { QualityPicker, selectBestVariant } from "./QualityPicker";

export const variants: StreamVariant[] = [
  {
    name: "1080p60-h264",
    resolution: { width: 1920, height: 1080 },
    fps: 60,
    codec: "h264",
    aliases: [],
  },
  {
    name: "1440p60-hevc",
    resolution: { width: 2560, height: 1440 },
    fps: 60,
    codec: "h265",
    aliases: ["best"],
  },
  {
    name: "1440p60-av1",
    resolution: { width: 2560, height: 1440 },
    fps: 60,
    codec: "av1",
    aliases: [],
  },
  {
    name: "future-ultra",
    resolution: { width: 5120, height: 2160 },
    codec: "unknown",
    aliases: [],
  },
];

describe("QualityPicker", () => {
  it("groups arbitrary backend variants by resolution and codec", () => {
    const onChange = vi.fn();
    render(
      <QualityPicker
        variants={variants}
        selected="1440p60-hevc"
        onChange={onChange}
      />,
    );

    expect(screen.getByRole("group", { name: "1440p" })).toBeVisible();
    expect(screen.getByRole("radio", { name: /HEVC/ })).toBeChecked();
    expect(screen.getByRole("radio", { name: /AV1/ })).toBeVisible();
    expect(screen.getByRole("group", { name: "2160p" })).toBeVisible();
    fireEvent.click(screen.getByRole("radio", { name: /Unknown codec/ }));
    expect(onChange).toHaveBeenCalledWith("future-ultra");
  });

  it("defaults to the best variant within codec and height constraints", () => {
    expect(
      selectBestVariant(variants, {
        preferredCodec: "h264",
        maximumHeight: 1080,
      })?.name,
    ).toBe("1080p60-h264");
    expect(selectBestVariant(variants, { maximumHeight: 1440 })?.name).toBe(
      "1440p60-hevc",
    );
  });

  it("shows compatibility guidance for modern codecs", () => {
    const { rerender } = render(
      <QualityPicker
        variants={variants}
        selected="1440p60-hevc"
        onChange={() => undefined}
      />,
    );
    expect(screen.getByRole("note")).toHaveTextContent("HEVC");
    rerender(
      <QualityPicker
        variants={variants}
        selected="1440p60-av1"
        onChange={() => undefined}
      />,
    );
    expect(screen.getByRole("note")).toHaveTextContent("AV1");
  });
});
