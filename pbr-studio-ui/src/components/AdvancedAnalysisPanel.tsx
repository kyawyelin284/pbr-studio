import { useState, useCallback } from 'react';
import type { TextureUrls } from './Viewport3D';

interface DuplicatePair {
  path_a: string;
  path_b: string;
  slot: string;
  material_a?: string;
  material_b?: string;
  similarity: number;
}

interface DuplicateAnalysisResult {
  duplicate_pairs: DuplicatePair[];
  similar_pairs: DuplicatePair[];
  duplicate_threshold: number;
  similar_threshold: number;
}

interface ResolutionDistribution {
  width: number;
  height: number;
  count: number;
  materials: string[];
}

interface MapCoverage {
  slot: string;
  present_count: number;
  total_count: number;
  coverage_percent: number;
  missing_in: string[];
}

interface CrossMaterialResult {
  material_count: number;
  resolution_distributions: ResolutionDistribution[];
  resolution_inconsistent: boolean;
  map_coverage: MapCoverage[];
  recommendations: string[];
}

interface TileabilityAnalysisEntry {
  path: string;
  slot: string;
  material_name?: string;
  edge_difference: number;
  needs_fix: boolean;
}

interface AdvancedAnalysisReport {
  duplicates: DuplicateAnalysisResult;
  cross_material: CrossMaterialResult;
  tileability_analysis: TileabilityAnalysisEntry[];
}

interface AdvancedAnalysisPanelProps {
  materialPaths: string[];
  currentTextureUrls: TextureUrls;
  isTauri?: boolean;
  onLog?: (level: 'info' | 'warn' | 'error' | 'success', message: string) => void;
  onTileabilityPreview?: (originalUrls: TextureUrls, fixedAlbedoPath: string) => void;
  onClearTileabilityPreview?: () => void;
}

