import { invoke } from '@tauri-apps/api/core';

export interface Theme {
  id: string;
  name: string;
  description: string;
  // Surface colors
  sidebar: string;
  nav: string;
  navHeader: string;
  content: string;
  editor: string;
  // Border colors
  border: string;
  borderHover: string;
  borderSubtle: string;
  // Text colors
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  textFaint: string;
  // Modal (always opaque)
  modalBg: string;
  // Glass properties
  glass: boolean;
}

const themes: Record<string, Theme> = {
  'dark-glass': {
    id: 'dark-glass',
    name: 'Dark Glass',
    description: 'Transparent with macOS vibrancy',
    sidebar: 'rgba(12,12,20,0.88)',
    nav: 'rgba(12,12,20,0.88)',
    navHeader: 'rgba(16,16,26,0.92)',
    content: 'rgba(14,14,24,0.88)',
    editor: 'rgba(12,12,20,0.88)',
    border: 'rgba(255,255,255,0.10)',
    borderHover: 'rgba(255,255,255,0.18)',
    borderSubtle: 'rgba(255,255,255,0.06)',
    textPrimary: '#e8e8f4',
    textSecondary: '#d0d0e4',
    textMuted: '#b0b0c8',
    textFaint: '#7878a0',
    modalBg: '#161628',
    glass: true,
  },
  'dark-solid': {
    id: 'dark-solid',
    name: 'Dark Solid',
    description: 'Opaque dark with purple tints',
    sidebar: '#12121f',
    nav: '#161629',
    navHeader: '#1c1c35',
    content: '#1e1e32',
    editor: '#151528',
    border: '#2d2d48',
    borderHover: '#3e3e62',
    borderSubtle: '#222238',
    textPrimary: '#e4e4f0',
    textSecondary: '#d0d0e4',
    textMuted: '#b0b0c8',
    textFaint: '#7878a0',
    modalBg: '#151528',
    glass: false,
  },
  'midnight': {
    id: 'midnight',
    name: 'Midnight',
    description: 'Pure black, zero distraction',
    sidebar: '#000000',
    nav: '#0a0a0a',
    navHeader: '#121212',
    content: '#0e0e0e',
    editor: '#080808',
    border: '#2a2a2a',
    borderHover: '#3a3a3a',
    borderSubtle: '#1c1c1c',
    textPrimary: '#e8e8e8',
    textSecondary: '#cccccc',
    textMuted: '#999999',
    textFaint: '#666666',
    modalBg: '#0e0e0e',
    glass: false,
  },
  'nord': {
    id: 'nord',
    name: 'Nord',
    description: 'Arctic blue-gray palette',
    sidebar: '#272d38',
    nav: '#2e3440',
    navHeader: '#3b4252',
    content: '#353d4b',
    editor: '#2e3440',
    border: '#4c566a',
    borderHover: '#5e6a82',
    borderSubtle: '#3b4252',
    textPrimary: '#eceff4',
    textSecondary: '#d8dee9',
    textMuted: '#a8b4c8',
    textFaint: '#7a8698',
    modalBg: '#2e3440',
    glass: false,
  },
  'light': {
    id: 'light',
    name: 'Light',
    description: 'Warm off-white, easy on the eyes',
    sidebar: '#f0f0ec',
    nav: '#f5f5f2',
    navHeader: '#eaeae6',
    content: '#fafaf8',
    editor: '#ffffff',
    border: '#e0ddd8',
    borderHover: '#ccc8c2',
    borderSubtle: '#eceae6',
    textPrimary: '#1a1a18',
    textSecondary: '#4a4a44',
    textMuted: '#8c8880',
    textFaint: '#b0a8a0',
    modalBg: '#ffffff',
    glass: false,
  },
};

export function applyTheme(themeId: string, accentColor?: string) {
  const theme = themes[themeId];
  if (!theme) return;

  const root = document.documentElement;
  root.style.setProperty('--s', theme.sidebar);
  root.style.setProperty('--n', theme.nav);
  root.style.setProperty('--n2', theme.navHeader);
  root.style.setProperty('--c', theme.content);
  root.style.setProperty('--e', theme.editor);
  root.style.setProperty('--b1', theme.border);
  root.style.setProperty('--b2', theme.borderHover);
  root.style.setProperty('--b-subtle', theme.borderSubtle);
  root.style.setProperty('--t1', theme.textPrimary);
  root.style.setProperty('--t2', theme.textSecondary);
  root.style.setProperty('--t3', theme.textMuted);
  root.style.setProperty('--t4', theme.textFaint);
  root.style.setProperty('--modal-bg', theme.modalBg);

  if (accentColor) {
    root.style.setProperty('--acc', accentColor);
  }

  // Light mode class
  if (themeId === 'light') {
    document.body.classList.add('light-mode');
    root.style.setProperty('--ok', '#16a34a');
    root.style.setProperty('--warn', '#d97706');
    root.style.setProperty('--err', '#dc2626');
  } else {
    document.body.classList.remove('light-mode');
    root.style.setProperty('--ok', '#1dc880');
    root.style.setProperty('--warn', '#f5a623');
    root.style.setProperty('--err', '#f04444');
  }

  // Glass-specific: add backdrop-filter class and set vibrancy
  if (theme.glass) {
    document.body.classList.add('glass-mode');
    setVibrancy('sidebar');
  } else {
    document.body.classList.remove('glass-mode');
    setVibrancy('none');
  }
}

