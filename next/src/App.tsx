import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

import { TauriBackend, type AppBackend } from "./api/backend";
import { AppLayout, type RouteName } from "./components/layout/AppLayout";
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

  return (
    <QueryClientProvider client={queryClient}>
      <AppLayout route={route} onNavigate={setRoute}>
        <BrowseRoute route={route} backend={backend} onNavigate={setRoute} />
      </AppLayout>
    </QueryClientProvider>
  );
}
