import { motion, useReducedMotion } from 'framer-motion';
import { fadeUp, inViewOnce, staggerParent } from './motion';

const STEPS = [
  {
    n: '01',
    title: 'Bring the keys you already have',
    body: 'Paste in an OpenAI, Anthropic, Cohere or self-hosted credential. Anything speaking the OpenAI wire format works with just a base URL.',
  },
  {
    n: '02',
    title: 'Get one unified key back',
    body: 'A single vk_live_ key stands in front of all of them. Rotate it, revoke it, or hand a separate one to each environment.',
  },
  {
    n: '03',
    title: 'Point your app at one endpoint',
    body: 'Change the base URL and the key. Your existing OpenAI client keeps working — the model name decides where the request lands.',
  },
];

/** Numbered rail, connected by a vertical gradient line on desktop. */
export function HowItWorks() {
  const reduce = useReducedMotion();
  const rise = fadeUp(reduce ? 0 : 24);

  return (
    <section id="how" className="relative mx-auto max-w-6xl px-5 py-20 sm:px-8 sm:py-28">
      <motion.div
        variants={staggerParent(0.08)}
        initial="hidden"
        whileInView="visible"
        viewport={inViewOnce}
        className="grid gap-14 lg:grid-cols-[0.9fr_1.1fr] lg:items-start lg:gap-20"
      >
        <div className="max-w-md">
          <motion.p variants={rise} className="text-sm font-medium tracking-tight text-blue-400">
            How it works
          </motion.p>
          <motion.h2
            variants={rise}
            className="mt-3 text-3xl font-bold leading-tight tracking-tight text-fg sm:text-4xl"
          >
            Three steps, and nothing to rewrite
          </motion.h2>
          <motion.p variants={rise} className="mt-4 text-base leading-relaxed text-muted">
            The integration is a base URL and a key. Everything else — which vendor serves which model, what happens
            when one is down — is configuration you can change without shipping code.
          </motion.p>
        </div>

        <ol className="relative space-y-8">
          <span
            aria-hidden
            className="absolute left-[19px] top-2 hidden h-[calc(100%-2rem)] w-px bg-gradient-to-b from-blue-500/50 via-blue-500/20 to-transparent sm:block"
          />
          {STEPS.map((step) => (
            <motion.li key={step.n} variants={rise} className="relative flex gap-5">
              <span className="relative z-10 flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-line bg-surface font-mono text-xs text-blue-400">
                {step.n}
              </span>
              <div className="pt-1.5">
                <h3 className="text-base font-semibold tracking-tight text-fg">{step.title}</h3>
                <p className="mt-1.5 text-sm leading-relaxed text-muted">{step.body}</p>
              </div>
            </motion.li>
          ))}
        </ol>
      </motion.div>
    </section>
  );
}
