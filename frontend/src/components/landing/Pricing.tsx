import { useState } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { fadeUp, inViewOnce, staggerParent } from './motion';

const TIERS = [
  {
    name: 'Solo',
    price: '$0',
    cadence: 'forever',
    blurb: 'For one developer wiring up a side project.',
    features: ['1 team', 'Unlimited provider keys', '10k routed requests / mo', 'Usage analytics'],
    cta: 'Start free',
  },
  {
    name: 'Team',
    price: '$49',
    cadence: 'per month',
    blurb: 'For a product team sharing credentials properly.',
    features: [
      'Unlimited teams & members',
      'Join requests and approvals',
      '2M routed requests / mo',
      'Cross-provider fallbacks',
      'Per-team spend breakdown',
    ],
    cta: 'Start 14-day trial',
    highlight: true,
  },
  {
    name: 'Enterprise',
    price: 'Talk to us',
    cadence: '',
    blurb: 'For when procurement gets involved.',
    features: ['SSO & SCIM', 'Self-hosted deployment', 'Custom retention', 'Priority support'],
    cta: 'Book a call',
  },
];

/**
 * Hovering a tier promotes it to the active choice and dims the others, so
 * the comparison follows the cursor instead of being fixed on whichever plan
 * we decided to call "popular". With nothing hovered it falls back to that
 * default, and reduced-motion users get the colour change without the lift.
 */
export function Pricing({ onSelect }: { onSelect: () => void }) {
  const reduce = useReducedMotion();
  const rise = fadeUp(reduce ? 0 : 24);
  const [hovered, setHovered] = useState<number | null>(null);
  const defaultIndex = TIERS.findIndex((t) => t.highlight);
  const activeIndex = hovered ?? defaultIndex;

  return (
    <section id="pricing" className="relative mx-auto max-w-6xl px-5 py-20 sm:px-8 sm:py-28">
      <motion.div
        variants={staggerParent(0.08)}
        initial="hidden"
        whileInView="visible"
        viewport={inViewOnce}
        className="mx-auto max-w-2xl text-center"
      >
        <motion.p variants={rise} className="text-sm font-medium tracking-tight text-blue-400">
          Pricing
        </motion.p>
        <motion.h2 variants={rise} className="mt-3 text-3xl font-bold leading-tight tracking-tight text-fg sm:text-4xl">
          You pay for routing, not for tokens
        </motion.h2>
        <motion.p variants={rise} className="mt-4 text-base leading-relaxed text-muted">
          Model spend goes to the vendors you already have accounts with, at whatever rate you negotiated. We never sit
          in the middle of that bill.
        </motion.p>
      </motion.div>

      <motion.div
        variants={staggerParent(0.08, 0.1)}
        initial="hidden"
        whileInView="visible"
        viewport={inViewOnce}
        onMouseLeave={() => setHovered(null)}
        className="mt-14 grid items-start gap-5 lg:grid-cols-3"
      >
        {TIERS.map((tier, i) => {
          const active = i === activeIndex;
          return (
            // Two elements on purpose. The outer one owns the scroll reveal,
            // the inner one owns the hover state. Both want to animate
            // `opacity`, and when a single element carries a parent-driven
            // variant *and* its own `animate` object, the variant wins — so
            // the sibling dimming silently never applied. Splitting them lets
            // each animation own its properties outright.
            <motion.div key={tier.name} variants={rise} className="h-full">
              <motion.div
                onMouseEnter={() => setHovered(i)}
                onFocusCapture={() => setHovered(i)}
                animate={{
                  opacity: hovered === null || active ? 1 : 0.55,
                  ...(reduce
                    ? {}
                    : {
                        y: active && hovered !== null ? -6 : 0,
                        scale: active && hovered !== null ? 1.01 : 1,
                      }),
                }}
                transition={{ type: 'spring', stiffness: 300, damping: 26 }}
                className={`relative flex h-full flex-col rounded-xl border p-6 backdrop-blur-sm transition-colors duration-300 ${
                  active ? 'border-blue-500/60 bg-surface shadow-2xl shadow-blue-950/40' : 'border-line bg-surface/50'
                }`}
              >
                {tier.highlight && (
                  <span className="absolute -top-3 left-6 rounded-full bg-blue-600 px-2.5 py-1 text-[10px] font-medium uppercase tracking-widest text-white">
                    Most picked
                  </span>
                )}

                <h3 className="text-sm font-medium tracking-tight text-fg">{tier.name}</h3>
                <p className="mt-4 flex items-baseline gap-1.5">
                  <span className="text-3xl font-semibold tracking-tight text-fg">{tier.price}</span>
                  {tier.cadence && <span className="text-xs text-muted">{tier.cadence}</span>}
                </p>
                <p className="mt-3 text-sm leading-relaxed text-muted">{tier.blurb}</p>

                <ul className="mt-6 flex-1 space-y-2.5">
                  {tier.features.map((feature) => (
                    <li key={feature} className="flex items-start gap-2.5 text-sm text-muted">
                      <svg
                        viewBox="0 0 20 20"
                        className={`mt-0.5 h-4 w-4 shrink-0 transition-colors duration-300 ${
                          active ? 'text-blue-400' : 'text-line'
                        }`}
                        fill="none"
                        stroke="currentColor"
                        strokeWidth={2}
                        aria-hidden
                      >
                        <path d="m4 10.5 4 4 8-9" strokeLinecap="round" strokeLinejoin="round" />
                      </svg>
                      {feature}
                    </li>
                  ))}
                </ul>

                <button
                  type="button"
                  onClick={onSelect}
                  className={`mt-7 rounded-full px-5 py-2.5 text-sm font-medium outline-none transition duration-200 focus-visible:ring-2 focus-visible:ring-blue-500/50 ${
                    active
                      ? 'bg-blue-600 text-white shadow-lg shadow-blue-600/25 hover:bg-blue-500 hover:shadow-blue-500/40'
                      : 'border border-line text-muted hover:border-blue-500/40 hover:text-fg'
                  }`}
                >
                  {tier.cta}
                </button>
              </motion.div>
            </motion.div>
          );
        })}
      </motion.div>
    </section>
  );
}
