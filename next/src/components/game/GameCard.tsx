import type { TwitchGame } from "../../domain/twitch";

export function GameCard({ game }: { game: TwitchGame }) {
  const image = game.boxArtUrl
    .replace("{width}", "285")
    .replace("{height}", "380");
  return (
    <article className="game-card">
      <div className="game-art">
        {image.startsWith("https://") ? <img src={image} alt="" /> : null}
      </div>
      <h3>{game.name}</h3>
      <p>Browse live channels</p>
    </article>
  );
}
