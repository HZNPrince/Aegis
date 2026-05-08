import { useWallet } from '@solana/wallet-adapter-react';
import { AnimatePresence, motion } from 'framer-motion';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Button,
  Card,
  CountUp,
  Eyebrow,
  PriceTicker,
  Reveal,
  tokens,
} from '../components/sonar';
import { SonarLogo } from '../components/SonarLogo';
import { useStatus } from '../hooks';

interface Props {
  onConnect: () => void;
}

const PROTOCOLS = ['Kamino', 'Save', 'MarginFi'];


const STEPS = [
  {
    n: '01',
    title: 'Connect',
    body: 'Sign in with your Solana wallet. Aegis backfills your open positions across Kamino, Save, and MarginFi in seconds.',
  },
  {
    n: '02',
    title: 'Watch',
    body: 'A single weighted health score across protocols. Per-position arc gauges, live LTV, liquidation buffer in dollars.',
  },
  {
    n: '03',
    title: 'Defend',
    body: 'Set guardrails — health below 1.2, repay 30%. Get a Telegram alert with inline buttons before liquidation. One-tap repay.',
  },
];

const FEATURES = [
  {
    title: 'Unified watchtower',
    body: 'One dashboard. Three protocols. Cross-protocol weighted health so you stop juggling tabs.',
  },
  {
    title: 'AI alerts that earn the ping',
    body: 'LLM summarizes the why — collateral price drop, oracle drift, rate spike — not just the number.',
  },
  {
    title: 'Guardrails that act',
    body: 'Pre-built repay, deleverage, and add-collateral rules. Triggered on-chain via your wallet, with cooldowns and daily caps.',
  },
  {
    title: 'Telegram, with intent',
    body: 'Critical alerts ship with inline Repay / Mute buttons. Anywhere you have a phone, you have your defense.',
  },
];

/* ── Random radar pulse messages ── */
const PULSE_MESSAGES = [
  { text: '● Kamino LTV approaching 72%', tone: 'warn' as const },
  { text: '● SOL price moved −4.2% / 1h', tone: 'critical' as const },
  { text: '● Save oracle lag 45s detected', tone: 'warn' as const },
  { text: '● Marginfi health recovered to 1.8', tone: 'info' as const },
  { text: '● Guard rule fired: Repay 500 USDC', tone: 'critical' as const },
  { text: '● JitoSOL collateral value +3.1%', tone: 'info' as const },
  { text: '● New position detected on Kamino', tone: 'info' as const },
  { text: '● Weighted health dropped below 1.2', tone: 'critical' as const },
];

