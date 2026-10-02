import { useTheme } from '../context/ThemeContext';
import { Icon } from './icons';

export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  const next = theme === 'dark' ? 'light' : 'dark';
  return (
    <button
      type="button"
      onClick={toggleTheme}
      aria-label={`Switch to ${next} theme`}
      title={`Switch to ${next} theme`}
      className="inline-flex h-9 w-9 items-center justify-center rounded-lg border border-line text-muted outline-none transition hover:border-blue-600/60 hover:text-blue-400 focus-visible:ring-2 focus-visible:ring-blue-500/50"
    >
      <Icon name={theme === 'dark' ? 'sun' : 'moon'} className="h-4 w-4" />
    </button>
  );
}
