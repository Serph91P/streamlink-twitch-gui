import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { BrowserBackend } from "./api/backend";
import { App } from "./App";
import { liveStream, topGame } from "./test/fixtures/twitch";

describe("browsing experience", () => {
  it("renders live streams and exposes every browse destination as navigation", async () => {
    const backend = new BrowserBackend({
      streams: async () => ({ items: [liveStream] }),
      topGames: async () => ({ items: [topGame] }),
    });
    render(<App backend={backend} />);

    expect(
      await screen.findByRole("heading", { name: "Live now" }),
    ).toBeVisible();
    expect(
      await screen.findByRole("link", { name: /Signal Noise/ }),
    ).toBeVisible();
    for (const name of [
      "Live",
      "Following",
      "Channels",
      "Categories",
      "Search",
      "Settings",
    ]) {
      expect(screen.getByRole("button", { name })).toBeVisible();
    }
  });

  it("loads followed streams, followed channels and category routes", async () => {
    const backend = new BrowserBackend({
      getSession: async () => ({
        status: "authenticated",
        user: {
          id: "user-1",
          login: "viewer",
          displayName: "Viewer",
          profileImageUrl: "",
        },
        expiresAt: "2030-01-01T00:00:00Z",
      }),
      followedStreams: async () => ({ items: [liveStream] }),
      followedChannels: async () => ({
        items: [
          {
            broadcasterId: "user-1",
            broadcasterLogin: "signalnoise",
            broadcasterName: "Signal Noise",
            followedAt: "2026-01-01T00:00:00Z",
          },
        ],
      }),
      topGames: async () => ({ items: [topGame] }),
    });
    render(<App backend={backend} />);

    fireEvent.click(screen.getByRole("button", { name: "Following" }));
    expect(
      await screen.findByRole("heading", { name: "Following live" }),
    ).toBeVisible();
    expect(
      await screen.findByRole("link", { name: /Signal Noise/ }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Channels" }));
    expect(
      await screen.findByRole("heading", { name: "Followed channels" }),
    ).toBeVisible();
    expect(await screen.findByText("@signalnoise")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Categories" }));
    expect(
      await screen.findByRole("heading", { name: "Top categories" }),
    ).toBeVisible();
    expect(await screen.findByText("Science & Technology")).toBeVisible();
  });

  it("renders loading, empty, offline and actionable error states", async () => {
    let resolveStreams: ((value: { items: [] }) => void) | undefined;
    const pending = new Promise<{ items: [] }>((resolve) => {
      resolveStreams = resolve;
    });
    const backend = new BrowserBackend({ streams: async () => pending });
    const { unmount } = render(<App backend={backend} />);
    expect(screen.getByRole("status")).toHaveTextContent(
      "Tuning the live feed",
    );

    resolveStreams?.({ items: [] });
    expect(await screen.findByText("No live channels found")).toBeVisible();

    const failing = new BrowserBackend({
      streams: async () => ({ items: [] }),
      searchChannels: async () => ({
        items: [
          {
            broadcasterLanguage: "en",
            broadcasterLogin: "signalnoise",
            displayName: "Signal Noise",
            gameId: "",
            gameName: "",
            id: "user-1",
            isLive: false,
            tags: [],
            thumbnailUrl: "",
            title: "",
            startedAt: "",
          },
        ],
      }),
    });
    unmount();
    render(<App backend={failing} />);
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    fireEvent.change(screen.getByRole("searchbox"), {
      target: { value: "signalnoise" },
    });
    fireEvent.submit(screen.getByRole("search"));
    fireEvent.click(
      await screen.findByRole("button", { name: /Signal Noise/ }),
    );

    await waitFor(() =>
      expect(screen.getByText("Signal Noise is offline")).toBeVisible(),
    );
    expect(screen.getByRole("button", { name: "Try again" })).toBeVisible();
  });
});
