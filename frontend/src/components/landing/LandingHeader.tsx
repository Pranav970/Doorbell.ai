import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Logo } from '../Shell';

const LINKS = [
  { href: '#features', label: 'Features' },
  { href: '#how', label: 'How it works' },
  { href: '#pricing', label: 'Pricing' },
];

/**
 * Sticky glass header: logo left, links centred, CTA right.
 *
 * The border and blur only appear once the page has scrolled, so the header
 * is invisible against the hero and materialises as content passes under it.
 */
export function LandingHeader({ onGetStarted }: { onGetStarted: () => void }) {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 12);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  return (
    <header
      className={`sticky top-0 z-40 transition duration-300 ${
        scrolled ? 'border-b border-line bg-ink/70 backdrop-blur-md' : 'border-b border-transparent'
      }`}
    >
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-5 sm:px-8">
        <Link to="/welcome" className="flex shrink-0 items-center gap-2" aria-label="Prismaxis home">
          <Logo className="h-7 w-auto" />
        </Link>

        <nav aria-label="Primary" className="hidden items-center gap-1 md:flex">
          {LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="rounded-lg px-3 py-2 text-sm text-muted outline-none transition hover:text-fg focus-visible:ring-2 focus-visible:ring-blue-500/50"
            >
              {link.label}
            </a>
          ))}
        </nav>

        <div className="flex shrink-0 items-center gap-2">
          <Link
            to="/login"
            className="hidden rounded-lg px-3 py-2 text-sm text-muted outline-none transition hover:text-fg focus-visible:ring-2 focus-visible:ring-blue-500/50 sm:block"
          >
            Sign in
          </Link>
          <button
            type="button"
            onClick={onGetStarted}
            className="rounded-full bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-lg shadow-blue-600/20 outline-none transition duration-200 hover:scale-[1.05] hover:bg-blue-500 hover:shadow-blue-500/40 focus-visible:ring-2 focus-visible:ring-blue-500/50 active:scale-100"
          >
            Get started
          </button>
        </div>
      </div>
    </header>
  );
}
