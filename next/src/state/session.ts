import { createStore } from "zustand/vanilla";

export type LoginStatus = "idle" | "waiting" | "complete";

interface SessionUiState {
  loginStatus: LoginStatus;
  setLoginStatus: (status: LoginStatus) => void;
  reset: () => void;
}

export const sessionUiStore = createStore<SessionUiState>()((set) => ({
  loginStatus: "idle",
  setLoginStatus: (loginStatus) => set({ loginStatus }),
  reset: () => set({ loginStatus: "idle" }),
}));
