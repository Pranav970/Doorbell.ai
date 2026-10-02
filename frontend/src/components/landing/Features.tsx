import { motion, useReducedMotion } from 'framer-motion';
import { Icon, type IconName } from '../icons';
import { fadeUp, inViewOnce, staggerParent } from './motion';

const FEATURES: { icon: IconName; title: string; body: string }[] = [
  {
    icon: 'key',
    title: 'Your keys stay yours',
    body: 'Provider credentials are sealed with AES-256-GCM envelope encryption before they touch the database, and carry a key version so they can be rotated without a migration.',
  },
  {
    icon: 'layers',
    title: 'One endpoint, every vendor',
    body: 'An OpenAI-compatible surface in front of OpenAI, Anthropic and Gemini natively — and any other vendor, or your own deployment, through a base URL you control.',
  },
  {
    icon: 'route',
    title: 'Fallbacks that actually fire',
    body: 'Rate limited, 5xx, timed out, or a model that quietly got deprecated? The request walks your fallback list in order, across providers, and tells you what it tried.',
  },
  {
    icon: 'users',
    title: 'Orgs, teams, approvals',
    body: 'Invite a team, approve who joins, and scope every key to it. Admins see usage across the org; members only ever see their own team.',
  },
  {
    icon: 'chart',
    title: 'Usage you can actually read',
    body: 'Per-provider and per-model request counts, token spend and error rates — from the same request log that backs your audit trail, not a sampled estimate.',
  },
  {
    icon: 'logs',
    title: 'Nothing secret in the logs',
    body: 'Virtual keys, JWTs and provider keys are redacted on the way to the log sink, as a net under the rule that they should never be logged in the first place.',
  },
];

export function Features() {
  const reduce = useReducedMotion();
  const rise = fadeUp(reduce ? 0 : 24);

  return (
    <section id="features" className="relative mx-auto max-w-6xl px-5 py-20 sm:px-8 sm:py-28">
      <motion.div
        variants={staggerParent(0.08)}
        initial="hidden"
        whileInView="visible"
        viewport={inViewOnce}
        className="max-w-2xl"
      >
        <motion.p variants={rise} className="text-sm font-medium tracking-tight text-blue-400">
          Why a gateway at all
        </motion.p>
        <motion.h2
          variants={rise}
          className="mt-3 text-3xl font-bold leading-tight tracking-tight text-fg sm:text-4xl"
        >
          Everything between your app and a dozen vendor SDKs
        </motion.h2>
        <motion.p variants={rise} className="mt-4 text-base leading-relaxed text-muted">
          You already have the accounts and the keys. What you don't have is one place to route them, watch them, and
          swap them without redeploying.
        </motion.p>
      </motion.div>

      <motion.ul
        variants={staggerParent(0.08, 0.1)}
        initial="hidden"
        whileInView="visible"
        viewport={inViewOnce}
        className="mt-14 grid gap-5 sm:grid-cols-2 lg:grid-cols-3"
      >
        {FEATURES.map((feature) => (
          <motion.li
            key={feature.title}
            variants={rise}
            whileHover={reduce ? undefined : { y: -5 }}
            transition={{ type: 'spring', stiffness: 300, damping: 24 }}
            className="group relative overflow-hidden rounded-xl border border-line bg-surface/60 p-6 backdrop-blur-sm transition-colors duration-300 hover:border-blue-500/50"
          >
            <div
              aria-hidden
              className="pointer-events-none absolute -right-12 -top-16 h-40 w-40 rounded-full bg-blue-500/0 blur-3xl transition duration-500 group-hover:bg-blue-500/20"
            />
            <div className="relative">
              <span className="inline-flex h-10 w-10 items-center justify-center rounded-xl border border-line bg-raised text-blue-400 transition duration-300 group-hover:border-blue-500/40 group-hover:shadow-[0_0_16px_rgba(59,130,246,0.25)]">
                <Icon name={feature.icon} className="h-4.5 w-4.5" />
              </span>
              <h3 className="mt-5 text-base font-semibold tracking-tight text-fg">{feature.title}</h3>
              <p className="mt-2 text-sm leading-relaxed text-muted">{feature.body}</p>
            </div>
          </motion.li>
        ))}
      </motion.ul>
    </section>
  );
}
