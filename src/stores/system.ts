import { create } from "zustand";
import { systemApi } from "@/lib/ipc";
import type { SystemInfo, TestEngineResult, UpdateInfo } from "@/lib/types";

interface SystemState {
  info: SystemInfo | null;
  update: UpdateInfo | null;
  connections: Record<string, TestEngineResult>;
  load: () => Promise<void>;
  setUpdate: (update: UpdateInfo | null) => void;
  recordConnection: (id: string, result: TestEngineResult) => void;
  clearConnection: (id: string) => void;
}
let generation = 0;
export const useSystemStore = create<SystemState>((set) => ({
  info: null, update: null, connections: {},
  load: async () => {
    const current = ++generation;
    try {
      const info = await systemApi.info();
      if (current === generation) set({ info });
    } catch { /* IPC wrapper reports native errors; keep the last valid snapshot. */ }
  },
  setUpdate: (update) => set({ update }),
  recordConnection: (id, result) => set((state) => ({ connections: { ...state.connections, [id]: result } })),
  clearConnection: (id) => set((state) => {
    const connections = { ...state.connections };
    delete connections[id];
    return { connections };
  }),
}));
