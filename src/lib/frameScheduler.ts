/** Coalesce pointer updates; draw reads the latest coordinates when the frame runs. */
export function frameScheduler(draw: () => void, request: (callback: FrameRequestCallback) => number, cancel: (id: number) => void) {
  let pending: number | null = null;
  const clear = () => {
    if (pending !== null) cancel(pending);
    pending = null;
  };
  return {
    schedule() {
      if (pending === null) pending = request(() => { pending = null; draw(); });
    },
    flush() { clear(); draw(); },
    cancel: clear,
  };
}
