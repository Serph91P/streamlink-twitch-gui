import type { ReactNode } from "react";

export type RouteName =
  "live" | "following" | "channels" | "categories" | "search" | "settings";

const destinations: Array<[RouteName, string]> = [
  ["live", "Live"],
  ["following", "Following"],
  ["channels", "Channels"],
  ["categories", "Categories"],
  ["search", "Search"],
  ["settings", "Settings"],
];

export function AppLayout({
  route,
  onNavigate,
  children,
}: {
  route: RouteName;
  onNavigate: (route: RouteName) => void;
  children: ReactNode;
}) {
  return (
    <div className="app-shell">
      <header className="masthead">
        <button
          className="identity"
          onClick={() => onNavigate("live")}
          aria-label="Streamlink Twitch GUI home"
        >
          <span className="brand-mark" aria-hidden="true">
            S
          </span>
          <span>
            <strong>Streamlink</strong>
            <small>Twitch GUI</small>
          </span>
        </button>
        <p className="signal">
          <i aria-hidden="true" /> Broadcast monitor online
        </p>
      </header>
      <nav className="primary-nav" aria-label="Primary navigation">
        {destinations.map(([value, label]) => (
          <button
            key={value}
            aria-current={route === value ? "page" : undefined}
            onClick={() => onNavigate(value)}
          >
            {label}
          </button>
        ))}
      </nav>
      <main className="content" tabIndex={-1}>
        {children}
      </main>
    </div>
  );
}
