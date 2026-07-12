import { describe, expect, it } from "vitest";
import { createJSONStorage } from "zustand/middleware";

import { createUiPreferencesStore, type UiPreferences } from "./settings";

describe("UI preferences", () => {
  it("persists only non-secret display preferences", () => {
    const values = new Map<string, string>();
    const storage = createJSONStorage<UiPreferences>(() => ({
      getItem: (key) => values.get(key) ?? null,
      setItem: (key, value) => {
        values.set(key, value);
      },
      removeItem: (key) => {
        values.delete(key);
      },
    }));
    const store = createUiPreferencesStore(storage);

    store.getState().setSidebarCollapsed(true);
    store.getState().setSearchHistory(["chess"]);

    const serialized = [...values.values()].join("");
    expect(serialized).toContain("sidebarCollapsed");
    expect(serialized).not.toMatch(/token|credential|secret|session/i);
  });
});
