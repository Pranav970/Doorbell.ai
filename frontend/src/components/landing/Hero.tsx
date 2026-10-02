import { motion, useReducedMotion } from 'framer-motion';
import { fadeUp, staggerParent } from './motion';

/**
 * Split hero: copy on the left, a floating console mockup on the right.
 *
 * The mockup is deliberately a real-looking request/response rather than a
 * stock screenshot — it shows the one thing the product actually does (one
 * endpoint, one key, whichever provider you configured behind it).
 */
export function Hero({ onGetStarted }: { onGetStarted: () => void }) {
  const reduce = useReducedMotion();
  const rise = fadeUp(reduce ? 0 : 20);

  return (
    <section className="relative overflow-hidden">
      {/* Two blue glows on black — same decorative budget as AuthShell. */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_55%_45%_at_15%_-10%,rgba(37,99,235,0.20),transparent),radial-gradient(ellipse_50%_40%_at_95%_0%,rgba(59,130,246,0.14),transparent)]"
      />

      <div className="relative mx-auto grid max-w-6xl gap-12 px-5 py-20 sm:px-8 sm:py-28 lg:grid-cols-2 lg:items-center lg:gap-16">
        {/* min-w-0: grid items default to `min-width: auto`, so the code
            block in the mockup would otherwise set a min-content floor wider
            than a phone and push both columns past the viewport. */}
        <motion.div variants={staggerParent(0.1)} initial="hidden" animate="visible" className="min-w-0">
          <motion.span
            variants={rise}
            className="inline-flex items-center gap-2 rounded-full border border-line bg-surface/60 px-3 py-1 text-xs font-medium text-blue-300 backdrop-blur-sm"
          >
            <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-blue-400 shadow-[0_0_8px_rgba(59,130,246,0.9)]" />
            BYOK — your keys, your accounts
          </motion.span>

          <motion.h1
            variants={rise}
            className="mt-6 text-4xl font-bold leading-[1.05] tracking-tight text-fg sm:text-5xl lg:text-6xl"
          >
            One key.
            <br />
            Every model.
            <br />
            <span className="bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">
              Your own credentials.
            </span>
          </motion.h1>

          <motion.p variants={rise} className="mt-6 max-w-lg text-base leading-relaxed text-muted sm:text-lg">
            Prismaxis is a bring-your-own-key gateway. Point your app at one OpenAI-compatible endpoint and route it to
            OpenAI, Anthropic, Gemini, Cohere, Mistral or a model you host yourself — without your provider keys ever
            leaving your own accounts.
          </motion.p>

          <motion.div variants={rise} className="mt-9 flex flex-col gap-3 sm:flex-row">
            <button
              type="button"
              onClick={onGetStarted}
              className="rounded-full bg-blue-600 px-7 py-3 text-sm font-medium text-white shadow-lg shadow-blue-600/25 outline-none transition duration-200 hover:scale-[1.05] hover:bg-blue-500 hover:shadow-xl hover:shadow-blue-500/40 focus-visible:ring-2 focus-visible:ring-blue-500/50 active:scale-100"
            >
              Get started free
            </button>
            <a
              href="#how"
              className="rounded-full border border-line px-7 py-3 text-center text-sm font-medium text-muted outline-none transition duration-200 hover:scale-[1.03] hover:border-blue-500/40 hover:text-fg focus-visible:ring-2 focus-visible:ring-blue-500/50 active:scale-100"
            >
              See how routing works
            </a>
          </motion.div>

          <motion.dl variants={rise} className="mt-12 grid max-w-md grid-cols-3 gap-6">
            {[
              { value: '1', label: 'endpoint to integrate' },
              { value: '0', label: 'keys we can read' },
              { value: '∞', label: 'providers you can add' },
            ].map((stat) => (
              <div key={stat.label}>
                <dt className="sr-only">{stat.label}</dt>
                <dd>
                  <span className="block font-mono text-2xl font-light text-blue-300">{stat.value}</span>
                  <span className="mt-1 block text-xs leading-snug text-muted">{stat.label}</span>
                </dd>
              </div>
            ))}
          </motion.dl>
        </motion.div>

        <motion.div
          initial={reduce ? { opacity: 0 } : { opacity: 0, y: 28 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.25, ease: [0.16, 1, 0.3, 1] }}
          className="relative min-w-0"
        >
          <ConsoleMockup floating={!reduce} />
        </motion.div>
      </div>
    </section>
  );
}

function ConsoleMockup({ floating }: { floating: boolean }) {
  return (
    <motion.div
      animate={floating ? { y: [0, -10, 0] } : undefined}
      transition={floating ? { duration: 6, repeat: Infinity, ease: 'easeInOut' } : undefined}
      className="relative rounded-2xl border border-line bg-surface/80 shadow-2xl shadow-blue-950/30 backdrop-blur-sm"
    >
      <div
        aria-hidden
        className="pointer-events-none absolute -inset-px rounded-2xl bg-gradient-to-b from-blue-500/20 to-transparent opacity-60"
      />

      <div className="relative flex items-center gap-2 border-b border-line px-4 py-3">
        <span aria-hidden className="h-2.5 w-2.5 rounded-full bg-line" />
        <span aria-hidden className="h-2.5 w-2.5 rounded-full bg-line" />
        <span aria-hidden className="h-2.5 w-2.5 rounded-full bg-line" />
        <span className="ml-2 font-mono text-[11px] text-muted">POST /v1/chat/completions</span>
      </div>

      <div className="relative space-y-4 p-5">
        <pre className="overflow-x-auto rounded-xl border border-line bg-raised p-4 font-mono text-[11px] leading-relaxed text-muted sm:text-xs">
          <code>
            {'curl https://gateway.prismaxis.dev/v1/chat/completions \\\n'}
            {'  -H "authorization: Bearer '}
            <span className="text-blue-400">vk_live_••••••••</span>
            {'" \\\n'}
            {'  -d \'{"model": "'}
            <span className="text-blue-400">command-a-03-2025</span>
            {'"}\''}
          </code>
        </pre>

        <div className="flex items-center justify-between rounded-xl border border-line bg-raised px-4 py-3">
          <div className="flex items-center gap-2.5">
            <span
              aria-hidden
              className="h-2 w-2 rounded-full bg-blue-400 shadow-[0_0_10px_rgba(59,130,246,0.9)]"
            />
            <span className="text-xs text-muted">Routed to</span>
            <span className="font-mono text-xs text-fg">cohere</span>
          </div>
          <span className="font-mono text-xs text-blue-300">302 ms</span>
        </div>

        <div>
          <div className="mb-2 flex items-center justify-between text-[11px] text-muted">
            <span>Requests today</span>
            <span className="font-mono text-blue-300">12.4k</span>
          </div>
          <Sparkline />
        </div>
      </div>
    </motion.div>
  );
}

/** Hand-drawn sparkline — no chart library needed for a decorative mockup. */
function Sparkline() {
  const points = [8, 14, 11, 20, 17, 26, 22, 31, 28, 38, 34, 44];
  const max = Math.max(...points);
  const path = points
    .map((p, i) => `${(i / (points.length - 1)) * 100},${40 - (p / max) * 34}`)
    .join(' L ');

  return (
    <svg viewBox="0 0 100 40" preserveAspectRatio="none" className="h-12 w-full" aria-hidden>
      <defs>
        <linearGradient id="hero-spark" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.45" />
          <stop offset="100%" stopColor="#3b82f6" stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={`M ${path} L 100,40 L 0,40 Z`} fill="url(#hero-spark)" />
      <path d={`M ${path}`} fill="none" stroke="#60a5fa" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
