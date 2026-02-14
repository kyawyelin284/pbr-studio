import { useState, useEffect, useCallback } from 'react';

export interface AuditEntry {
  timestamp: string;
  action: 'validation' | 'optimization' | 'report_generation';
  material_path: string | null;
  score: number | null;
  passed: boolean | null;
  min_score: number | null;
  issue_count: number | null;
  error_count: number | null;
  warning_count: number | null;
  output_path: string | null;
  preset: string | null;
  format: string | null;
  texture_count: number | null;
  certified: boolean;
}

interface AuditLogPanelProps {
  isTauri: boolean;
  refreshTrigger?: number; // Increment to trigger refresh
}

export function AuditLogPanel({ isTauri, refreshTrigger = 0 }: AuditLogPanelProps) {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [limit] = useState(50);

  const loadEntries = useCallback(async () => {
    if (!isTauri) return;
    setLoading(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const json = await invoke<string>('get_audit_log', { limit });
      const data = JSON.parse(json) as AuditEntry[];
      setEntries(data);
    } catch {
      setEntries([]);
    } finally {
      setLoading(false);
    }
  }, [isTauri, limit]);

  useEffect(() => {
    loadEntries();
  }, [loadEntries, refreshTrigger]);

  const actionLabel = (action: string) => {
    switch (action) {
      case 'validation': return 'validation';
      case 'optimization': return 'optimization';
      case 'report_generation': return 'report';
      default: return action;
    }
  };

  const handleExport = useCallback(async () => {
    if (!isTauri || exporting) return;
    setExporting(true);
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const { invoke } = await import('@tauri-apps/api/core');
      const path = await save({
        defaultPath: `audit-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [
          { name: 'JSON', extensions: ['json'] },
          { name: 'Text', extensions: ['txt'] },
        ],
      });
      if (path) {
        const format = path.toLowerCase().endsWith('.txt') ? 'text' : 'json';
        await invoke('export_audit_log', { outputPath: path, format, limit });
        const { message } = await import('@tauri-apps/plugin-dialog');
        await message(`Audit log exported to:\n${path}`, { title: 'Export complete', kind: 'info' });
      }
    } catch {
      // ignore
    } finally {
      setExporting(false);
    }
  }, [isTauri, exporting, limit]);

  return (
    <div className="panel panel-console">
      <div className="panel-header panel-header-row">
        <span>Audit Log</span>
        {isTauri && (
          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              type="button"
              className="audit-refresh-btn"
              onClick={loadEntries}
              disabled={loading}
              title="Refresh"
            >
              {loading ? '...' : '↻'}
            </button>
            <button
              type="button"
              className="audit-export-btn"
              onClick={handleExport}
              disabled={exporting || entries.length === 0}
              title="Export to JSON or text file"
            >
              {exporting ? '...' : 'Export'}
            </button>
          </div>
        )}
      </div>
      <div className="console-output audit-output">
        {entries.length === 0 && !loading ? (
          <div className="console-empty">
            No audit entries yet. Run validation, optimization, or export a report to see activity.
          </div>
        ) : (
          entries.map((e, i) => (
            <div key={`${e.timestamp}-${i}`} className="audit-line">
              <span className="audit-time">{e.timestamp}</span>
              <span className={`audit-action audit-action-${e.action}`}>
                [{actionLabel(e.action)}]
              </span>
              <span className="audit-path" title={e.material_path ?? '-'}>
                {e.material_path ? truncatePath(e.material_path) : '-'}
              </span>
              {e.score != null && (
                <span className="audit-score">
                  score={e.score}
                  {e.min_score != null && `/${e.min_score}`}
                </span>
              )}
              {e.certified && (
                <span className="audit-certified" title="Material Certified for Pipeline">✓ certified</span>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function truncatePath(path: string, maxLen = 48): string {
  if (path.length <= maxLen) return path;
  const parts = path.split(/[/\\]/);
  const last = parts[parts.length - 1] ?? path;
  if (last.length >= maxLen - 6) return '.../ ' + last.slice(-(maxLen - 6));
  return '.../' + last;
}
