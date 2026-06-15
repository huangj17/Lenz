/**
 * 原图覆盖层窗口(占位)。
 *
 * TODO(M6): 监听 overlay://init 接收数据,按 bbox 绝对定位渲染每个译文块;
 *           字号自适应;鼠标穿透(由后端 set_ignore_cursor_events);
 *           ESC 关闭并 emit overlay://closed。(spec 09 §5 / spec 07)
 */
export function OverlayWindow() {
  return <div className="h-screen w-screen" />;
}

export default OverlayWindow;
