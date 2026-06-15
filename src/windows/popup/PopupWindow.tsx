/**
 * 翻译结果弹框窗口(占位)。
 *
 * TODO(M4): 语言行 + 关闭 / 原文(可折叠)/ 译文 / 工具栏;
 *           loading 骨架 / success / error 三态;动态高度 window.setSize;
 *           点击外部 / ESC 隐藏;复制译文。(spec 09 §3)
 */
export function PopupWindow() {
  return (
    <div className="flex min-h-[120px] items-center justify-center rounded-lg bg-bg-elevated/95 p-3 text-sm text-fg-muted backdrop-blur">
      Lenz popup
    </div>
  );
}

export default PopupWindow;