export function getThemes() { return Object.values(themes); }
export function getTheme(id: string) { return themes[id]; }

export async function setVibrancy(material: string) {
  try {
    await invoke('set_vibrancy', { material });
  } catch (e) {
    console.warn('Vibrancy not supported:', e);
  }
}

// xterm.js terminal themes matched to each app theme
export const TERMINAL_THEMES: Record<string, Record<string, string>> = {
  'dark-glass': {
    background: '#0d0d18',
    foreground: '#e8e8f4',
    cursor: '#6366f1',
    cursorAccent: '#0d0d18',
    selectionBackground: 'rgba(99,102,241,0.3)',
    black: '#484858', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
    blue: '#58a6ff', magenta: '#d2a8ff', cyan: '#56d4dd', white: '#e6edf3',
    brightBlack: '#6e7681', brightRed: '#ffa198', brightGreen: '#56d364', brightYellow: '#e3b341',
    brightBlue: '#79c0ff', brightMagenta: '#d2a8ff', brightCyan: '#76e4f7', brightWhite: '#ffffff',
  },
  'dark-solid': {
    background: '#12121f',
    foreground: '#e4e4f0',
    cursor: '#6366f1',
    cursorAccent: '#12121f',
    selectionBackground: 'rgba(99,102,241,0.3)',
    black: '#484858', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
    blue: '#58a6ff', magenta: '#d2a8ff', cyan: '#56d4dd', white: '#e6edf3',
    brightBlack: '#6e7681', brightRed: '#ffa198', brightGreen: '#56d364', brightYellow: '#e3b341',
    brightBlue: '#79c0ff', brightMagenta: '#d2a8ff', brightCyan: '#76e4f7', brightWhite: '#ffffff',
  },
  'midnight': {
    background: '#000000',
    foreground: '#e8e8e8',
    cursor: '#6366f1',
    cursorAccent: '#000000',
    selectionBackground: 'rgba(99,102,241,0.25)',
    black: '#3a3a3a', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
    blue: '#58a6ff', magenta: '#d2a8ff', cyan: '#56d4dd', white: '#e8e8e8',
    brightBlack: '#666666', brightRed: '#ffa198', brightGreen: '#56d364', brightYellow: '#e3b341',
    brightBlue: '#79c0ff', brightMagenta: '#d2a8ff', brightCyan: '#76e4f7', brightWhite: '#ffffff',
  },
  'nord': {
    background: '#2e3440',
    foreground: '#eceff4',
    cursor: '#88c0d0',
    cursorAccent: '#2e3440',
    selectionBackground: 'rgba(136,192,208,0.25)',
    black: '#3b4252', red: '#bf616a', green: '#a3be8c', yellow: '#ebcb8b',
    blue: '#81a1c1', magenta: '#b48ead', cyan: '#88c0d0', white: '#eceff4',
    brightBlack: '#4c566a', brightRed: '#bf616a', brightGreen: '#a3be8c', brightYellow: '#ebcb8b',
    brightBlue: '#81a1c1', brightMagenta: '#b48ead', brightCyan: '#8fbcbb', brightWhite: '#eceff4',
  },
  'light': {
    background: '#ffffff',
    foreground: '#1a1a18',
    cursor: '#6366f1',
    cursorAccent: '#ffffff',
    selectionBackground: 'rgba(99,102,241,0.15)',
    black: '#24292f', red: '#cf222e', green: '#116329', yellow: '#4d2d00',
    blue: '#0550ae', magenta: '#8250df', cyan: '#1b7c83', white: '#6e7781',
    brightBlack: '#57606a', brightRed: '#a40e26', brightGreen: '#1a7f37', brightYellow: '#633c01',
    brightBlue: '#0969da', brightMagenta: '#8250df', brightCyan: '#3192aa', brightWhite: '#24292f',
  },
};

/** Get xterm theme for a given app theme, with accent color as cursor */
export function getTerminalTheme(themeId: string, accentColor?: string): Record<string, string> {
  const termTheme = TERMINAL_THEMES[themeId] || TERMINAL_THEMES['dark-glass'];
  if (accentColor) {
    return { ...termTheme, cursor: accentColor };
  }
  return { ...termTheme };
}

// Method colors for HTTP methods
export const METHOD_COLORS: Record<string, { color: string; bg: string }> = {
  GET:    { color: '#60a5fa', bg: '#162640' },
  POST:   { color: '#34d399', bg: '#0d2818' },
  PUT:    { color: '#fbbf24', bg: '#1c1808' },
  PATCH:  { color: '#c4b5fd', bg: '#1e162e' },
  DELETE: { color: '#f87171', bg: '#2a1010' },
};

export const METHOD_COLORS_LIGHT: Record<string, { color: string; bg: string }> = {
  GET:    { color: '#2563eb', bg: '#eef4ff' },
  POST:   { color: '#16a34a', bg: '#ecfdf5' },
  PUT:    { color: '#d97706', bg: '#fefce8' },
  PATCH:  { color: '#7c3aed', bg: '#f5f3ff' },
  DELETE: { color: '#dc2626', bg: '#fef2f2' },
};
