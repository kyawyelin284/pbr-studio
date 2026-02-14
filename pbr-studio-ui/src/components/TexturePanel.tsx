import { useState, useEffect } from 'react';
import { MaterialList } from './MaterialList';
import { usePreferences } from '../context/PreferencesContext';

const TEXTURE_SLOTS = [
  { id: 'albedo', label: 'Albedo / Base Color' },
  { id: 'normal', label: 'Normal' },
  { id: 'roughness', label: 'Roughness' },
  { id: 'metallic', label: 'Metallic' },
  { id: 'ao', label: 'Ambient Occlusion' },
  { id: 'height', label: 'Height' },
] as const;

export type ExportPresetId = '4k' | 'unreal' | 'unity' | 'mobile' | string;

interface CustomPreset {
  id: string;
  name: string;
  target_resolution: string;
  include_lod?: boolean;
}

interface TexturePanelProps {
  folderPath?: string;
  onFolderSelect?: (paths: string[]) => void;
  onAnalyze?: () => void;
  onRefresh?: () => void;
  dragOver?: boolean;
  onExportPreset?: (preset: ExportPresetId, includeLod?: boolean) => void;
  exportLoading?: boolean;
  materials?: { path: string; name: string; score: number | null; loading?: boolean }[];
  selectedIndex?: number | null;
  selectedIndices?: number[];
  compareIndex?: number | null;
  onSelectMaterial?: (index: number, evt?: React.MouseEvent) => void;
  onSelectAll?: () => void;
  onClearSelection?: () => void;
  onCompareMaterial?: (index: number | null) => void;
  onRemoveMaterial?: (index: number) => void;
  onBatchAnalyze?: () => void;
  onBatchExport?: () => void;
  canUndo?: boolean;
  canRedo?: boolean;
  undoCount?: number;
  redoCount?: number;
  onUndo?: () => void;
  onRedo?: () => void;
}

const BUILTIN_PRESETS: { id: ExportPresetId; label: string }[] = [
  { id: '4k', label: '4K' },
  { id: 'unreal', label: 'Unreal Engine' },
  { id: 'unity', label: 'Unity' },
  { id: 'mobile', label: 'Mobile Optimized' },
];

export function TexturePanel({
  folderPath = '',
  onFolderSelect,
  onAnalyze,
  onRefresh,
  onExportPreset,
  exportLoading = false,
  materials = [],
  selectedIndex = null,
  selectedIndices = [],
  compareIndex = null,
  onSelectMaterial,
  onSelectAll,
  onClearSelection,
  onCompareMaterial,
  onRemoveMaterial,
  onBatchAnalyze,
  onBatchExport,
  canUndo = false,
  canRedo = false,
  undoCount = 0,
  redoCount = 0,
  onUndo,
  onRedo,
  dragOver = false,
}: TexturePanelProps) {
  const [isTauri] = useState(() => typeof window !== 'undefined' && '__TAURI__' in window);
  const [includeLod, setIncludeLod] = useState(false);
  const [customPresets, setCustomPresets] = useState<CustomPreset[]>([]);

  const pluginsDir = usePreferences().preferences.pluginsDir;
  useEffect(() => {
    if (!isTauri) return;
    const load = async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const json = await invoke<string>('get_plugin_presets', {
          pluginsDir: pluginsDir || undefined,
        });
        const presets = JSON.parse(json) as CustomPreset[];
        setCustomPresets(presets ?? []);
      } catch {
        setCustomPresets([]);
      }
    };
    load();
  }, [isTauri, pluginsDir]);

  const exportPresets = [
    ...BUILTIN_PRESETS,
    ...customPresets.map((p) => ({ id: p.id as ExportPresetId, label: p.name })),
  ];

  const handleOpenFolder = async () => {
    if (isTauri) {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          directory: true,
          multiple: true,
        });
        if (selected) {
          const paths = Array.isArray(selected) ? selected : [selected];
          if (paths.length > 0) {
            onFolderSelect?.(paths);
          }
        }
      } catch (e) {
        console.error('Failed to open folder:', e);
      }
    } else {
      const path = prompt('Enter folder path (Tauri required for file dialog):');
      if (path) {
        onFolderSelect?.([path]);
      }
    }
  };

  return (
    <div className={`panel panel-left ${dragOver ? 'panel-drop-target' : ''}`}>
      <div className="panel-header">Texture Maps</div>
      <div className="panel-content">
        {onSelectMaterial && onCompareMaterial && onRemoveMaterial && onUndo && onRedo && (
          <MaterialList
            materials={materials}
            selectedIndex={selectedIndex}
            selectedIndices={selectedIndices}
            compareIndex={compareIndex ?? null}
            onSelect={onSelectMaterial}
            onSelectAll={onSelectAll ?? (() => {})}
            onClearSelection={onClearSelection ?? (() => {})}
            onCompare={onCompareMaterial}
            onRemove={onRemoveMaterial}
            onBatchAnalyze={onBatchAnalyze}
            onBatchExport={onBatchExport}
            canUndo={canUndo}
            canRedo={canRedo}
            undoCount={undoCount}
            redoCount={redoCount}
            onUndo={onUndo}
            onRedo={onRedo}
          />
        )}
        <div className="texture-slot">
          <span className="texture-slot-label">Material Folder</span>
          <div className="texture-slot-input">
            <input
              type="text"
              value={folderPath}
              onChange={(e) => onFolderSelect?.([e.target.value])}
              placeholder="Select folder(s)..."
            />
            <button onClick={handleOpenFolder}>Browse</button>
          </div>
        </div>

        <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
          <button
            onClick={onAnalyze}
            disabled={!folderPath}
            style={{ flex: 1 }}
          >
            {materials.length > 0 ? 'Re-analyze Selected' : 'Analyze Material'}
          </button>
          {onRefresh && folderPath && (
            <button
              onClick={onRefresh}
              disabled={exportLoading}
              title="Refresh score and preview (detects file changes)"
              style={{ padding: '8px 12px' }}
            >
              ↻
            </button>
          )}
        </div>

        {isTauri && onExportPreset && (
          <div className="export-presets" style={{ marginTop: 24 }}>
            <span className="texture-slot-label" style={{ marginBottom: 10, display: 'block' }}>
              Export Presets
            </span>
            <label style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10, fontSize: '0.875rem' }}>
              <input
                type="checkbox"
                checked={includeLod}
                onChange={(e) => setIncludeLod(e.target.checked)}
              />
              Include LOD chain (512, 256, 128)
            </label>
            <div className="export-preset-buttons">
              {exportPresets.map(({ id, label }) => (
                <button
                  key={id}
                  onClick={() => onExportPreset(id, includeLod)}
                  disabled={!folderPath || exportLoading}
                  className="export-preset-btn"
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
        )}

        <div style={{ marginTop: 24 }}>
          {isTauri && (
          <p className="texture-slot-hint" style={{ marginTop: 8, marginBottom: 16 }}>
            Tip: Drag multiple folders or a root folder (recursive scan). Use Browse to select several folders at once.
          </p>
        )}
        <span className="texture-slot-label" style={{ marginBottom: 12, display: 'block' }}>
            Texture Slots
          </span>
          {TEXTURE_SLOTS.map((slot) => (
            <div key={slot.id} className="texture-slot">
              <span className="texture-slot-label">{slot.label}</span>
              <div className="texture-slot-input">
                <input type="text" placeholder={`${slot.id}.png`} />
                <button>...</button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