export function Landing({ onConnect }: Props) {
  const { connecting } = useWallet();
  const statusQ = useStatus();
  // Show live indexer-backed numbers; fall back to 0 while the request is in
  // flight (CountUp animates up from 0 anyway, so no flicker).
  const stats: { k: string; v: number; suf?: string; pre?: string }[] = [
    { k: 'Protocols watched', v: 3 },
    { k: 'Wallets monitored', v: statusQ.data?.wallets_monitored ?? 0 },
    { k: 'Positions tracked', v: statusQ.data?.positions_cached ?? 0 },
  ];

  return (
    <div style={{ background: tokens.paper, color: tokens.ink, overflow: 'hidden' }}>
      {/* ─── Hero ─── */}
      <section
        style={{
          position: 'relative',
          minHeight: '100dvh',
          padding: '90px clamp(20px, 3.5vw, 56px) 280px',
          maxWidth: 1320,
          margin: '0 auto',
          width: '100%',
        }}
      >
        <InteractiveSonar />

        <div style={{ position: 'relative', zIndex: 1, maxWidth: 580, textAlign: 'left' }}>
          {/* AEGIS wordmark — extreme top-left anchor */}
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1, duration: 0.8, ease: [0.2, 0.8, 0.2, 1] }}
            style={{
              fontFamily: tokens.sans,
              fontWeight: 800,
              fontSize: 'clamp(64px, 10vw, 132px)',
              lineHeight: 0.9,
              letterSpacing: '-0.04em',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'flex-start',
              gap: '0.08em',
              color: tokens.ink,
            }}
          >
            <span>A</span>
            <span>E</span>
            <span style={{ position: 'relative', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>
              G
              <span
                style={{
                  position: 'absolute',
                  width: '0.18em',
                  height: '0.18em',
                  background: tokens.rust,
                  borderRadius: '50%',
                  boxShadow: `0 0 24px var(--rust)`,
                }}
              />
            </span>
            <span>I</span>
            <span>S</span>
          </motion.div>

          {/* Type-animated body — same grey/sans as before, slightly bolder */}
          <Typewriter
            text="A unified watchtower for your Solana lending positions — with guardrails and an AI sentinel that only speaks when it matters."
            delay={0.4}
            speed={20}
            style={{
              fontFamily: tokens.sans,
              fontWeight: 600,
              fontSize: 17,
              color: 'color-mix(in oklab, var(--ink) 50%, transparent)',
              lineHeight: 1.6,
              marginTop: 48,
              maxWidth: 480,
              textAlign: 'left',
              minHeight: 96,
            }}
          />

          {/* Single CTA — wallet connect lives in the nav */}
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 3.6, duration: 0.5 }}
            style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 18, marginTop: 24 }}
          >
            <span style={{ width: 40, height: 1, background: 'color-mix(in oklab, var(--ink) 22%, transparent)' }} />
            <Button variant="ghost" size="lg" onClick={() => {
              document.getElementById('how')?.scrollIntoView({ behavior: 'smooth' });
            }}>
              How it works ↓
            </Button>
          </motion.div>
        </div>

        {/* Protocol strip — bottom of hero */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 1.2, duration: 0.5 }}
          style={{
            position: 'absolute',
            bottom: 40,
            display: 'flex',
            alignItems: 'center',
            gap: 18,
            fontFamily: tokens.mono,
            fontSize: 10,
            letterSpacing: '0.18em',
            textTransform: 'uppercase',
            color: 'color-mix(in oklab, var(--ink) 38%, transparent)',
          }}
        >
          <span>Tracking</span>
          {PROTOCOLS.map((p, i) => (
            <span key={p} style={{ display: 'inline-flex', alignItems: 'center', gap: 18 }}>
              {i > 0 && <span style={{ width: 1, height: 10, background: 'color-mix(in oklab, var(--ink) 12%, transparent)' }} />}
              {p}
            </span>
          ))}
        </motion.div>
      </section>

      {/* ─── Price Ticker ─── */}
      <PriceTicker />

      {/* ─── Stats strip ─── */}
      <section style={{ borderTop: `1px solid ${tokens.line}`, borderBottom: `1px solid ${tokens.line}`, background: tokens.paper2 }}>
        <div style={{ maxWidth: 1180, margin: '0 auto', display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }}>
          {stats.map((s, i) => (
            <Reveal key={s.k} delay={i * 0.08}>
              <div style={{ padding: '32px 36px', borderLeft: i > 0 ? `1px solid ${tokens.line}` : 'none' }}>
                <Eyebrow>{s.k}</Eyebrow>
                <div
                  style={{
                    fontFamily: tokens.serif,
                    fontWeight: 500,
                    fontSize: 44,
                    letterSpacing: '-0.01em',
                    marginTop: 8,
                  }}
                >
                  <CountUp to={s.v} prefix={s.pre ?? ''} suffix={s.suf ?? ''} />
                </div>
              </div>
            </Reveal>
          ))}
        </div>
      </section>

      {/* ─── How it works ─── */}
      <section id="how" style={{ padding: '100px 32px', borderTop: `1px solid ${tokens.line}` }}>
        <div style={{ maxWidth: 1180, margin: '0 auto' }}>
          <Reveal>
            <div style={{ textAlign: 'center', marginBottom: 56 }}>
              <Eyebrow>How it works</Eyebrow>
              <h2
                style={{
                  fontFamily: tokens.sans,
                  fontWeight: 750,
                  fontSize: 44,
                  letterSpacing: '-0.035em',
                  lineHeight: 1.1,
                  marginTop: 14,
                }}
              >
                Three steps from{' '}
                <span style={{ color: tokens.rust }}>exposed</span>{' '}
                to defended.
              </h2>
            </div>
          </Reveal>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 24 }}>
            {STEPS.map((s, i) => (
              <Reveal key={s.n} delay={i * 0.1}>
                <Card pad={28} style={{ height: '100%' }}>
                  <div style={{ fontFamily: tokens.mono, fontSize: 11, letterSpacing: '0.16em', color: tokens.rust }}>
                    {s.n}
                  </div>
                  <div style={{ fontFamily: tokens.sans, fontWeight: 700, fontSize: 22, letterSpacing: '-0.02em', marginTop: 16 }}>
                    {s.title}
                  </div>
                  <div style={{ fontFamily: tokens.sans, fontSize: 14, color: tokens.ink2, lineHeight: 1.6, marginTop: 10 }}>
                    {s.body}
                  </div>
                </Card>
              </Reveal>
            ))}
          </div>
        </div>
      </section>

      {/* ─── Features ─── */}
      <section style={{ padding: '100px 32px', borderTop: `1px solid ${tokens.line}`, background: tokens.paper2 }}>
        <div style={{ maxWidth: 1180, margin: '0 auto' }}>
          <Reveal>
            <div style={{ textAlign: 'center', marginBottom: 48 }}>
              <Eyebrow>What you get</Eyebrow>
              <h2 style={{ fontFamily: tokens.sans, fontWeight: 750, fontSize: 40, letterSpacing: '-0.035em', marginTop: 14 }}>
                Built for the moment{' '}
                <span style={{ color: tokens.rust }}>before</span>{' '}
                the trouble.
              </h2>
            </div>
          </Reveal>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 0, border: `1px solid ${tokens.line}`, background: tokens.paper }}>
            {FEATURES.map((f, i) => (
              <Reveal key={f.title} delay={i * 0.06}>
                <div
                  style={{
                    padding: '36px 32px',
                    borderRight: i % 2 === 0 ? `1px solid ${tokens.line}` : 'none',
                    borderTop: i >= 2 ? `1px solid ${tokens.line}` : 'none',
                    minHeight: 180,
                  }}
                >
                  <div style={{ fontFamily: tokens.sans, fontWeight: 700, fontSize: 22, letterSpacing: '-0.02em' }}>
                    {f.title}
                  </div>
                  <div style={{ fontFamily: tokens.sans, fontSize: 14.5, color: tokens.ink2, lineHeight: 1.6, marginTop: 12, maxWidth: 440 }}>
                    {f.body}
                  </div>
                </div>
              </Reveal>
            ))}
          </div>
        </div>
      </section>

      {/* ─── Telegram bot preview ─── */}
      <section style={{ padding: '100px 32px', position: 'relative' }}>
        <div style={{ maxWidth: 900, margin: '0 auto' }}>
          <Reveal>
            <div style={{ textAlign: 'center', marginBottom: 56 }}>
              <Eyebrow>Telegram integration</Eyebrow>
              <h2
                style={{
                  fontFamily: tokens.sans,
                  fontWeight: 750,
                  fontSize: 40,
                  letterSpacing: '-0.035em',
                  lineHeight: 1.1,
                  marginTop: 14,
                }}
              >
                Alerts that reach you{' '}
                <span style={{ color: tokens.cobalt }}>anywhere.</span>
              </h2>
            </div>
          </Reveal>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 24 }}>
            {/* Bot preview */}
            <Reveal delay={0.08}>
              <Card pad={24} style={{ background: 'var(--surface-1)' }}>
                <Eyebrow>Preview · @AegisBot</Eyebrow>
                <div style={{ marginTop: 18, display: 'grid', gap: 14 }}>
                  <TgAlert />
                  <Bubble align="right">/health</Bubble>
                  <Bubble>Weighted health: 1.42{'\n'}4 positions · 2 watch · 1 critical</Bubble>
                  <Bubble align="right">/repay p3 1200</Bubble>
                  <div style={{ border: `1px solid ${tokens.lineSoft}`, borderRadius: 12, padding: 14, maxWidth: 340, background: 'var(--surface-1)' }}>
                    <div style={{ fontFamily: tokens.sans, fontSize: 15 }}>Repay $1,200 on MarginFi mSOL/USDT?</div>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginTop: 12 }}>
                      <Button>Confirm</Button>
                      <Button>Cancel</Button>
                    </div>
                  </div>
                </div>
              </Card>
            </Reveal>

            {/* Commands list */}
            <Reveal delay={0.16}>
              <Card pad={0} style={{ background: 'var(--surface-1)', overflow: 'hidden' }}>
                <div style={{ padding: '20px 22px', borderBottom: `1px solid ${tokens.lineSoft}` }}>
                  <Eyebrow>Bot commands</Eyebrow>
                </div>
                {[
                  ['/positions', 'List tracked positions with health and LTV.'],
                  ['/health', 'Return weighted portfolio health.'],
                  ['/repay <pos> <amt>', 'Create a repay confirmation.'],
                  ['/set_guardrail', 'Configure auto-repay rules.'],
                  ['/alerts', 'Show your last 10 alerts.'],
                  ['/mute <hours>', 'Pause non-critical alerts.'],
                ].map(([cmd, desc]) => (
                  <div key={cmd} style={{ display: 'grid', gridTemplateColumns: '180px 1fr', gap: 12, padding: '14px 22px', borderBottom: `1px solid ${tokens.lineSoft}` }}>
                    <code style={{ fontFamily: tokens.mono, fontSize: 13 }}>{cmd}</code>
                    <span style={{ fontFamily: tokens.sans, fontSize: 13, color: tokens.ink2, lineHeight: 1.45 }}>{desc}</span>
                  </div>
                ))}
              </Card>
            </Reveal>
          </div>
        </div>
      </section>

      {/* ─── Closing CTA ─── */}
      <section style={{ padding: '120px 32px 140px', textAlign: 'center', borderTop: `1px solid ${tokens.line}` }}>
        <Reveal>
          <SonarLogo size={140} animate />
        </Reveal>
        <Reveal delay={0.05}>
          <h2 style={{ fontFamily: tokens.sans, fontWeight: 800, fontSize: 'clamp(32px, 5vw, 56px)', letterSpacing: '-0.02em', lineHeight: 1.1, maxWidth: 720, margin: '28px auto 0' }}>
            One dashboard.{' '}
            <span style={{ color: tokens.rust }}>Multiple venues.</span>{' '}
            Zero surprises.
          </h2>
        </Reveal>
        <Reveal delay={0.1}>
          <div style={{ marginTop: 36 }}>
            <Button variant="primary" size="lg" onClick={onConnect} disabled={connecting}>
              {connecting ? 'Connecting…' : 'Connect wallet'}
            </Button>
          </div>
        </Reveal>
      </section>

      <footer style={{ borderTop: `1px solid ${tokens.line}`, padding: '24px 32px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <SonarLogo size={22} />
          <span style={{ fontFamily: tokens.sans, fontWeight: 700 }}>Aegis</span>
        </div>
        <div className="eyebrow">© 2026 · Solana mainnet · v0.1</div>
      </footer>
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────────────────
   Interactive Sonar Background
   - Static faint arcs in the bottom-right
   - Random pulses spawn on arc lines
   - Hovering a pulse shows an alert message popup with fade/pop animation
   ───────────────────────────────────────────────────────────────────────────── */

interface PulsePoint {
  id: number;
  cx: number;
  cy: number;
  msg: string;
  tone: 'info' | 'warn' | 'critical';
}

const SWEEPS = [
  { r: 220, dur: 8,  delay: 0, len: 45, color: 'var(--cobalt)', glow: 'var(--cobalt)' },
  { r: 420, dur: 12, delay: 2, len: 60, color: 'var(--moss)',   glow: 'var(--moss)' },
  { r: 520, dur: 16, delay: 5, len: 75, color: '#c69423',       glow: '#c69423' },
  { r: 720, dur: 22, delay: 7, len: 90, color: 'var(--rust)',   glow: 'var(--rust)' },
];

/** Typewriter — characters appear left-to-right with a blinking caret. */
function Typewriter({ text, delay = 0, speed = 24, style }: { text: string; delay?: number; speed?: number; style?: React.CSSProperties }) {
  const [i, setI] = useState(0);
  const [started, setStarted] = useState(false);

  useEffect(() => {
    const startTimer = window.setTimeout(() => setStarted(true), delay * 1000);
    return () => clearTimeout(startTimer);
  }, [delay]);

  useEffect(() => {
    if (!started) return;
    if (i >= text.length) return;
    const t = window.setTimeout(() => setI((n) => n + 1), speed);
    return () => clearTimeout(t);
  }, [started, i, text, speed]);

  const done = i >= text.length;
  return (
    <p style={{ margin: 0, ...style }} aria-label={text}>
      {text.slice(0, i)}
      <motion.span
        animate={{ opacity: done ? [1, 0, 1] : 1 }}
        transition={done ? { duration: 1.1, repeat: Infinity, ease: 'easeInOut' } : { duration: 0 }}
        style={{
          display: 'inline-block',
          width: '0.5ch',
          marginLeft: 2,
          height: '1em',
          verticalAlign: '-0.15em',
          background: 'var(--cobalt)',
          opacity: 0.85,
        }}
      />
    </p>
  );
}

function InteractiveSonar() {
  const [pulses, setPulses] = useState<PulsePoint[]>([]);
  const nextId = useRef(0);
  const startTime = useRef(Date.now());

  // Center bottom-right
  const cx = 1100;
  const cy = 1000;
  // Extra radius added as requested
  const radii = [120, 220, 320, 420, 520, 620, 720, 820];

  const spawnPulse = useCallback(() => {
    // Pick a random sweep
    const sweep = SWEEPS[Math.floor(Math.random() * SWEEPS.length)];
    // Calculate current rotation angle
    const t = (Date.now() - startTime.current) / 1000;
    const t_active = Math.max(0, t - sweep.delay);
    // Sweeps only go from 180 to 360 degrees
    const angleDeg = 180 + (((t_active / sweep.dur) * 180) % 180);
    const angleRad = angleDeg * (Math.PI / 180);
    
    // Pulse spawns exactly at the rotating head
    const px = cx + sweep.r * Math.cos(angleRad);
    const py = cy + sweep.r * Math.sin(angleRad);

    const msg = PULSE_MESSAGES[Math.floor(Math.random() * PULSE_MESSAGES.length)];
    const id = nextId.current++;
    setPulses((prev) => [...prev.slice(-6), { id, cx: px, cy: py, msg: msg.text, tone: msg.tone }]);
    
    // Auto-remove after 4s
    setTimeout(() => setPulses((prev) => prev.filter((p) => p.id !== id)), 4000);
  }, []);

  useEffect(() => {
    // Spawn a pulse every 1.5-3s
    const spawn = () => {
      spawnPulse();
      // 20% slower than the prior 1150–2300ms window
      const delay = 1440 + Math.random() * 1440;
      timer = window.setTimeout(spawn, delay);
    };
    let timer = window.setTimeout(spawn, 750);
    return () => clearTimeout(timer);
  }, [spawnPulse]);

  const toneColors = useMemo(() => ({
    info: { dot: 'var(--cobalt)', bg: 'rgba(59,91,219,0.08)', border: 'rgba(59,91,219,0.2)', text: 'var(--cobalt)' },
    warn: { dot: '#c69423', bg: 'rgba(198,148,35,0.08)', border: 'rgba(198,148,35,0.22)', text: '#8a6a1f' },
    critical: { dot: 'var(--rust)', bg: 'rgba(196,69,54,0.08)', border: 'rgba(196,69,54,0.22)', text: 'var(--rust)' },
  }), []);

  return (
    <div
      style={{
        position: 'absolute',
        bottom: 30, // 10px visually below the Tracking text at bottom: 40
        right: 0,
        width: 1400,
        height: 1000,
        pointerEvents: 'none',
        zIndex: 0,
        overflow: 'hidden',
      }}
    >
      <svg
        width="100%"
        height="100%"
        viewBox="0 0 1400 1000"
        xmlns="http://www.w3.org/2000/svg"
        style={{ position: 'absolute', inset: 0, overflow: 'visible' }}
      >
        <defs>
          {/* Clip everything strictly above (and on) the horizon — keeps the sweep
              from dipping into the bottom half. */}
          <clipPath id="sonar-upper-half">
            <rect x={cx - 1500} y={cy - 1500} width={3000} height={1500} />
          </clipPath>
        </defs>
        {/* Horizon line through center */}
        <line x1={cx - 1000} y1={cy} x2={cx + 300} y2={cy} stroke="var(--ink)" strokeWidth="1" opacity="0.14" />

        {/* Static arcs */}
        {radii.map((r) => (
          <path
            key={r}
            d={`M ${cx - r} ${cy} A ${r} ${r} 0 0 1 ${cx + r} ${cy}`}
            stroke="var(--ink)"
            strokeWidth="1.25"
            fill="none"
            opacity="0.14"
          />
        ))}

        {/* Center dot */}
        <circle cx={cx} cy={cy} r="5" fill="var(--rust)" opacity="0.8" />
        <circle cx={cx} cy={cy} r="8" stroke="var(--rust)" strokeWidth="1" fill="none" opacity="0.3" />

        {/* Animated circumference sweeps — SMIL animateTransform so the rotation
            happens entirely in SVG user space and shares a single sub-pixel
            rounding path with the static arcs. */}
        {SWEEPS.map((sweep, i) => {
          const c = 2 * Math.PI * sweep.r;
          const arcLen = (sweep.len / 360) * c;
          return (
            <g key={`sweep-${i}`} clipPath="url(#sonar-upper-half)">
              <g>
                <circle
                  cx={cx}
                  cy={cy}
                  r={sweep.r}
                  stroke={sweep.color}
                  strokeWidth="1.25"
                  fill="none"
                  strokeDasharray={`${arcLen} ${c - arcLen}`}
                  strokeLinecap="butt"
                  opacity="0.55"
                  transform={`rotate(-${sweep.len} ${cx} ${cy})`}
                />
                <circle
                  cx={cx + sweep.r}
                  cy={cy}
                  r="4"
                  fill={sweep.color}
                  style={{ filter: `drop-shadow(0 0 6px ${sweep.glow})` }}
                />
                <animateTransform
                  attributeName="transform"
                  type="rotate"
                  from={`180 ${cx} ${cy}`}
                  to={`360 ${cx} ${cy}`}
                  dur={`${sweep.dur}s`}
                  begin={`${sweep.delay}s`}
                  repeatCount="indefinite"
                />
              </g>
            </g>
          );
        })}

        {/* Pulse dots & alerts — Rendered inside SVG for perfect coordinate alignment */}
        <AnimatePresence>
          {pulses.map((p) => {
            const tc = toneColors[p.tone];
            return (
              <motion.g
                key={p.id}
                initial={{ scale: 0, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                exit={{ scale: 0, opacity: 0 }}
                transition={{ duration: 0.35, ease: [0.2, 0.8, 0.2, 1] }}
                style={{ transformOrigin: `${p.cx}px ${p.cy}px` }}
              >
                {/* Pulsing ring */}
                <motion.circle
                  cx={p.cx}
                  cy={p.cy}
                  r={5}
                  fill="none"
                  stroke={tc.dot}
                  strokeWidth={1.5}
                  animate={{ r: [5, 12], opacity: [0.6, 0] }}
                  transition={{ duration: 1.5, repeat: Infinity, ease: 'easeOut' }}
                />
                {/* Solid dot */}
                <circle cx={p.cx} cy={p.cy} r={4.5} fill={tc.dot} style={{ filter: `drop-shadow(0 0 4px ${tc.dot})` }} />

                {/* Tooltip — always shown, using foreignObject to embed HTML */}
                <foreignObject x={p.cx - 150} y={p.cy - 60} width="300" height="60" style={{ pointerEvents: 'none' }}>
                  <motion.div
                    initial={{ opacity: 0, scale: 0.85, y: 4 }}
                    animate={{ opacity: 1, scale: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.85, y: 4 }}
                    transition={{ duration: 0.2, delay: 0.1 }}
                    style={{
                      display: 'flex',
                      justifyContent: 'center',
                      alignItems: 'center',
                      width: '100%',
                      height: '100%',
                    }}
                  >
                    <div
                      style={{
                        background: tc.bg,
                        border: `1px solid ${tc.border}`,
                        backdropFilter: 'blur(12px)',
                        borderRadius: 8,
                        padding: '6px 10px',
                        whiteSpace: 'nowrap',
                        fontFamily: tokens.mono,
                        fontSize: 10,
                        color: tc.text,
                        boxShadow: '0 4px 14px rgba(0,0,0,0.06)',
                      }}
                    >
                      {p.msg}
                    </div>
                  </motion.div>
                </foreignObject>
              </motion.g>
            );
          })}
        </AnimatePresence>
      </svg>
    </div>
  );
}

/* ─── Telegram preview helpers ─── */

function TgAlert() {
  return (
    <div style={{ border: '1px solid rgba(196,69,54,0.28)', borderRadius: 12, padding: 14, background: 'rgba(196,69,54,0.04)' }}>
      <div style={{ fontFamily: tokens.sans, color: tokens.rust, fontWeight: 600, fontSize: 13 }}>
        ● CRITICAL · MarginFi mSOL/USDT
      </div>
      <div style={{ fontFamily: tokens.sans, fontSize: 14, lineHeight: 1.4, marginTop: 8 }}>
        Health 1.04. Liq $154.20, current $161.80 (-6.8% / 24h).
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 6, marginTop: 12 }}>
        <Button>Repay 50%</Button>
        <Button>Custom</Button>
        <Button>Mute 1h</Button>
      </div>
    </div>
  );
}

function Bubble({ children, align = 'left' }: { children: React.ReactNode; align?: 'left' | 'right' }) {
  const isRight = align === 'right';
  return (
    <div
      style={{
        justifySelf: isRight ? 'end' : 'start',
        maxWidth: 300,
        padding: '10px 14px',
        background: isRight ? tokens.ink : tokens.paper2,
        color: isRight ? tokens.paper : tokens.ink,
        border: `1px solid ${isRight ? tokens.ink : tokens.line}`,
        borderRadius: 12,
        fontFamily: tokens.mono,
        fontSize: 12,
        lineHeight: 1.4,
        whiteSpace: 'pre-line',
      }}
    >
      {children}
    </div>
  );
}
