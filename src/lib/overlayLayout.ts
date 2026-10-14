export function fitOverlayFont(text: string, width: number, height: number, measure: (text: string, size: number) => number): number {
  let size = Math.max(8, Math.min(32, height * 0.8));
  while (size > 8 && measure(text, size) > Math.max(0, width - 4)) size -= 1;
  return Math.max(8, size);
}
