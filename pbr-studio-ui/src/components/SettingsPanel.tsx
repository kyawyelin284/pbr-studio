import { useState, useEffect } from 'react';
import {
  usePreferences,
  type ThemeMode,
  type LayoutPreset,
} from '../context/PreferencesContext';

const APP_VERSION = '1.0.0';

interface SettingsPanelProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsPanel({ open, onClose }: SettingsPanelProps) {
  const { preferences, setTheme, setLayout, setValidationColors, setUndoHistorySize, setPluginsDir, resetToDefaults } = usePreferences();
  const [appVersion, setAppVersion] = useState(APP_VERSION);
  const [localCritical, setLocalCritical] = useState(preferences.validationColors.critical);
  const [localWarning, setLocalWarning] = useState(preferences.validationColors.warning);
  const [localPass, setLocalPass] = useState(preferences.validationColors.pass);

  useEffect(() => {
    if (open) {
      setLocalCritical(preferences.validationColors.critical);
      setLocalWarning(preferences.validationColors.warning);
      setLocalPass(preferences.validationColors.pass);
      if (typeof window !== 'undefined' && '__TAURI__' in window) {
        import('@tauri-apps/api/app').then(({ getVersion }) => getVersion()).then(setAppVersion).catch(() => {});
      }
    }
  }, [open, preferences.validationColors]);

  if (!open) return null;

  const handleApplyColors = () => {
    setValidationColors({
      critical: localCritical,
      warning: localWarning,
      pass: localPass,
    });
  };

  const handleResetColors = () => {
    setLocalCritical('#ef4444');
    setLocalWarning('#eab308');
    setLocalPass('#22c55e');
    setValidationColors({
      critical: '#ef4444',
      warning: '#eab308',
      pass: '#22c55e',
    });
  };

  return (
    <div className="settings-overlay" onClick={onClose} role="presentation">
      <div
        className="settings-panel"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <div className="settings-header">
          <h2 id="settings-title" className="settings-title">Settings</h2>
          <button type="button" className="settings-close" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        <div className="settings-content">
          {/* Theme */}
          <div className="settings-section">
            <div className="settings-section-title">Theme</div>
            <div className="settings-options">
              {(['dark', 'light'] as ThemeMode[]).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  className={`settings-option-btn ${preferences.theme === mode ? 'active' : ''}`}
                  onClick={() => setTheme(mode)}
                >
                  {mode === 'dark' ? 'Dark' : 'Light'}
                </button>
              ))}
            </div>
          </div>

          {/* Layout preset */}
          <div className="settings-section">
            <div className="settings-section-title">Layout preset</div>
            <div className="settings-options">
              {(['compact', 'normal', 'spacious'] as LayoutPreset[]).map((preset) => (
                <button
                  key={preset}
                  type="button"
                  className={`settings-option-btn ${preferences.layout === preset ? 'active' : ''}`}
                  onClick={() => setLayout(preset)}
                >
                  {preset === 'compact' ? 'Compact' : preset === 'normal' ? 'Normal' : 'Spacious'}
                </button>
              ))}
            </div>
            <div className="settings-hint">
              Compact: smaller panels. Normal: default. Spacious: larger panels for big screens.
            </div>
          </div>

          {/* Plugins directory */}
          <div className="settings-section">
            <div className="settings-section-title">Plugins directory</div>
            <div className="settings-plugins-row">
              <input
                type="text"
                value={preferences.pluginsDir}
                onChange={(e) => setPluginsDir(e.target.value)}
                placeholder="Default: ./.pbr-studio/plugins, ~/.config/pbr-studio/plugins"
                className="settings-plugins-input"
                aria-label="Plugin directory path"
              />
              {typeof window !== 'undefined' && '__TAURI__' in window && (
                <button
                  type="button"
                  className="settings-btn"
                  onClick={async () => {
                    try {
                      const { open } = await import('@tauri-apps/plugin-dialog');
                      const selected = await open({ directory: true, multiple: false });
                      if (selected && typeof selected === 'string') {
                        setPluginsDir(selected);
                      }
                    } catch {
                      // ignore
                    }
                  }}
                >
                  Browse
                </button>
              )}
            </div>
            <div className="settings-hint">
              Custom validation rules and export presets from JSON/TOML. Leave empty for defaults. Offline only.
            </div>
          </div>

          {/* Undo/redo history */}
          <div className="settings-section">
            <div className="settings-section-title">Undo history</div>
            <div className="settings-options">
              {[10, 25, 50, 100].map((n) => (
                <button
                  key={n}
                  type="button"
                  className={`settings-option-btn ${preferences.undoHistorySize === n ? 'active' : ''}`}
                  onClick={() => setUndoHistorySize(n)}
                >
                  {n} steps
                </button>
              ))}
            </div>
            <div className="settings-hint">
              Revert last N validation and material changes. Stored locally, no cloud.
            </div>
          </div>

          {/* Validation colors */}
          <div className="settings-section">
            <div className="settings-section-title">Validation panel colors</div>
            <div className="settings-color-row">
              <label className="settings-color-label">
                <span>Critical</span>
                <input
                  type="color"
                  value={localCritical}
                  onChange={(e) => setLocalCritical(e.target.value)}
                  className="settings-color-input"
                />
                <span className="settings-color-hex">{localCritical}</span>
              </label>
            </div>
            <div className="settings-color-row">
              <label className="settings-color-label">
                <span>Warning</span>
                <input
                  type="color"
                  value={localWarning}
                  onChange={(e) => setLocalWarning(e.target.value)}
                  className="settings-color-input"
                />
                <span className="settings-color-hex">{localWarning}</span>
              </label>
            </div>
            <div className="settings-color-row">
              <label className="settings-color-label">
                <span>Pass</span>
                <input
                  type="color"
                  value={localPass}
                  onChange={(e) => setLocalPass(e.target.value)}
                  className="settings-color-input"
                />
                <span className="settings-color-hex">{localPass}</span>
              </label>
            </div>
            <div className="settings-color-actions">
              <button type="button" className="settings-btn" onClick={handleApplyColors}>
                Apply colors
              </button>
              <button type="button" className="settings-btn" onClick={handleResetColors}>
                Reset to defaults
              </button>
            </div>
          </div>

          <div className="settings-about">
            <div className="settings-about-title">PBR Studio</div>
            <div className="settings-about-version">v{appVersion}</div>
            <div className="settings-storage-note">Offline PBR texture analyzer. All data stays local.</div>
          </div>

          <div className="settings-footer">
            <button type="button" className="settings-btn settings-btn-secondary" onClick={resetToDefaults}>
              Reset all preferences
            </button>
            <div className="settings-storage-note">All settings saved locally. No cloud required.</div>
          </div>
        </div>
      </div>
    </div>
  );
}
