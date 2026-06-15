/**
 * 会话 store(zustand)—— 空骨架。
 *
 * 后续里程碑(M4/M5/M6)用于承载运行时易变状态:当前翻译会话、
 * 选区/截图进度、popup 状态机等。M1 仅占位,保证目录与导出存在。
 */
import { create } from "zustand";
import type { TriggerKind } from "@/lib/types";

interface SessionState {
  /** 当前活跃的触发类型(无活跃会话时为 null) */
  activeTrigger: TriggerKind | null;
  setActiveTrigger: (trigger: TriggerKind | null) => void;
  // TODO(M4/M5/M6): 翻译会话、进度、popup 状态机
}

export const useSessionStore = create<SessionState>((set) => ({
  activeTrigger: null,
  setActiveTrigger: (activeTrigger) => set({ activeTrigger }),
}));

export type { SessionState };
export default useSessionStore;
