import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { TauriBackend, type AppBackend } from "./api/backend";
import { AppLayout, type RouteName } from "./components/layout/AppLayout";
import type { Settings } from "./domain/settings";
import { BrowseRoute } from "./routes/BrowseRoute";

export function App({
  backend = new TauriBackend(),
}: {
  backend?: AppBackend;
}) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { retry: false, refetchOnWindowFocus: false },
        },
      }),
  );
  const [route, setRoute] = useState<RouteName>("live");
  const [settings, setSettings] = useState<Settings>();
  const [settingsError, setSettingsError] = useState("");

  useEffect(() => {
    let current = true;
    void backend.loadSettings().then(
      (loaded) => {
        if (!current) return;
        setSettings(loaded);
      },
      (reason: unknown) => {
        if (current) {
          setSettingsError(
            reason instanceof Error
              ? reason.message
              : "Could not load settings",
          );
        }
      },
    );
    return () => {
      current = false;
    };
  }, [backend]);

  useEffect(() => {
    if (!settings) return;
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.lang = settings.language;
  }, [settings]);

  if (settingsError) return <p role="alert">{settingsError}</p>;
  if (!settings) return <p role="status">Loading settings...</p>;

  return (
    <QueryClientProvider client={queryClient}>
      <AppLayout route={route} onNavigate={setRoute}>
        <BrowseRoute
          route={route}
          backend={backend}
          settings={settings}
          onSettingsSaved={setSettings}
          onNavigate={setRoute}
        />
      </AppLayout>
    </QueryClientProvider>
  );
}
