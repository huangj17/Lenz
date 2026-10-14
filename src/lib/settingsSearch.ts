export type SettingsSection = "general" | "shortcuts" | "translate" | "ocr" | "about";
export interface SettingsSearchItem { id: string; section: SettingsSection; label: string; keywords: string }
export function searchSettings(items: SettingsSearchItem[], query: string): SettingsSearchItem[] {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (!terms.length) return [];
  return items.filter((item) => terms.every((term) => (item.label + " " + item.keywords).toLocaleLowerCase().includes(term)));
}
export function moveLanguage(languages: string[], language: string, target: string): string[] {
  const from = languages.indexOf(language), to = languages.indexOf(target);
  if (from < 0 || to < 0 || from === to) return languages;
  const next = [...languages];
  next.splice(from, 1);
  next.splice(to, 0, language);
  return next;
}
