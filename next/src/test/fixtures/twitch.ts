import type { TwitchGame, TwitchStream } from "../../domain/twitch";

export const liveStream: TwitchStream = {
  id: "stream-1",
  userId: "user-1",
  userLogin: "signalnoise",
  userName: "Signal Noise",
  gameId: "game-1",
  gameName: "Science & Technology",
  title: "Building a tiny synthesizer",
  viewerCount: 1842,
  startedAt: "2026-07-12T10:00:00Z",
  thumbnailUrl:
    "https://static-cdn.jtvnw.net/previews-ttv/live_user_signalnoise-640x360.jpg",
  isMature: false,
};

export const topGame: TwitchGame = {
  id: "game-1",
  name: "Science & Technology",
  boxArtUrl: "https://static-cdn.jtvnw.net/ttv-boxart/509670-285x380.jpg",
};
