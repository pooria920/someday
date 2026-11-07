export const colorThemes = [
  { id: 'lime', name: 'Lime', description: 'Fresh & focused' },
  { id: 'violet', name: 'Violet', description: 'Soft & electric' },
  { id: 'ocean', name: 'Ocean', description: 'Cool & clear' },
  { id: 'rose', name: 'Rose', description: 'Warm & playful' },
  { id: 'amber', name: 'Amber', description: 'Golden & calm' },
  { id: 'notebook', name: 'Notebook', description: 'Ruled paper & ink' },
  { id: 'terminal', name: 'Terminal', description: 'Monospace & sharp edges' },
] as const;

export type ColorTheme = typeof colorThemes[number]['id'];
export interface ThemeChoice { light: boolean; palette: ColorTheme; }
