interface Issue {
  rule_id: string;
  severity: 'critical' | 'major' | 'minor' | 'error' | 'warning' | 'info';
  message: string;
}

interface OptimizationSuggestion {
  category: string;
  message: string;
  priority?: number;
  details?: string;
}

interface VramEstimate {
  bytes: number;
  formatted: string;
  include_mipmaps?: boolean;
  packed_orm: boolean;
}

interface AiInsights {
  classification?: string;
  classification_confidence?: number;
  smart_suggestions?: { category: string; message: string; confidence: number }[];
  anomalies?: { slot: string; message: string; score: number }[];
}

interface MaterialReport {
  issues: Issue[];
  optimization_suggestions: OptimizationSuggestion[];
  passed: boolean;
  error_count: number;
  warning_count: number;
  score?: number;
  vram_estimate?: VramEstimate;
  ai_insights?: AiInsights;
}

interface ValidationPanelProps {
  report?: MaterialReport | null;
  /** Second report when in side-by-side comparison mode */
  compareReport?: MaterialReport | null;
  compareMode?: boolean;
  compareName?: string;
  loading?: boolean;
  error?: string;
  /** Path of current material folder (for export) */
  materialPath?: string;
  /** All material paths (for batch export) */
  allPaths?: string[];
  /** Whether Tauri is available */
  isTauri?: boolean;
  onExportReport?: (paths: string[], format: 'html' | 'pdf') => void;
}

function getScore(report: MaterialReport): number {
  if (report.score != null) return report.score;
  const errorPenalty = 20;
  const warningPenalty = 10;
  const deduction = report.error_count * errorPenalty + report.warning_count * warningPenalty;
  return Math.max(0, 100 - deduction);
}

function severityToClass(severity: string): string {
  if (severity === 'critical' || severity === 'error') return 'severity-error';
  if (severity === 'major' || severity === 'warning') return 'severity-warning';
  return 'severity-info';
}

function getScoreColor(score: number): string {
  if (score >= 80) return 'var(--score-good)';
  if (score >= 50) return 'var(--score-medium)';
  return 'var(--score-low)';
}

export function ValidationPanel({ report, compareReport, compareMode, compareName, loading, error, materialPath, allPaths = [], isTauri, onExportReport }: ValidationPanelProps) {
  if (loading) {
    return (
      <div className="panel panel-right">
        <div className="panel-header">Validation Results</div>
        <div className="panel-content">
          <div className="empty-state">Analyzing...</div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="panel panel-right">
        <div className="panel-header">Validation Results</div>
        <div className="panel-content">
          <div className="issue-item severity-error">
            <div className="issue-rule">Error</div>
            <div className="issue-message">{error}</div>
          </div>
        </div>
      </div>
    );
  }

  if (!report) {
    return (
      <div className="panel panel-right">
        <div className="panel-header">Validation Results</div>
        <div className="panel-content">
          <div className="empty-state">
            Select a material folder and click Analyze to run validation.
          </div>
        </div>
      </div>
    );
  }

  const score = getScore(report);
  const compareScore = compareReport != null ? getScore(compareReport) : null;

  return (
    <div className="panel panel-right">
      <div className="panel-header">Validation Results</div>
      <div className="panel-content">
        {/* Material Score(s) - side-by-side when comparing */}
        <div className={compareMode ? 'score-section-compare' : ''}>
          <div className="score-section">
            <div className="score-label">Material Score</div>
            <div
              className="score-value"
              style={{ color: getScoreColor(score) }}
            >
              {score}
            </div>
            <div className="score-status">
              {report.passed ? 'Passed' : 'Needs attention'}
            </div>
          </div>
          {compareMode && compareReport != null && (
            <>
              <div className="score-section-divider" />
              <div className="score-section">
                <div className="score-label">{compareName ? `${compareName} Score` : 'Compare'}</div>
                <div
                  className="score-value"
                  style={{ color: getScoreColor(compareScore ?? 0) }}
                >
                  {compareScore}
                </div>
                <div className="score-status">
                  {compareReport.passed ? 'Passed' : 'Needs attention'}
                </div>
              </div>
            </>
          )}
        </div>

        {/* AI Insights */}
        {report.ai_insights && (report.ai_insights.classification || (report.ai_insights.anomalies && report.ai_insights.anomalies.length > 0)) && (
          <div className="score-section ai-section">
            <div className="score-label">AI Insights</div>
            {report.ai_insights.classification && (
              <div className="ai-class">
                Classification: <strong>{report.ai_insights.classification}</strong>
                {report.ai_insights.classification_confidence != null && (
                  <span className="ai-confidence"> ({Math.round(report.ai_insights.classification_confidence * 100)}%)</span>
                )}
              </div>
            )}
            {report.ai_insights.anomalies && report.ai_insights.anomalies.length > 0 && (
              <div className="ai-anomalies">
                <div className="section-title small">Anomalies detected</div>
                <ul className="anomaly-list">
                  {report.ai_insights.anomalies.map((a, i) => (
                    <li key={i} className="anomaly-item">
                      <span className="anomaly-slot">{a.slot}</span>: {a.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}

        {/* VRAM Estimate */}
        {report.vram_estimate && (
          <div className="score-section vram-section">
            <div className="score-label">VRAM Estimate</div>
            <div className="score-value vram-value">{report.vram_estimate.formatted}</div>
            <div className="score-status">
              {report.vram_estimate.packed_orm ? 'With packed ORM' : 'Unpacked'}
            </div>
          </div>
        )}

        {/* Issues */}
        <div className="section">
          <div className="section-title">Issues</div>
          {report.issues.length === 0 ? (
            <div className="empty-state small">No issues found</div>
          ) : (
            <div className="issue-list">
              {report.issues.map((issue, i) => (
                <div
                  key={i}
                  className={`issue-item ${severityToClass(issue.severity)}`}
                >
                  <div className="issue-rule">{issue.rule_id}</div>
                  <div className="issue-message">{issue.message}</div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Suggested Optimizations */}
        <div className="section">
          <div className="section-title">Suggested Optimizations</div>
          {report.optimization_suggestions.length === 0 ? (
            <div className="empty-state small">No suggestions</div>
          ) : (
            <div className="suggestion-list">
              {report.optimization_suggestions.map((suggestion, i) => (
                <div key={i} className="suggestion-item">
                  <div className="suggestion-category">{suggestion.category}</div>
                  <div className="suggestion-message">{suggestion.message}</div>
                  {suggestion.details && (
                    <div className="suggestion-details">{suggestion.details}</div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Export Report */}
        {isTauri && onExportReport && (materialPath || allPaths.length > 0) && (
          <div className="section export-section">
            <div className="section-title">Export Report</div>
            <div className="export-buttons">
              <button
                type="button"
                className="export-btn"
                onClick={() => onExportReport(allPaths.length > 0 ? allPaths : (materialPath ? [materialPath] : []), 'html')}
              >
                Export HTML
              </button>
              <button
                type="button"
                className="export-btn"
                onClick={() => onExportReport(allPaths.length > 0 ? allPaths : (materialPath ? [materialPath] : []), 'pdf')}
              >
                Export PDF
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