export function AdvancedAnalysisPanel({
  materialPaths,
  currentTextureUrls,
  isTauri,
  onLog,
  onTileabilityPreview,
  onClearTileabilityPreview,
}: AdvancedAnalysisPanelProps) {
  const [report, setReport] = useState<AdvancedAnalysisReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [fixingPath, setFixingPath] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<'duplicates' | 'cross' | 'tileability'>('duplicates');

  const runAnalysis = useCallback(async () => {
    if (!isTauri || materialPaths.length === 0) return;
    setLoading(true);
    setReport(null);
    onLog?.('info', `Running advanced analysis on ${materialPaths.length} material(s)...`);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const json = await invoke<string>('run_advanced_analysis_cmd', {
        paths: materialPaths,
        duplicateThreshold: 0.99,
        similarThreshold: 0.8,
      });
      const parsed = JSON.parse(json) as AdvancedAnalysisReport;
      setReport(parsed);
      const dupCount = parsed.duplicates.duplicate_pairs.length + parsed.duplicates.similar_pairs.length;
      const tileCount = parsed.tileability_analysis.filter((e) => e.needs_fix).length;
      onLog?.(
        'success',
        `Analysis complete. Duplicates/similar: ${dupCount}, Tileability issues: ${tileCount}`
      );
    } catch (e) {
      const msg = String(e);
      onLog?.('error', `Analysis failed: ${msg}`);
    } finally {
      setLoading(false);
    }
  }, [isTauri, materialPaths, onLog]);

  const handleFixTileability = useCallback(
    async (entry: TileabilityAnalysisEntry) => {
      if (!isTauri) return;
      setFixingPath(entry.path);
      onClearTileabilityPreview?.();
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const { save } = await import('@tauri-apps/plugin-dialog');
        const baseName = entry.path.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, '') || 'albedo';
        const outputPath = await save({
          defaultPath: `${baseName}-fixed.png`,
          filters: [{ name: 'PNG', extensions: ['png'] }],
        });
        if (!outputPath) {
          setFixingPath(null);
          return;
        }
        const result = await invoke<{
          output_path: string;
          original_edge_difference: number;
          fixed_edge_difference: number;
          improved: boolean;
        }>('fix_tileability_texture', {
          path: entry.path,
          outputPath,
          blendWidth: 4,
        });
        onLog?.(
          'success',
          `Fixed: edge diff ${result.original_edge_difference.toFixed(1)} → ${result.fixed_edge_difference.toFixed(1)} (improved: ${result.improved})`
        );
        onTileabilityPreview?.(currentTextureUrls, result.output_path);
      } catch (e) {
        const msg = String(e);
        onLog?.('error', `Fix tileability failed: ${msg}`);
      } finally {
        setFixingPath(null);
      }
    },
    [isTauri, currentTextureUrls, onLog, onTileabilityPreview, onClearTileabilityPreview]
  );

  if (!isTauri) return null;

  return (
    <div className="panel panel-console panel-advanced">
      <div className="panel-header">Advanced Analysis</div>
      <div className="advanced-analysis-actions">
        <button
          type="button"
          className="btn-primary"
          onClick={runAnalysis}
          disabled={loading || materialPaths.length === 0}
        >
          {loading ? 'Analyzing…' : 'Run Analysis'}
        </button>
        {materialPaths.length === 0 && (
          <span className="advanced-hint">Add material folders first.</span>
        )}
      </div>

      {report && (
        <>
          <div className="advanced-section-tabs">
            <button
              type="button"
              className={activeSection === 'duplicates' ? 'active' : ''}
              onClick={() => setActiveSection('duplicates')}
            >
              Duplicates ({report.duplicates.duplicate_pairs.length + report.duplicates.similar_pairs.length})
            </button>
            <button
              type="button"
              className={activeSection === 'cross' ? 'active' : ''}
              onClick={() => setActiveSection('cross')}
            >
              Cross-material
            </button>
            <button
              type="button"
              className={activeSection === 'tileability' ? 'active' : ''}
              onClick={() => setActiveSection('tileability')}
            >
              Tileability ({report.tileability_analysis.filter((e) => e.needs_fix).length})
            </button>
          </div>

          <div className="advanced-section-content">
            {activeSection === 'duplicates' && (
              <div className="advanced-duplicates">
                {report.duplicates.duplicate_pairs.length > 0 && (
                  <div className="advanced-subsection">
                    <h4>Exact duplicates</h4>
                    <ul>
                      {report.duplicates.duplicate_pairs.map((p, i) => (
                        <li key={i}>
                          <span className="slot-badge">{p.slot}</span>
                          <span className="similarity">{(p.similarity * 100).toFixed(0)}%</span>
                          <span className="paths">
                            {p.path_a.split(/[/\\]/).pop()} ↔ {p.path_b.split(/[/\\]/).pop()}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
                {report.duplicates.similar_pairs.length > 0 && (
                  <div className="advanced-subsection">
                    <h4>Similar textures</h4>
                    <ul>
                      {report.duplicates.similar_pairs.map((p, i) => (
                        <li key={i}>
                          <span className="slot-badge">{p.slot}</span>
                          <span className="similarity">{(p.similarity * 100).toFixed(0)}%</span>
                          <span className="paths">
                            {p.path_a.split(/[/\\]/).pop()} ↔ {p.path_b.split(/[/\\]/).pop()}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
                {report.duplicates.duplicate_pairs.length === 0 &&
                  report.duplicates.similar_pairs.length === 0 && (
                    <p className="advanced-empty">No duplicates or similar textures found.</p>
                  )}
              </div>
            )}

            {activeSection === 'cross' && (
              <div className="advanced-cross">
                {report.cross_material.recommendations.length > 0 && (
                  <div className="advanced-subsection">
                    <h4>Recommendations</h4>
                    <ul>
                      {report.cross_material.recommendations.map((r, i) => (
                        <li key={i}>{r}</li>
                      ))}
                    </ul>
                  </div>
                )}
                {report.cross_material.resolution_inconsistent && (
                  <div className="advanced-subsection">
                    <h4>Resolution distribution</h4>
                    <ul>
                      {report.cross_material.resolution_distributions.map((d, i) => (
                        <li key={i}>
                          {d.width}×{d.height}: {d.count} material(s)
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
                <div className="advanced-subsection">
                  <h4>Map coverage</h4>
                  <ul>
                    {report.cross_material.map_coverage.map((c) => (
                      <li key={c.slot}>
                        <span className="slot-badge">{c.slot}</span>
                        {c.present_count}/{c.total_count} ({c.coverage_percent.toFixed(0)}%)
                        {c.missing_in.length > 0 && (
                          <span className="missing"> — missing in: {c.missing_in.join(', ')}</span>
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            )}

            {activeSection === 'tileability' && (
              <div className="advanced-tileability">
                {report.tileability_analysis.filter((e) => e.needs_fix).length > 0 ? (
                  <ul>
                    {report.tileability_analysis
                      .filter((e) => e.needs_fix)
                      .map((e, i) => (
                        <li key={i} className="tileability-entry">
                          <div className="tileability-info">
                            <span className="slot-badge">{e.slot}</span>
                            <span className="edge-diff">edge diff: {e.edge_difference.toFixed(1)}</span>
                            <span className="path-short">{e.path.split(/[/\\]/).pop()}</span>
                          </div>
                          <button
                            type="button"
                            className="btn-fix"
                            onClick={() => handleFixTileability(e)}
                            disabled={fixingPath === e.path}
                          >
                            {fixingPath === e.path ? 'Fixing…' : 'Fix Tileability'}
                          </button>
                        </li>
                      ))}
                  </ul>
                ) : (
                  <p className="advanced-empty">All textures pass tileability check.</p>
                )}
                {report.tileability_analysis.filter((e) => !e.needs_fix).length > 0 && (
                  <div className="advanced-subsection">
                    <h4>OK (no fix needed)</h4>
                    <ul>
                      {report.tileability_analysis
                        .filter((e) => !e.needs_fix)
                        .map((e, i) => (
                          <li key={i}>
                            <span className="slot-badge">{e.slot}</span>
                            edge diff: {e.edge_difference.toFixed(1)} — {e.path.split(/[/\\]/).pop()}
                          </li>
                        ))}
                    </ul>
                  </div>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
