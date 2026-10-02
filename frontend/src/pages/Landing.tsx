import { useCallback, useState } from 'react';
import { Link } from 'react-router-dom';
import { Logo } from '../components/Shell';
import { ThemeToggle } from '../components/ThemeToggle';
import { LandingHeader } from '../components/landing/LandingHeader';
import { Hero } from '../components/landing/Hero';
import { Features } from '../components/landing/Features';
import { HowItWorks } from '../components/landing/HowItWorks';
import { Pricing } from '../components/landing/Pricing';
import { GradientDivider, WaveDivider } from '../components/landing/Divider';
import { GetStartedModal } from '../components/landing/GetStartedModal';

/**
 * Public marketing page.
 *
 * Deliberately its own route rather than a replacement for `/`, so the
 * existing signed-in/signed-out redirect logic in `App.tsx` is untouched. See
 * the note in that file for the one-line change that makes this the front
 * door instead.
 *
 * Every "Get started" affordance on the page — header, hero, pricing, closing
 * band — funnels into the same modal, so there is one place to change what
 * conversion means.
 */
export function LandingPage() {
  const [modalOpen, setModalOpen] = useState(false);
  const openModal = useCallback(() => setModalOpen(true), []);
  const closeModal = useCallback(() => setModalOpen(false), []);

  return (
    <div className="min-h-screen bg-ink text-fg">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-lg focus:bg-blue-600 focus:px-4 focus:py-2 focus:text-sm focus:text-white"
      >
        Skip to content
      </a>

      <LandingHeader onGetStarted={openModal} />

      <main id="main">
        <Hero onGetStarted={openModal} />
        <WaveDivider />
        <Features />
        <GradientDivider />
        <HowItWorks />
        <WaveDivider flip />
        <Pricing onSelect={openModal} />
        <ClosingBand onGetStarted={openModal} />
      </main>

      <LandingFooter />
      <GetStartedModal open={modalOpen} onClose={closeModal} />
    </div>
  );
}

function ClosingBand({ onGetStarted }: { onGetStarted: () => void }) {
  return (
    <section className="mx-auto max-w-6xl px-5 pb-24 sm:px-8">
      <div className="relative overflow-hidden rounded-2xl border border-line bg-surface/60 px-6 py-14 text-center backdrop-blur-sm sm:px-12">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_50%_70%_at_50%_0%,rgba(59,130,246,0.18),transparent)]"
        />
        <div className="relative">
          <h2 className="text-2xl font-bold tracking-tight text-fg sm:text-3xl">
            Stop writing the same provider adapter twice
          </h2>
          <p className="mx-auto mt-3 max-w-xl text-sm leading-relaxed text-muted sm:text-base">
            Bring the keys you already own. Get one endpoint, one key, and a routing table you can change without a
            deploy.
          </p>
          <button
            type="button"
            onClick={onGetStarted}
            className="mt-8 rounded-full bg-blue-600 px-7 py-3 text-sm font-medium text-white shadow-lg shadow-blue-600/25 outline-none transition duration-200 hover:scale-[1.05] hover:bg-blue-500 hover:shadow-xl hover:shadow-blue-500/40 focus-visible:ring-2 focus-visible:ring-blue-500/50 active:scale-100"
          >
            Get started free
          </button>
        </div>
      </div>
    </section>
  );
}

function LandingFooter() {
  return (
    <footer className="border-t border-line">
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-5 px-5 py-8 sm:flex-row sm:px-8">
        <div className="flex items-center gap-3">
          <Logo className="h-6 w-auto" />
          <span className="text-xs text-muted">BYOK LLM gateway</span>
        </div>
        <nav aria-label="Footer" className="flex items-center gap-5 text-xs text-muted">
          <a href="#features" className="transition hover:text-fg">
            Features
          </a>
          <a href="#pricing" className="transition hover:text-fg">
            Pricing
          </a>
          <Link to="/login" className="transition hover:text-fg">
            Sign in
          </Link>
          <ThemeToggle />
        </nav>
      </div>
    </footer>
  );
}
