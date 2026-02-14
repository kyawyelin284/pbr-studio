import { createContext, useContext, useEffect, useCallback, useState, type ReactNode } from 'react';

const STORAGE_KEY = 'pbr-studio-preferences';

export type ThemeMode = 'dark' | 'light';

export type LayoutPreset = 'compact' | 'normal' | 'spacious';

export interface ValidationColors {
  critical: string;
  warning: string;
  pass: string;
}

export interface Preferences {
  theme: ThemeMode;
  layout: LayoutPreset;
  validationColors: ValidationColors;
  /** Max undo/redo steps (10–100). Stored locally. */
  undoHistorySize: number;
  /** Custom plugin directory for validation rules and presets. Empty = use defaults. */
  pluginsDir: string;
}

const DEFAULT_VALIDATION_COLORS: ValidationColors = {
  critical: '#ef4444',
  warning: '#eab308',
  pass: '#22c55e',
};

const DEFAULT_PREFERENCES: Preferences = {
  theme: 'dark',
  layout: 'normal',
  validationColors: DEFAULT_VALIDATION_COLORS,
  undoHistorySize: 50,
  pluginsDir: '',
};

function loadFromStorage(): Preferences {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_PREFERENCES;
    const parsed = JSON.parse(raw) as Partial<Preferences>;
    return {
      theme: parsed.theme ?? DEFAULT_PREFERENCES.theme,
      layout: parsed.layout ?? DEFAULT_PREFERENCES.layout,
      validationColors: {
        critical: parsed.validationColors?.critical ?? DEFAULT_VALIDATION_COLORS.critical,
        warning: parsed.validationColors?.warning ?? DEFAULT_VALIDATION_COLORS.warning,
        pass: parsed.validationColors?.pass ?? DEFAULT_VALIDATION_COLORS.pass,
      },
      undoHistorySize: Math.min(100, Math.max(10, parsed.undoHistorySize ?? 50)),
      pluginsDir: typeof parsed.pluginsDir === 'string' ? parsed.pluginsDir : '',
    };
  } catch {
    return DEFAULT_PREFERENCES;
  }
}

function saveToStorage(prefs: Preferences): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch {
    // ignore
  }
}

interface PreferencesContextValue {
  preferences: Preferences;
  setTheme: (theme: ThemeMode) => void;
  setLayout: (layout: LayoutPreset) => void;
  setValidationColors: (colors: Partial<ValidationColors>) => void;
  setUndoHistorySize: (size: number) => void;
  setPluginsDir: (dir: string) => void;
  resetToDefaults: () => void;
}

const PreferencesContext = createContext<PreferencesContextValue | null>(null);

export function PreferencesProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState<Preferences>(loadFromStorage);

  useEffect(() => {
    saveToStorage(preferences);
    // Apply theme to document
    document.documentElement.setAttribute('data-theme', preferences.theme);
    document.documentElement.setAttribute('data-layout', preferences.layout);
    // Apply validation colors as CSS variables
    document.documentElement.style.setProperty('--validation-critical', preferences.validationColors.critical);
    document.documentElement.style.setProperty('--validation-warning', preferences.validationColors.warning);
    document.documentElement.style.setProperty('--validation-pass', preferences.validationColors.pass);
    document.documentElement.style.setProperty('--severity-error', preferences.validationColors.critical);
    document.documentElement.style.setProperty('--severity-warning', preferences.validationColors.warning);
    document.documentElement.style.setProperty('--score-good', preferences.validationColors.pass);
  }, [preferences]);

  const setTheme = useCallback((theme: ThemeMode) => {
    setPreferences((p) => ({ ...p, theme }));
  }, []);

  const setLayout = useCallback((layout: LayoutPreset) => {
    setPreferences((p) => ({ ...p, layout }));
  }, []);

  const setValidationColors = useCallback((colors: Partial<ValidationColors>) => {
    setPreferences((p) => ({
      ...p,
      validationColors: { ...p.validationColors, ...colors },
    }));
  }, []);

  const setUndoHistorySize = useCallback((size: number) => {
    setPreferences((p) => ({
      ...p,
      undoHistorySize: Math.min(100, Math.max(10, size)),
    }));
  }, []);

  const setPluginsDir = useCallback((dir: string) => {
    setPreferences((p) => ({ ...p, pluginsDir: dir }));
  }, []);

  const resetToDefaults = useCallback(() => {
    setPreferences(DEFAULT_PREFERENCES);
  }, []);

  return (
    <PreferencesContext.Provider
      value={{
        preferences,
        setTheme,
        setLayout,
        setValidationColors,
        setUndoHistorySize,
        setPluginsDir,
        resetToDefaults,
      }}
    >
      {children}
    </PreferencesContext.Provider>
  );
}

export function usePreferences() {
  const ctx = useContext(PreferencesContext);
  if (!ctx) throw new Error('usePreferences must be used within PreferencesProvider');
  return ctx;
}
