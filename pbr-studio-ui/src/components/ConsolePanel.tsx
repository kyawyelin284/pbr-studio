import { useRef, useEffect } from 'react';

export type LogLevel = 'info' | 'success' | 'warn' | 'error';

export interface LogEntry {
  id: number;
  timestamp: string;
  level: LogLevel;
  message: string;
}

interface ConsolePanelProps {
  entries: LogEntry[];
  maxLines?: number;
}

export function ConsolePanel({ entries }: ConsolePanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [entries]);

  const levelClass = (level: LogLevel) => {
    switch (level) {
      case 'error': return 'log-error';
      case 'warn': return 'log-warn';
      case 'success': return 'log-success';
      default: return 'log-info';
    }
  };

  return (
    <div className="panel panel-console">
      <div className="panel-header">Console</div>
      <div className="console-output" ref={containerRef}>
        {entries.length === 0 ? (
          <div className="console-empty">No messages yet. Drop a material folder or analyze to see output.</div>
        ) : (
          entries.map((e) => (
            <div key={e.id} className={`console-line ${levelClass(e.level)}`}>
              <span className="console-time">{e.timestamp}</span>
              <span className="console-msg">{e.message}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
