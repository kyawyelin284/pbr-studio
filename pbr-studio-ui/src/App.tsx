import { useState, useCallback, useEffect, useRef } from 'react';
import { TexturePanel } from './components/TexturePanel';
import { Viewport3D, type TextureUrls } from './components/Viewport3D';
import { ValidationPanel } from './components/ValidationPanel';
import { ConsolePanel, type LogEntry, type LogLevel } from './components/ConsolePanel';
import { AuditLogPanel } from './components/AuditLogPanel';
import { AdvancedAnalysisPanel } from './components/AdvancedAnalysisPanel';
import { SettingsPanel } from './components/SettingsPanel';
import { usePreferences } from './context/PreferencesContext';
import { useUndoRedo } from './hooks/useUndoRedo';
import './App.css';

function useConsole() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const idRef = useRef(0);
  const log = useCallback((level: LogLevel, message: string) => {
    const id = ++idRef.current;
    setEntries((prev) => {
      const next = [...prev.slice(-99), { id, timestamp: new Date().toLocaleTimeString(), level, message }];
      return next;
    });
  }, []);
  return { entries, log };
}

interface ReportIssue {
  rule_id: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
}

interface OptimizationSuggestion {
  category: string;
  message: string;
  priority?: number;
  details?: string;
}

interface MaterialReport {
  issues: ReportIssue[];
  optimization_suggestions: OptimizationSuggestion[];
  passed: boolean;
  error_count: number;
  warning_count: number;
  score?: number;
  vram_estimate?: { bytes: number; formatted: string; packed_orm: boolean; include_mipmaps?: boolean };
}

interface MaterialState {
  path: string;
  name: string;
  report: MaterialReport | null;
  textureUrls: TextureUrls;
  loading: boolean;
}

const emptyMaterialState: MaterialState[] = [];

