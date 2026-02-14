import { useCallback } from 'react';

export interface MaterialEntry {
  path: string;
  name: string;
  score: number | null;
  loading?: boolean;
}

interface MaterialListProps {
  materials: MaterialEntry[];
  selectedIndex: number | null;
  selectedIndices: number[];
  compareIndex: number | null;
  onSelect: (index: number, evt?: React.MouseEvent) => void;
  onSelectAll: () => void;
  onClearSelection: () => void;
  onCompare: (index: number | null) => void;
  onRemove: (index: number) => void;
  onBatchAnalyze?: () => void;
  onBatchExport?: () => void;
  canUndo: boolean;
  canRedo: boolean;
  undoCount?: number;
  redoCount?: number;
  onUndo: () => void;
  onRedo: () => void;
}

function getScoreColor(score: number | null): string {
  if (score == null) return 'var(--text-muted)';
  if (score >= 80) return 'var(--score-good)';
  if (score >= 50) return 'var(--score-medium)';
  return 'var(--score-low)';
}

export function MaterialList({
  materials,
  selectedIndex,
  selectedIndices,
  compareIndex,
  onSelect,
  onSelectAll,
  onClearSelection,
  onCompare,
  onRemove,
  onBatchAnalyze,
  onBatchExport,
  canUndo,
  canRedo,
  undoCount = 0,
  redoCount = 0,
  onUndo,
  onRedo,
}: MaterialListProps) {
  const handleCompareClick = useCallback(
    (e: React.MouseEvent, index: number) => {
      e.stopPropagation();
      onCompare(compareIndex === index ? null : index);
    },
    [compareIndex, onCompare]
  );

  const handleItemClick = useCallback(
    (e: React.MouseEvent, index: number) => {
      onSelect(index, e);
    },
    [onSelect]
  );

  const handleCheckboxClick = useCallback(
    (e: React.MouseEvent, index: number) => {
      e.stopPropagation();
      onSelect(index, e);
    },
    [onSelect]
  );

  const hasSelection = selectedIndices.length > 0;

  return (
    <div className="material-list">
      <div className="material-list-header">
        <span className="texture-slot-label">
          Materials {materials.length > 0 && `(${materials.length})`}
        </span>
        <div className="material-list-header-actions">
          {materials.length > 0 && (
            <>
              <button
                type="button"
                title="Select all"
                onClick={onSelectAll}
                className="undo-redo-btn"
              >
                All
              </button>
              {hasSelection && (
                <button
                  type="button"
                  title="Clear selection"
                  onClick={onClearSelection}
                  className="undo-redo-btn"
                >
                  None
                </button>
              )}
            </>
          )}
          <button
            type="button"
            title={canUndo ? `Undo (${undoCount} step${undoCount !== 1 ? 's' : ''} available)` : 'Undo'}
            disabled={!canUndo}
            onClick={onUndo}
            className="undo-redo-btn"
          >
            ↶
          </button>
          <button
            type="button"
            title={canRedo ? `Redo (${redoCount} step${redoCount !== 1 ? 's' : ''} available)` : 'Redo'}
            disabled={!canRedo}
            onClick={onRedo}
            className="undo-redo-btn"
          >
            ↷
          </button>
        </div>
      </div>
      {hasSelection && (onBatchAnalyze || onBatchExport) && (
        <div className="material-list-batch-actions">
          {onBatchAnalyze && (
            <button type="button" onClick={onBatchAnalyze} className="batch-btn">
              Analyze {selectedIndices.length} selected
            </button>
          )}
          {onBatchExport && (
            <button type="button" onClick={onBatchExport} className="batch-btn">
              Export report {selectedIndices.length} selected
            </button>
          )}
        </div>
      )}
      <div className="material-list-items">
        {materials.length === 0 ? (
          <div className="empty-state small">Drop folders or select to add</div>
        ) : (
          materials.map((m, i) => (
            <div
              key={`${m.path}-${i}`}
              className={`material-list-item ${selectedIndex === i ? 'selected' : ''} ${selectedIndices.includes(i) ? 'multi-selected' : ''} ${compareIndex === i ? 'compare' : ''}`}
              onClick={(e) => handleItemClick(e, i)}
            >
              <input
                type="checkbox"
                checked={selectedIndices.includes(i)}
                onChange={() => {}}
                onClick={(e) => handleCheckboxClick(e, i)}
                className="material-list-checkbox"
                aria-label={`Select ${m.name}`}
              />
              <div className="material-list-item-main">
                <span className="material-list-name" title={m.path}>
                  {m.name}
                </span>
                {m.loading ? (
                  <span className="material-list-score loading">…</span>
                ) : (
                  <span
                    className="material-list-score"
                    style={{ color: getScoreColor(m.score) }}
                  >
                    {m.score != null ? m.score : '—'}
                  </span>
                )}
              </div>
              <div className="material-list-item-actions">
                <button
                  type="button"
                  title={compareIndex === i ? 'Remove from comparison' : 'Compare'}
                  onClick={(e) => handleCompareClick(e, i)}
                  className={`compare-btn ${compareIndex === i ? 'active' : ''}`}
                >
                  A↔B
                </button>
                <button
                  type="button"
                  title="Remove"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemove(i);
                  }}
                  className="remove-btn"
                >
                  ×
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
