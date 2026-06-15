/**
 * 配置 store(zustand)。
 *
 * 持有完整 Config,提供 load / patch / reset,经 lib/ipc 调用后端
 * get_config / set_config / reset_config。
 *
 * 容错:M1 阶段后端命令可能为占位 / 不可用(浏览器内开发、命令未注册),
 * 此时回退到本地默认配置,保证 UI 可渲染、不抛未捕获异常。
 */
import { create } from "zustand";
import type { Config, DeepPartial } from "@/lib/types";
import { configApi } from "@/lib/ipc";
import { createDefaultConfig } from "@/lib/default-config";

interface ConfigState {
  config: Config;
  /** 是否已成功从后端加载过(false = 仍是本地默认兜底) */
  loaded: boolean;
  loading: boolean;
  /** 最近一次加载/写入的错误信息(已被 ipc 分发,这里仅供 UI 提示) */
  error: string | null;

  /** 从后端加载;失败则保留当前(默认)配置 */
  load: () => Promise<void>;
  /** JSON Merge Patch 局部更新;成功后用后端返回的完整 config 覆盖 */
  patch: (patch: DeepPartial<Config>) => Promise<void>;
  /** 重置为后端默认配置 */
  reset: () => Promise<void>;
  /** 由 config://changed 事件等外部来源直接替换(不回写后端) */
  setConfig: (config: Config) => void;
}

export const useConfigStore = create<ConfigState>((set) => ({
  config: createDefaultConfig(),
  loaded: false,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const config = await configApi.get();
      set({ config, loaded: true, loading: false });
    } catch (err) {
      // ipc 已做全局分发;此处保留默认 config,仅记录提示信息
      set({
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  patch: async (patch) => {
    set({ error: null });
    try {
      const config = await configApi.patch(patch);
      set({ config, loaded: true });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  reset: async () => {
    set({ error: null });
    try {
      const config = await configApi.reset();
      set({ config, loaded: true });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  setConfig: (config) => set({ config }),
}));

/** 便捷选择器:当前完整配置 */
export const selectConfig = (s: ConfigState) => s.config;
export type { ConfigState };
export default useConfigStore;
