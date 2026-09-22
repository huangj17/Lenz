import { create } from "zustand";
import type { Config, DeepPartial } from "@/lib/types";
import { configApi } from "@/lib/ipc";
import { createDefaultConfig } from "@/lib/default-config";
import { SerialQueue } from "@/lib/serialQueue";
import { errorMessage } from "@/lib/errors";

type ConfigPatch = DeepPartial<Config> | ((current: Config) => DeepPartial<Config>);
interface ConfigState {
  config: Config;
  loaded: boolean;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  patch: (patch: ConfigPatch) => Promise<boolean>;
  reset: () => Promise<boolean>;
  setError: (error: unknown) => void;
}

const queue = new SerialQueue();
export const useConfigStore = create<ConfigState>((set) => ({
  config: createDefaultConfig(),
  loaded: false,
  loading: false,
  error: null,
  setError: (error) => set({ error: errorMessage(error) }),
  load: () => queue.run(async () => {
    set({ loading: true });
    try {
      set({ config: await configApi.get(), loaded: true });
    } catch (err) {
      set({ error: errorMessage(err) });
    } finally {
      set({ loading: false });
    }
  }),
  patch: (patch) => queue.run(async () => {
    try {
      // Resolve array edits only when prior saves have completed, against the
      // backend snapshot so edits in other windows are also retained.
      const next = typeof patch === "function" ? patch(await configApi.get()) : patch;
      set({ config: await configApi.patch(next), loaded: true, error: null });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  }),
  reset: () => queue.run(async () => {
    try {
      set({ config: await configApi.reset(), loaded: true, error: null });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  }),
}));

export const selectConfig = (s: ConfigState) => s.config;
export type { ConfigState };
export default useConfigStore;
