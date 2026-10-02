// ponytail: hand-drawn outline paths instead of an icon-library dependency — swap for one if this set outgrows sidebar nav
const PATHS = {
  home: 'M3 12 12 4l9 8M6 10v10h4v-6h4v6h4V10',
  chart: 'M4 20V11M12 20V5M20 20v-8M3 20h18',
  logs: 'M4 6h16M4 12h16M4 18h10',
  key: 'M14.5 9.5a3.5 3.5 0 1 1-3.4 4.3L4 21H2v-2l1-1h2v-2h2l2-2M13.5 6.5l4 4',
  layers: 'M12 3 3 8l9 5 9-5-9-5ZM3 12l9 5 9-5M3 16.5l9 5 9-5',
  play: 'M8 5v14l11-7z',
  users: 'M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8ZM2.5 20a6.5 6.5 0 0 1 13 0M17 10.2a3 3 0 1 0-1.8-5.4M15.3 13a5 5 0 0 1 6.2 4.8',
  inbox: 'M3 3h18v13l-3 3H6l-3-3V3ZM3 13h5l2 3h4l2-3h5',
  copy: 'M9 9h10v10H9zM5 15V5h10',
  logout: 'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9',
  route: 'M6 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6ZM6 9v6M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM18 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6ZM6 9c0 4 3 6 8 6h1M18 6h-3',
  info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM12 11v5M12 8h.01',
  sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M19.1 4.9l-1.4 1.4M6.3 17.7l-1.4 1.4',
  moon: 'M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z',
  chevronLeft: 'M15 19l-7-7 7-7',
  chevronRight: 'M9 5l7 7-7 7',
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({ name, className }: { name: IconName; className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
