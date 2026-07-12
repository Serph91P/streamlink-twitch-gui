import { createStore } from "zustand/vanilla";
import {
  createJSONStorage,
  persist,
  type PersistStorage,
} from "zustand/middleware";

export interface UiPreferences {
  sidebarCollapsed: boolean;
  searchHistory: string[];
  setSidebarCollapsed: (collapsed: boolean) => void;
  setSearchHistory: (history: string[]) => void;
}

export function createUiPreferencesStore(
  storage?: PersistStorage<UiPreferences>,
) {
  const selectedStorage =
    storage ?? createJSONStorage<UiPreferences>(() => localStorage);
  return createStore<UiPreferences>()(
    persist(
      (set) => ({
        sidebarCollapsed: false,
        searchHistory: [],
        setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
        setSearchHistory: (searchHistory) => set({ searchHistory }),
      }),
      {
        name: "streamlink-twitch-gui-ui",
        storage: selectedStorage,
        partialize: ({ sidebarCollapsed, searchHistory }) => ({
          sidebarCollapsed,
          searchHistory,
          setSidebarCollapsed: () => undefined,
          setSearchHistory: () => undefined,
        }),
      },
    ),
  );
}
