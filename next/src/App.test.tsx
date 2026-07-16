import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { BrowserBackend, defaultSettings } from "./api/backend";
import { App } from "./App";
import { liveStream, topGame } from "./test/fixtures/twitch";

const openUrl = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

const authenticatedSession = {
  status: "authenticated" as const,
  user: {
    id: "user-1",
    login: "viewer",
    displayName: "Viewer",
    profileImageUrl: "",
  },
  expiresAt: "2030-01-01T00:00:00Z",
};

afterEach(() => {
  openUrl.mockReset();
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("lang");
});

describe("application settings", () => {
  it("applies the persisted theme and language at startup", async () => {
    const backend = new BrowserBackend({
      loadSettings: async () => ({
        ...defaultSettings,
        theme: "light",
        language: "de",
      }),
    });

    render(<App backend={backend} />);

    await waitFor(() =>
      expect(document.documentElement).toHaveAttribute("data-theme", "light"),
    );
    expect(document.documentElement).toHaveAttribute("lang", "de");
  });

  it("applies successful settings saves immediately from one settings source", async () => {
    const loadSettings = vi.fn(async () => defaultSettings);
    const backend = new BrowserBackend({
      loadSettings,
      saveSettings: async (settings) => settings,
    });
    render(<App backend={backend} />);

    fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
    fireEvent.change(await screen.findByLabelText("Theme"), {
      target: { value: "light" },
    });
    fireEvent.change(screen.getByLabelText("Language"), {
      target: { value: "fr" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    await waitFor(() =>
      expect(document.documentElement).toHaveAttribute("data-theme", "light"),
    );
    expect(document.documentElement).toHaveAttribute("lang", "fr");
    expect(loadSettings).toHaveBeenCalledOnce();
  });
});

describe("browsing experience", () => {
  it("renders live streams and exposes every browse destination as navigation", async () => {
    const backend = new BrowserBackend({
      getSession: async () => authenticatedSession,
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
        ...authenticatedSession,
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

    fireEvent.click(await screen.findByRole("button", { name: "Following" }));
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
    const backend = new BrowserBackend({
      getSession: async () => authenticatedSession,
      streams: async () => pending,
    });
    const { unmount } = render(<App backend={backend} />);
    expect(await screen.findByText("Tuning the live feed...")).toBeVisible();

    resolveStreams?.({ items: [] });
    expect(await screen.findByText("No live channels found")).toBeVisible();

    const failing = new BrowserBackend({
      getSession: async () => authenticatedSession,
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
    fireEvent.click(await screen.findByRole("button", { name: "Search" }));
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

  it("guides anonymous users to sign in without firing Helix queries", async () => {
    const streams = vi.fn(async () => ({ items: [] }));
    const topGames = vi.fn(async () => ({ items: [] }));
    const followedStreams = vi.fn(async () => ({ items: [] }));
    const followedChannels = vi.fn(async () => ({ items: [] }));
    const searchChannels = vi.fn(async () => ({ items: [] }));
    const searchCategories = vi.fn(async () => ({ items: [] }));
    const backend = new BrowserBackend({
      getSession: async () => ({ status: "anonymous" }),
      streams,
      topGames,
      followedStreams,
      followedChannels,
      searchChannels,
      searchCategories,
    });
    render(<App backend={backend} />);

    expect(
      await screen.findByText("Sign in with Twitch to browse"),
    ).toBeVisible();
    for (const name of [
      "Following",
      "Channels",
      "Categories",
      "Search",
      "Live",
    ]) {
      fireEvent.click(screen.getByRole("button", { name }));
      expect(
        await screen.findByRole("button", { name: "Sign in with Twitch" }),
      ).toBeVisible();
    }

    expect(streams).not.toHaveBeenCalled();
    expect(topGames).not.toHaveBeenCalled();
    expect(followedStreams).not.toHaveBeenCalled();
    expect(followedChannels).not.toHaveBeenCalled();
    expect(searchChannels).not.toHaveBeenCalled();
    expect(searchCategories).not.toHaveBeenCalled();
  });

  it("waits for restored startup credentials before loading Twitch data", async () => {
    let restoreSession!: (session: typeof authenticatedSession) => void;
    const getSession = vi.fn(
      () =>
        new Promise<typeof authenticatedSession>((resolve) => {
          restoreSession = resolve;
        }),
    );
    const streams = vi.fn(async () => ({ items: [liveStream] }));
    const backend = new BrowserBackend({ getSession, streams });
    render(<App backend={backend} />);

    expect(await screen.findByText("Checking Twitch session...")).toBeVisible();
    expect(streams).not.toHaveBeenCalled();

    restoreSession(authenticatedSession);

    expect(
      await screen.findByRole("link", { name: /Signal Noise/ }),
    ).toBeVisible();
    expect(getSession).toHaveBeenCalledOnce();
    expect(streams).toHaveBeenCalledOnce();
    expect(
      screen.queryByText("Sign in with Twitch to browse"),
    ).not.toBeInTheDocument();
  });

  it("completes device authorization, refreshes Twitch data, and signs out", async () => {
    const pollTwitchLogin = vi
      .fn()
      .mockResolvedValueOnce({ status: "anonymous" })
      .mockResolvedValueOnce(authenticatedSession);
    const signOut = vi.fn(async () => undefined);
    const streams = vi.fn(async () => ({ items: [liveStream] }));
    const backend = new BrowserBackend({
      getSession: async () => ({ status: "anonymous" }),
      beginTwitchLogin: async () => ({
        verificationUri: "https://www.twitch.tv/activate",
        userCode: "ABCD-EFGH",
        expiresInSeconds: 10,
        pollingIntervalSeconds: 1,
      }),
      pollTwitchLogin,
      signOut,
      streams,
    });
    render(<App backend={backend} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "Sign in with Twitch" }),
    );
    expect(await screen.findByText("ABCD-EFGH")).toBeVisible();
    expect(screen.getByText("https://www.twitch.tv/activate")).toBeVisible();
    expect(openUrl).toHaveBeenCalledWith("https://www.twitch.tv/activate");

    expect(
      await screen.findByText("Signed in as Viewer", {}, { timeout: 3_000 }),
    ).toBeVisible();
    expect(pollTwitchLogin).toHaveBeenCalledTimes(2);
    expect(
      await screen.findByRole("link", { name: /Signal Noise/ }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    await waitFor(() => expect(signOut).toHaveBeenCalledOnce());
    expect(
      await screen.findByRole("button", { name: "Sign in with Twitch" }),
    ).toBeVisible();
  });

  it("expires and cancels device authorization without polling loops", async () => {
    const pollTwitchLogin = vi.fn(async () => ({
      status: "anonymous" as const,
    }));
    const backend = new BrowserBackend({
      getSession: async () => ({ status: "anonymous" }),
      beginTwitchLogin: async () => ({
        verificationUri: "https://www.twitch.tv/activate",
        userCode: "SHORT-LIVED",
        expiresInSeconds: 1,
        pollingIntervalSeconds: 1,
      }),
      pollTwitchLogin,
    });
    const { unmount } = render(<App backend={backend} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "Sign in with Twitch" }),
    );
    expect(
      await screen.findByText(
        "Twitch authorization expired",
        {},
        { timeout: 2_000 },
      ),
    ).toBeVisible();
    expect(pollTwitchLogin).not.toHaveBeenCalled();

    unmount();
    render(<App backend={backend} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Sign in with Twitch" }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    expect(screen.queryByText("SHORT-LIVED")).not.toBeInTheDocument();
  });

  it("refuses to open a verification URL outside the Twitch origin", async () => {
    const backend = new BrowserBackend({
      getSession: async () => ({ status: "anonymous" }),
      beginTwitchLogin: async () => ({
        verificationUri: "https://example.test/activate",
        userCode: "UNTRUSTED",
        expiresInSeconds: 600,
        pollingIntervalSeconds: 5,
      }),
    });
    render(<App backend={backend} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "Sign in with Twitch" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "unsupported verification URL",
    );
    expect(openUrl).not.toHaveBeenCalled();
    expect(screen.queryByText("UNTRUSTED")).not.toBeInTheDocument();
  });
});
