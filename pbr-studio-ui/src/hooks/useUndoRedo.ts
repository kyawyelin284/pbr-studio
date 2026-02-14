import { useState, useCallback } from 'react';

export interface HistoryState<T> {
  past: T[];
  present: T;
  future: T[];
}

export type UndoActionType = 'add' | 'remove' | 'validation' | 'refresh' | 'batch';

export interface UndoActionMeta {
  type: UndoActionType;
  materialPath?: string;
  materialName?: string;
  /** Human-readable description for UI */
  label?: string;
}

export function useUndoRedo<T>(initial: T, maxHistory = 50) {
  const [state, setState] = useState<HistoryState<T>>({
    past: [],
    present: initial,
    future: [],
  });

  /**
   * Update state and push to undo history. Use for user-initiated actions:
   * add material, remove material, validation, refresh, etc.
   */
  const set = useCallback(
    (valueOrUpdater: T | ((prev: T) => T)) => {
      setState((s) => {
        const newPresent =
          typeof valueOrUpdater === 'function'
            ? (valueOrUpdater as (prev: T) => T)(s.present)
            : valueOrUpdater;
        return {
          past: [...s.past.slice(-(maxHistory - 1)), s.present],
          present: newPresent,
          future: [],
        };
      });
    },
    [maxHistory]
  );

  /**
   * Update state without adding to undo history. Use for automatic updates
   * (e.g. live file-watch refresh) that should not be revertible.
   */
  const setWithoutHistory = useCallback((valueOrUpdater: T | ((prev: T) => T)) => {
    setState((s) => {
      const newPresent =
        typeof valueOrUpdater === 'function'
          ? (valueOrUpdater as (prev: T) => T)(s.present)
          : valueOrUpdater;
      return {
        ...s,
        present: newPresent,
      };
    });
  }, []);

  const undo = useCallback(() => {
    setState((s) => {
      if (s.past.length === 0) return s;
      const previous = s.past[s.past.length - 1];
      const newPast = s.past.slice(0, -1);
      return {
        past: newPast,
        present: previous,
        future: [s.present, ...s.future],
      };
    });
  }, []);

  const redo = useCallback(() => {
    setState((s) => {
      if (s.future.length === 0) return s;
      const next = s.future[0];
      const newFuture = s.future.slice(1);
      return {
        past: [...s.past, s.present],
        present: next,
        future: newFuture,
      };
    });
  }, []);

  const reset = useCallback((value: T) => {
    setState({ past: [], present: value, future: [] });
  }, []);

  return {
    state: state.present,
    set,
    setWithoutHistory,
    undo,
    redo,
    reset,
    canUndo: state.past.length > 0,
    canRedo: state.future.length > 0,
    /** Number of undo steps available */
    undoCount: state.past.length,
    /** Number of redo steps available */
    redoCount: state.future.length,
  };
}