function App() {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [selectedIndices, setSelectedIndices] = useState<number[]>([]);
  const [compareIndex, setCompareIndex] = useState<number | null>(null);
  const [exportLoading, setExportLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hdriPreset, setHdriPreset] = useState('studio');
  const [dragOver, setDragOver] = useState(false);
  const [bottomTab, setBottomTab] = useState<'console' | 'audit' | 'advanced'>('console');
  const [tileabilityPreview, setTileabilityPreview] = useState<{
    originalUrls: TextureUrls;
    fixedTextureUrls: TextureUrls;
  } | null>(null);
  const [auditRefreshTrigger, setAuditRefreshTrigger] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [textureRefreshKey, setTextureRefreshKey] = useState(0);
  const lastMtimeMapRef = useRef<Map<string, number>>(new Map());
  const { entries: consoleEntries, log } = useConsole();
  const { preferences } = usePreferences();

  const {
    state: materials,
    set: setMaterials,
    setWithoutHistory,
    undo,
    redo,
    canUndo,
    canRedo,
    undoCount,
    redoCount,
  } = useUndoRedo<MaterialState[]>(emptyMaterialState, preferences.undoHistorySize);

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  const loadTextureUrls = useCallback(async (path: string, refreshKey?: number): Promise<TextureUrls> => {
    if (!isTauri) return {};
    try {
      const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
      const json = await invoke<string>('get_texture_paths', { path });
      const paths = JSON.parse(json) as Record<string, string>;
      const bust = refreshKey != null ? `?t=${refreshKey}` : '';
      const urls: TextureUrls = {};
      if (paths.albedo) urls.albedo = convertFileSrc(paths.albedo) + bust;
      if (paths.normal) urls.normal = convertFileSrc(paths.normal) + bust;
      if (paths.roughness) urls.roughness = convertFileSrc(paths.roughness) + bust;
      if (paths.metallic) urls.metallic = convertFileSrc(paths.metallic) + bust;
      if (paths.ao) urls.ao = convertFileSrc(paths.ao) + bust;
      return urls;
    } catch {
      return {};
    }
  }, [isTauri]);

  const analyzeAndAddMaterials = useCallback(
    async (paths: string[]) => {
      if (!paths.length || !isTauri) return;

      setError(null);
      log('info', `Expanding ${paths.length} path(s)...`);

      const { invoke } = await import('@tauri-apps/api/core');
      let resolved: string[] = [];
      try {
        resolved = await invoke<string[]>('expand_material_paths', { paths });
      } catch {
        // Fallback: treat each path as a single material folder
        for (const p of paths) {
          try {
            const folder = await invoke<string>('resolve_material_folder', { path: p });
            resolved.push(folder);
          } catch {
            log('warn', `Skipped invalid path: ${p}`);
          }
        }
      }

      if (resolved.length === 0) {
        setError('No valid material folders found.');
        return;
      }

      log('info', `Analyzing ${resolved.length} material folder(s)...`);

      let insertStart = 0;
      let pathsToAnalyze: string[] = [];
      setMaterials((prev) => {
        const existingPaths = new Set(prev.map((m) => m.path));
        pathsToAnalyze = resolved.filter((p) => !existingPaths.has(p));
        if (pathsToAnalyze.length === 0) {
          log('info', 'All folders already in list.');
          return prev;
        }
        insertStart = prev.length;
        const newMaterials: MaterialState[] = pathsToAnalyze.map((path) => ({
          path,
          name: path.split(/[/\\]/).filter(Boolean).pop() || path,
          report: null,
          textureUrls: {},
          loading: true,
        }));
        return [...prev, ...newMaterials];
      });

      if (pathsToAnalyze.length === 0) return;

      try {
        const reportsJson = await invoke<string[]>('analyze_folders', {
          paths: pathsToAnalyze,
          pluginsDir: preferences.pluginsDir || undefined,
        });
        const texturePromises = pathsToAnalyze.map((p) => loadTextureUrls(p));

        const reports: MaterialReport[] = reportsJson.map((json) => {
          try {
            const p = JSON.parse(json) as MaterialReport & { error?: string };
            if (p.error) return { issues: [], optimization_suggestions: [], passed: false, error_count: 1, warning_count: 0 };
            return p;
          } catch {
            return { issues: [], optimization_suggestions: [], passed: false, error_count: 1, warning_count: 0 };
          }
        });

        const urlsList = await Promise.all(texturePromises);

        setMaterials((prev) => {
          const next = [...prev];
          for (let i = 0; i < pathsToAnalyze.length; i++) {
            const idx = insertStart + i;
            if (idx < next.length) {
              next[idx] = {
                ...next[idx],
                report: reports[i] ?? null,
                textureUrls: urlsList[i] ?? {},
                loading: false,
              };
            }
          }
          return next;
        });

        const scores = reports.map((r) => r.score ?? null);
        log('success', `Analyzed ${pathsToAnalyze.length} material(s). Scores: ${scores.join(', ')}`);
        if (insertStart === 0) setSelectedIndex(0);
        setAuditRefreshTrigger((t) => t + 1);
      } catch (e) {
        const msg = String(e);
        setError(msg);
        log('error', `Batch analysis failed: ${msg}`);
        setMaterials((prev) => prev.filter((m) => !m.loading));
      }
    },
    [isTauri, loadTextureUrls, log, setMaterials, preferences.pluginsDir]
  );

  const handleAnalyze = useCallback(
    async (pathOverride?: string) => {
      const path = pathOverride ?? (selectedIndex != null ? materials[selectedIndex]?.path : null);
      const pathsToUse = path ? [path] : (materials.length > 0 ? materials.map((m) => m.path) : []);
      if (!pathsToUse.length) {
        setError('Please add material folders first (use Add folders or drag-drop).');
        log('warn', 'Please add material folders first.');
        return;
      }
      await analyzeAndAddMaterials(pathsToUse);
    },
    [materials, selectedIndex, analyzeAndAddMaterials, log]
  );

  const handleSelectMaterial = useCallback((index: number, evt?: React.MouseEvent) => {
    const meta = evt?.ctrlKey || evt?.metaKey;
    const shift = evt?.shiftKey;

    if (meta) {
      setSelectedIndices((prev) => {
        const next = prev.includes(index) ? prev.filter((i) => i !== index) : [...prev, index].sort((a, b) => a - b);
        setSelectedIndex(next.length === 1 ? next[0] : next.includes(index) ? index : prev[0] ?? null);
        return next;
      });
    } else if (shift) {
      setSelectedIndex((anchor) => {
        const a = anchor ?? index;
        const from = Math.min(a, index);
        const to = Math.max(a, index);
        const next = Array.from({ length: to - from + 1 }, (_, i) => from + i);
        setSelectedIndices(next);
        return index;
      });
    } else {
      setSelectedIndices([index]);
      setSelectedIndex(index);
    }
  }, []);

  const handleSelectAll = useCallback(() => {
    const all = materials.map((_, i) => i);
    setSelectedIndices(all);
    setSelectedIndex(all.length > 0 ? 0 : null);
  }, [materials.length]);

  const handleClearSelection = useCallback(() => {
    setSelectedIndices([]);
    setSelectedIndex(null);
  }, []);

  const handleBatchAnalyze = useCallback(() => {
    const paths = selectedIndices.length > 0
      ? selectedIndices.map((i) => materials[i]?.path).filter(Boolean) as string[]
      : materials.map((m) => m.path);
    if (paths.length > 0) {
      analyzeAndAddMaterials(paths);
    }
  }, [selectedIndices, materials, analyzeAndAddMaterials]);

  const handleRemoveMaterial = useCallback(
    (index: number) => {
      const next = materials.filter((_, i) => i !== index);
      setMaterials(next);
      setSelectedIndices((prev) =>
        prev
          .filter((i) => i !== index)
          .map((i) => (i > index ? i - 1 : i))
      );
      if (selectedIndex === index) {
        setSelectedIndex(next.length > 0 ? Math.min(index, next.length - 1) : null);
      } else if (selectedIndex != null && selectedIndex > index) {
        setSelectedIndex(selectedIndex - 1);
      }
      if (compareIndex === index) {
        setCompareIndex(null);
      } else if (compareIndex != null && compareIndex > index) {
        setCompareIndex(compareIndex - 1);
      }
    },
    [materials, setMaterials, selectedIndex, compareIndex]
  );

  const currentReport = selectedIndex != null ? materials[selectedIndex]?.report ?? null : null;
  const currentTextureUrls = selectedIndex != null ? materials[selectedIndex]?.textureUrls ?? {} : {};
  const compareTextureUrls = compareIndex != null ? materials[compareIndex]?.textureUrls ?? {} : {};
  const folderPath = selectedIndex != null ? materials[selectedIndex]?.path ?? '' : '';
  const exportPaths = selectedIndices.length > 0
    ? selectedIndices.map((i) => materials[i]?.path).filter(Boolean) as string[]
    : materials.map((m) => m.path);

  const handleFolderSelect = useCallback(
    (paths: string[]) => {
      const valid = paths.filter(Boolean);
      if (!valid.length) return;
      analyzeAndAddMaterials(valid);
    },
    [analyzeAndAddMaterials]
  );

  const handleExportReport = useCallback(
    async (paths: string[], format: 'html' | 'pdf') => {
      if (!paths.length || !isTauri) return;
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const ext = format === 'html' ? '.html' : '.pdf';
        const defaultName = paths.length === 1
          ? `pbr-report-${paths[0].split(/[/\\]/).filter(Boolean).pop() || 'material'}${ext}`
          : `pbr-batch-report${ext}`;
        const outputPath = await save({
          defaultPath: defaultName,
          filters: [{ name: format.toUpperCase(), extensions: [format] }],
        });
        if (!outputPath) return;

        setExportLoading(true);
        setError(null);
        log('info', `Exporting report (${format}) to ${outputPath}`);
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('export_report', {
          paths,
          format,
          output_path: outputPath,
          track: true,
        });
        log('success', `Report exported to ${outputPath}`);
        setAuditRefreshTrigger((t) => t + 1);
        const { message } = await import('@tauri-apps/plugin-dialog');
        await message(`Report exported to:\n${outputPath}`, { title: 'Export complete', kind: 'info' });
      } catch (e) {
        const msg = String(e);
        setError(msg);
        log('error', `Export failed: ${msg}`);
      } finally {
        setExportLoading(false);
      }
    },
    [isTauri, log]
  );

  const handleBatchExport = useCallback(() => {
    const paths = selectedIndices.length > 0
      ? selectedIndices.map((i) => materials[i]?.path).filter(Boolean) as string[]
      : materials.map((m) => m.path);
    if (paths.length > 0) {
      handleExportReport(paths, 'html');
    }
  }, [selectedIndices, materials, handleExportReport]);

  const handleBatchExportPreset = useCallback(
    async (preset: string, includeLod?: boolean) => {
      if (!isTauri || materials.length === 0) return;
      const paths = selectedIndices.length > 0
        ? selectedIndices.map((i) => materials[i]?.path).filter(Boolean) as string[]
        : materials.map((m) => m.path);
      if (paths.length === 0) return;
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const outputRoot = await open({
          directory: true,
          multiple: false,
          title: `Batch export (${paths.length} materials) – choose output folder`,
        });
        if (!outputRoot || typeof outputRoot !== 'string') return;

        setExportLoading(true);
        setError(null);
        const presetLabel = preset === 'unreal' ? 'Unreal Engine' : preset === 'unity' ? 'Unity' : preset === 'mobile' ? 'Mobile' : preset;
        log('info', `Batch exporting ${paths.length} material(s) as ${presetLabel} to ${outputRoot}`);
        const { invoke } = await import('@tauri-apps/api/core');
        const written = await invoke<string[]>('batch_export_preset', {
          sourcePaths: paths,
          outputRoot,
          preset,
          includeLod: includeLod ?? false,
          pluginsDir: preferences.pluginsDir || undefined,
        });
        setError(null);
        if (written?.length) {
          log('success', `Exported ${written.length} texture(s) to ${outputRoot}`);
          setAuditRefreshTrigger((t) => t + 1);
          const { message } = await import('@tauri-apps/plugin-dialog');
          await message(
            `Exported ${written.length} texture(s) to:\n${outputRoot}`,
            { title: 'Batch export complete', kind: 'info' }
          );
        }
      } catch (e) {
        const msg = String(e);
        setError(msg);
        log('error', `Batch export failed: ${msg}`);
      } finally {
        setExportLoading(false);
      }
    },
    [isTauri, materials, selectedIndices, log, preferences.pluginsDir]
  );

  const handleTileabilityPreview = useCallback(
    async (originalUrls: TextureUrls, fixedAlbedoPath: string) => {
      try {
        const { convertFileSrc } = await import('@tauri-apps/api/core');
        const url = convertFileSrc(fixedAlbedoPath) + `?t=${Date.now()}`;
        setTileabilityPreview({
          originalUrls,
          fixedTextureUrls: { ...originalUrls, albedo: url },
        });
      } catch {
        setTileabilityPreview(null);
      }
    },
    []
  );

  const handleClearTileabilityPreview = useCallback(() => {
    setTileabilityPreview(null);
  }, []);

  const handleRefreshSelected = useCallback(async () => {
    const path = selectedIndex != null ? materials[selectedIndex]?.path : null;
    if (!path || !isTauri) return;
    const key = Date.now();
    setTextureRefreshKey(key);
    lastMtimeMapRef.current.delete(path);
    try {
      setError(null);
      setMaterials((prev) => {
        const idx = prev.findIndex((m) => m.path === path);
        if (idx < 0) return prev;
        const next = [...prev];
        next[idx] = { ...next[idx], loading: true };
        return next;
      });
      const { invoke } = await import('@tauri-apps/api/core');
      const [reportJson] = await invoke<string[]>('analyze_folders', {
        paths: [path],
        pluginsDir: preferences.pluginsDir || undefined,
      });
      const report = (() => {
        try {
          const p = JSON.parse(reportJson) as MaterialReport & { error?: string };
          if (p.error) return { issues: [], optimization_suggestions: [], passed: false, error_count: 1, warning_count: 0 };
          return p;
        } catch {
          return null;
        }
      })();
      const urls = await loadTextureUrls(path, key);
      setMaterials((prev) => {
        const idx = prev.findIndex((m) => m.path === path);
        if (idx < 0) return prev;
        const next = [...prev];
        next[idx] = { ...next[idx], report, textureUrls: urls, loading: false };
        return next;
      });
      log('success', `Refreshed: score ${report?.score ?? '—'}`);
    } catch (e) {
      setError(String(e));
      setMaterials((prev) => prev.map((m) => (m.path === path ? { ...m, loading: false } : m)));
    }
  }, [selectedIndex, materials, isTauri, loadTextureUrls, setMaterials, log, preferences.pluginsDir]);

  const handleExportPreset = useCallback(
    async (preset: string, includeLod?: boolean) => {
      if (!folderPath || !isTauri) return;
      const presetLabel =
        preset === 'unreal' ? 'Unreal Engine' :
        preset === 'unity' ? 'Unity' :
        preset === 'mobile' ? 'Mobile Optimized' :
        preset === '4k' ? '4K' : preset;
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const outputPath = await open({
          directory: true,
          multiple: false,
          title: `Export as ${presetLabel}`,
        });
        if (!outputPath || typeof outputPath !== 'string') return;

        setExportLoading(true);
        setError(null);
        log('info', `Exporting as ${presetLabel} to ${outputPath}`);
        const { invoke } = await import('@tauri-apps/api/core');
        const written = await invoke<string[]>('export_preset', {
          sourcePath: folderPath,
          outputPath,
          preset,
          includeLod: includeLod ?? false,
          pluginsDir: preferences.pluginsDir || undefined,
        });
        setError(null);
        if (written?.length) {
          log('success', `Exported ${written.length} texture(s) to ${outputPath}`);
          setAuditRefreshTrigger((t) => t + 1);
          const { message } = await import('@tauri-apps/plugin-dialog');
          await message(
            `Exported ${written.length} texture(s) to:\n${outputPath}`,
            { title: 'Export complete', kind: 'info' }
          );
        }
      } catch (e) {
        const msg = String(e);
        setError(msg);
        log('error', `Export failed: ${msg}`);
      } finally {
        setExportLoading(false);
      }
    },
    [folderPath, isTauri, log, preferences.pluginsDir]
  );

  useEffect(() => {
    if (!isTauri) return;
    let unlisten: (() => void) | undefined;
    const setup = async () => {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      unlisten = await getCurrentWebviewWindow().onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === 'enter' || payload.type === 'over') {
          setDragOver(true);
        } else if (payload.type === 'leave') {
          setDragOver(false);
        } else if (payload.type === 'drop' && 'paths' in payload && payload.paths?.length) {
          setDragOver(false);
          const paths = payload.paths as string[];
          analyzeAndAddMaterials(paths);
        }
      });
    };
    setup();
    return () => unlisten?.();
  }, [isTauri, analyzeAndAddMaterials]);

  // Live scoring: poll selected (and compare) material folders for file changes; re-analyze and refresh when mtime changes
  const watchedPath = selectedIndex != null ? materials[selectedIndex]?.path : null;
  const comparePath = compareIndex != null ? materials[compareIndex]?.path : null;
  const watchedPaths = [watchedPath, comparePath].filter(Boolean) as string[];

  useEffect(() => {
    if (!isTauri || watchedPaths.length === 0) return;
    const interval = setInterval(async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        for (const path of watchedPaths) {
          const mtime = await invoke<number | null>('get_material_folder_mtime', { path });
          if (mtime == null) continue;
          const prev = lastMtimeMapRef.current.get(path);
          lastMtimeMapRef.current.set(path, mtime);
          if (prev != null && mtime !== prev) {
            log('info', 'Material folder changed, updating score and preview...');
            const key = Date.now();
            setTextureRefreshKey(key);
            const [reportJson] = await invoke<string[]>('analyze_folders', {
              paths: [path],
              pluginsDir: preferences.pluginsDir || undefined,
            });
            const report = (() => {
              try {
                const p = JSON.parse(reportJson) as MaterialReport & { error?: string };
                if (p.error) return { issues: [], optimization_suggestions: [], passed: false, error_count: 1, warning_count: 0 };
                return p;
              } catch {
                return null;
              }
            })();
            const urls = await loadTextureUrls(path, key);
            setWithoutHistory((p) => {
              const idx = p.findIndex((m) => m.path === path);
              if (idx < 0) return p;
              const next = [...p];
              next[idx] = { ...next[idx], report, textureUrls: urls };
              return next;
            });
          } else if (prev == null) {
            lastMtimeMapRef.current.set(path, mtime);
          }
        }
      } catch {
        // Ignore polling errors
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [isTauri, watchedPaths.join('|'), loadTextureUrls, setWithoutHistory, log, preferences.pluginsDir]);

  // When running in browser without Tauri, show desktop-only message and hide filesystem-dependent UI
  if (!isTauri) {
    return (
      <div className="app desktop-only-overlay">
        <div className="desktop-only-content">
          <div className="desktop-only-icon" aria-hidden="true">🖥</div>
          <h1 className="desktop-only-title">PBR Studio</h1>
          <p className="desktop-only-message">
            PBR Studio is a desktop application. Please download the desktop app for Linux, Windows, or macOS.
          </p>
          <p className="desktop-only-hint">
            Use <code>npm run tauri:dev</code> or build the desktop app to access material analysis, validation, and export.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <button
        type="button"
        className="settings-trigger"
        onClick={() => setSettingsOpen(true)}
        aria-label="Open settings"
        title="Settings (theme, colors, layout)"
      >
        ⚙
      </button>
      <SettingsPanel open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      {dragOver && (
        <div className="drop-overlay" aria-hidden="true">
          <div className="drop-overlay-content">
            <span className="drop-overlay-icon">📁</span>
            <span>Drop material folder(s) to analyze</span>
          </div>
        </div>
      )}
      <div className="app-main">
        <TexturePanel
          folderPath={folderPath}
          onFolderSelect={handleFolderSelect}
          onAnalyze={handleAnalyze}
          onRefresh={handleRefreshSelected}
          dragOver={dragOver}
          onExportPreset={handleExportPreset}
          onBatchExportPreset={handleBatchExportPreset}
          exportLoading={exportLoading}
          materials={materials.map((m) => ({ path: m.path, name: m.name, score: m.report?.score ?? null, loading: m.loading }))}
          selectedIndex={selectedIndex}
          selectedIndices={selectedIndices}
          compareIndex={compareIndex}
          onSelectMaterial={handleSelectMaterial}
          onSelectAll={handleSelectAll}
          onClearSelection={handleClearSelection}
          onCompareMaterial={setCompareIndex}
          onRemoveMaterial={handleRemoveMaterial}
          onBatchAnalyze={handleBatchAnalyze}
          onBatchExport={handleBatchExport}
          canUndo={canUndo}
          canRedo={canRedo}
          undoCount={undoCount}
          redoCount={redoCount}
          onUndo={undo}
          onRedo={redo}
        />
        <Viewport3D
          textureUrls={tileabilityPreview ? tileabilityPreview.originalUrls : currentTextureUrls}
          textureUrlsB={
            tileabilityPreview
              ? tileabilityPreview.fixedTextureUrls
              : compareIndex != null
                ? compareTextureUrls
                : undefined
          }
          hdriPreset={hdriPreset}
          onPresetChange={setHdriPreset}
          compareMode={tileabilityPreview != null || compareIndex != null}
          labelA={tileabilityPreview ? 'Original' : selectedIndex != null ? materials[selectedIndex]?.name : 'A'}
          labelB={tileabilityPreview ? 'Fixed' : compareIndex != null ? materials[compareIndex]?.name : 'B'}
          refreshKey={textureRefreshKey}
          tileabilityPreviewActive={tileabilityPreview != null}
          onClearTileabilityPreview={handleClearTileabilityPreview}
        />
        <ValidationPanel
          report={currentReport}
          compareReport={compareIndex != null ? materials[compareIndex]?.report ?? null : null}
          compareMode={compareIndex != null}
          compareName={compareIndex != null ? materials[compareIndex]?.name : undefined}
          loading={selectedIndex != null && materials[selectedIndex]?.loading}
          error={error ?? undefined}
          materialPath={folderPath || undefined}
          allPaths={exportPaths}
          isTauri={isTauri}
          onExportReport={handleExportReport}
        />
      </div>
      <div className="bottom-panel-area">
        <div className="bottom-tabs">
          <button
            type="button"
            className={`bottom-tab ${bottomTab === 'console' ? 'active' : ''}`}
            onClick={() => setBottomTab('console')}
          >
            Console
          </button>
          <button
            type="button"
            className={`bottom-tab ${bottomTab === 'audit' ? 'active' : ''}`}
            onClick={() => setBottomTab('audit')}
          >
            Audit Log
          </button>
          <button
            type="button"
            className={`bottom-tab ${bottomTab === 'advanced' ? 'active' : ''}`}
            onClick={() => setBottomTab('advanced')}
          >
            Advanced Analysis
          </button>
        </div>
        {bottomTab === 'console' ? (
          <ConsolePanel entries={consoleEntries} />
        ) : bottomTab === 'audit' ? (
          <AuditLogPanel isTauri={isTauri} refreshTrigger={auditRefreshTrigger} />
        ) : (
          <AdvancedAnalysisPanel
            materialPaths={exportPaths}
            currentTextureUrls={currentTextureUrls}
            isTauri={isTauri}
            onLog={log}
            onTileabilityPreview={handleTileabilityPreview}
            onClearTileabilityPreview={handleClearTileabilityPreview}
          />
        )}
      </div>
    </div>
  );
}

export default App;
