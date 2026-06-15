/**
 * 截图选区蒙层窗口(占位)。
 *
 * TODO(M4): 全屏 canvas 绘制半透遮罩 + 镂空选区 + 边框 + 尺寸提示;
 *           mousedown/move/up 绘制,ESC 取消;
 *           clientX/Y + innerPosition() → 虚拟屏逻辑坐标;
 *           emit capture://region-selected。(spec 09 §4)
 */
export function SelectionWindow() {
  return <div className="h-screen w-screen" />;
}

export default SelectionWindow;
