import type { TwitchSearchChannel } from "../../domain/twitch";

export function ChannelCard({
  channel,
  onOpen,
}: {
  channel: TwitchSearchChannel;
  onOpen: () => void;
}) {
  return (
    <button className="channel-card" onClick={onOpen}>
      <span className={channel.isLive ? "channel-state live" : "channel-state"}>
        {channel.isLive ? "Live" : "Offline"}
      </span>
      <strong>{channel.displayName}</strong>
      <span>
        {channel.gameName || channel.broadcasterLanguage.toUpperCase()}
      </span>
      <small>{channel.title || "View channel details"}</small>
    </button>
  );
}
