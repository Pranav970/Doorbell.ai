import type { ReactNode } from 'react';

/**
 * Minimal dark data table. Scrolls horizontally inside its own container so
 * a wide row never pushes the page sideways.
 */
export function Table({ columns, children }: { columns: ReactNode[]; children: ReactNode }) {
  return (
    <div className="-mx-5 overflow-x-auto px-5">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-line">
            {columns.map((col, i) => (
              <th key={i} className="whitespace-nowrap py-2 pr-6 text-xs font-medium uppercase tracking-wider text-muted">
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-line/70">{children}</tbody>
      </table>
    </div>
  );
}

export function Tr({ children }: { children: ReactNode }) {
  return <tr className="transition hover:bg-blue-500/5">{children}</tr>;
}

export function Td({ children, className = '' }: { children?: ReactNode; className?: string }) {
  return <td className={`whitespace-nowrap py-2.5 pr-6 text-fg ${className}`}>{children}</td>;
}
