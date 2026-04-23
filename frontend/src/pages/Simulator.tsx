import { useWallet } from '@solana/wallet-adapter-react';
import { motion, useSpring } from 'framer-motion';
import { useEffect, useMemo, useState } from 'react';
import { DEMO_MODE } from '../api';
import { HealthGauge } from '../components/HealthGauge';
import { Card, SectionLabel } from '../components/ui';
import { useHealth, useScenario } from '../hooks';
import { MOCK_HEALTH } from '../mockData';
import { fmtUsd, walletRiskToHealth } from '../utils';

export function Simulator() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;

  const healthQ = useHealth(useLive ? wallet : null);
  const scenario = useScenario();

  const base = useMemo(
    () => (useLive && healthQ.data ? walletRiskToHealth(healthQ.data) : MOCK_HEALTH),
    [useLive, healthQ.data],
  );

  const BASE_COLL = base.protocol_ltvs.reduce((s, p) => s + p.total_collateral_usd, 0);
  const BASE_BORROW = base.protocol_ltvs.reduce((s, p) => s + p.total_borrow_usd, 0);
  const BASE_LTV = BASE_COLL > 0 ? BASE_BORROW / BASE_COLL : 0;

  const [solDelta, setSolDelta] = useState(0);
  const [addColl, setAddColl] = useState(0);
  const [repayDebt, setRepayDebt] = useState(0);
  const [breached, setBreached] = useState<boolean | null>(null);

  const computeScore = (solD: number, addC: number, repayD: number): number => {
    const collChange = BASE_COLL * (solD / 100) + addC;
    const borrowChange = -repayD;
    const newCollateral = BASE_COLL + collChange;
    const newBorrow = BASE_BORROW + borrowChange;
    if (newCollateral <= 0) return 0;
    const newLtv = newBorrow / newCollateral;
    const avgThreshold = 0.75;
    const buffer = (avgThreshold - newLtv) / avgThreshold;
    return Math.max(0, Math.min(100, Math.round(base.health_score + buffer * 60)));
  };

  const simScore = computeScore(solDelta, addColl, repayDebt);
  const delta = simScore - base.health_score;

  const newCollTotal = BASE_COLL + BASE_COLL * (solDelta / 100) + addColl;
  const newBorrTotal = BASE_BORROW - repayDebt;

  const computing = scenario.isPending;

  const handleApply = () => {
    if (useLive && wallet) {
      scenario.mutate(
        {
          wallet,
          collateral_shock_pct: solDelta / 100,
          debt_shock_pct: 0,
          protocol_overrides: {},
        },
        { onSuccess: (res) => setBreached(res.breached) },
      );
    }
  };

  return (
    <div style={{ padding: '88px 28px 60px', maxWidth: 1100, margin: '0 auto' }}>
      <h2
        style={{
          fontFamily: "'Fraunces', serif",
          fontSize: 32,
          fontWeight: 600,
          color: '#F5F4EF',
          letterSpacing: '-0.02em',
          marginBottom: 6,
        }}
      >
        Scenario Simulator
      </h2>
      <p
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 14,
          color: 'rgba(245,244,239,0.4)',
          marginBottom: 32,
        }}
      >
        Stress-test your positions before markets move.
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20, alignItems: 'start' }}>
        <motion.div
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.5, ease: [0.25, 0.46, 0.45, 0.94] }}
        >
          <Card style={{ padding: '28px 28px 24px' }}>
            <SectionLabel>Adjust scenario</SectionLabel>

            <SimSlider
              label="SOL price"
              value={solDelta}
              min={-60}
              max={60}
              step={1}
              format={(v) => `${v > 0 ? '+' : ''}${v}%`}
              color={solDelta >= 0 ? '#7DA87B' : '#D9604E'}
              onChange={setSolDelta}
            />
            <SimSlider
              label="Add collateral"
              value={addColl}
              min={0}
              max={5000}
              step={50}
              format={(v) => `+${fmtUsd(v)}`}
              color="#7DA87B"
              onChange={setAddColl}
            />
            <SimSlider
              label="Repay debt"
              value={repayDebt}
              min={0}
              max={3000}
              step={50}
              format={(v) => `−${fmtUsd(v)}`}
              color="#7AA2C2"
              onChange={setRepayDebt}
            />

            <button
              onClick={handleApply}
              style={{
                marginTop: 20,
                width: '100%',
                background: '#D97757',
                border: 'none',
                cursor: 'pointer',
                padding: '13px',
                borderRadius: 100,
                fontFamily: "'Inter', sans-serif",
                fontSize: 14,
                fontWeight: 600,
                color: '#1F1E1D',
                transition: 'opacity 0.15s',
                opacity: computing ? 0.6 : 1,
              }}
            >
              {computing ? 'Computing…' : 'Apply scenario'}
            </button>

            {breached !== null && (
              <div
                style={{
                  marginTop: 12,
                  padding: '8px 14px',
                  borderRadius: 10,
                  background: breached ? 'rgba(217,96,78,0.1)' : 'rgba(125,168,123,0.1)',
                  border: `1px solid ${breached ? 'rgba(217,96,78,0.3)' : 'rgba(125,168,123,0.3)'}`,
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 12,
                  color: breached ? '#D9604E' : '#7DA87B',
                  textAlign: 'center',
                }}
              >
                Backend: {breached ? 'liquidation threshold breached' : 'position safe'}
              </div>
            )}

            <button
              onClick={() => {
                setSolDelta(0);
                setAddColl(0);
                setRepayDebt(0);
                setBreached(null);
              }}
              style={{
                marginTop: 10,
                width: '100%',
                background: 'none',
                border: '1px solid rgba(255,255,255,0.1)',
                cursor: 'pointer',
                padding: '11px',
                borderRadius: 100,
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                color: 'rgba(245,244,239,0.4)',
              }}
            >
              Reset
            </button>
          </Card>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.5, ease: [0.25, 0.46, 0.45, 0.94] }}
        >
          <Card style={{ padding: '28px 28px 24px' }}>
            <SectionLabel>Before / After</SectionLabel>

            <div
              style={{
                display: 'flex',
                justifyContent: 'space-around',
                alignItems: 'center',
                marginBottom: 28,
              }}
            >
              <div style={{ textAlign: 'center' }}>
                <HealthGauge score={base.health_score} size={140} animate={false} />
                <div
                  style={{
                    fontFamily: "'Inter', sans-serif",
                    fontSize: 12,
                    color: 'rgba(245,244,239,0.35)',
                    marginTop: 6,
                  }}
                >
                  Current
                </div>
              </div>
              <motion.div
                animate={{ x: [0, 4, 0, -4, 0], opacity: [1, 0.7, 1] }}
                transition={{ duration: 0.5, repeat: computing ? Infinity : 0 }}
                style={{ fontSize: 20, color: 'rgba(245,244,239,0.2)' }}
              >
                →
              </motion.div>
              <div style={{ textAlign: 'center' }}>
                <AnimatedGauge score={simScore} size={140} />
                <div
                  style={{
                    fontFamily: "'Inter', sans-serif",
                    fontSize: 12,
                    color: 'rgba(245,244,239,0.35)',
                    marginTop: 6,
                  }}
                >
                  Simulated
                </div>
              </div>
            </div>

            <motion.div
              key={delta}
              initial={{ scale: 0.85, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              transition={{ type: 'spring', stiffness: 400, damping: 20 }}
              style={{ textAlign: 'center', marginBottom: 20 }}
            >
              <span
                style={{
                  fontFamily: "'Fraunces', serif",
                  fontSize: 28,
                  fontWeight: 600,
                  color: delta > 0 ? '#7DA87B' : delta < 0 ? '#D9604E' : 'rgba(245,244,239,0.4)',
                }}
              >
                {delta > 0 ? `+${delta}` : delta}
              </span>
              <span
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 13,
                  color: 'rgba(245,244,239,0.4)',
                  marginLeft: 8,
                }}
              >
                health pts
              </span>
            </motion.div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {[
                {
                  label: 'Total Collateral',
                  before: fmtUsd(BASE_COLL),
                  after: fmtUsd(newCollTotal),
                  up: newCollTotal >= BASE_COLL,
                },
                {
                  label: 'Total Borrow',
                  before: fmtUsd(BASE_BORROW),
                  after: fmtUsd(newBorrTotal),
                  up: newBorrTotal <= BASE_BORROW,
                },
                {
                  label: 'Blended LTV',
                  before: `${(BASE_LTV * 100).toFixed(1)}%`,
                  after: `${((newBorrTotal / newCollTotal) * 100).toFixed(1)}%`,
                  up: newBorrTotal / newCollTotal <= BASE_LTV,
                },
                {
                  label: 'Buffer',
                  before: fmtUsd(base.liquidation_buffer_usd),
                  after: fmtUsd(
                    base.liquidation_buffer_usd + (simScore - base.health_score) * 60,
                  ),
                  up: simScore >= base.health_score,
                },
              ].map(({ label, before, after, up }) => (
                <div
                  key={label}
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    padding: '8px 0',
                    borderBottom: '1px solid rgba(255,255,255,0.05)',
                  }}
                >
                  <span
                    style={{
                      fontFamily: "'Inter', sans-serif",
                      fontSize: 12,
                      color: 'rgba(245,244,239,0.4)',
                    }}
                  >
                    {label}
                  </span>
                  <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
                    <span
                      style={{
                        fontFamily: "'JetBrains Mono', monospace",
                        fontSize: 12,
                        color: 'rgba(245,244,239,0.4)',
                      }}
                    >
                      {before}
                    </span>
                    <span style={{ color: 'rgba(245,244,239,0.2)', fontSize: 11 }}>→</span>
                    <motion.span
                      key={after}
                      initial={{ color: '#D97757' }}
                      animate={{ color: up ? '#7DA87B' : '#D9604E' }}
                      transition={{ duration: 0.8 }}
                      style={{
                        fontFamily: "'JetBrains Mono', monospace",
                        fontSize: 12,
                        fontWeight: 600,
                      }}
                    >
                      {after}
                    </motion.span>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </motion.div>
      </div>
    </div>
  );
}

function AnimatedGauge({ score, size }: { score: number; size: number }) {
  const springScore = useSpring(score, { stiffness: 80, damping: 18 });
  const [display, setDisplay] = useState(score);

  useEffect(() => {
    springScore.set(score);
    return springScore.on('change', (v) => setDisplay(Math.round(v)));
  }, [score, springScore]);

  return <HealthGauge score={display} size={size} animate={false} />;
}

function SimSlider({
  label,
  value,
  min,
  max,
  step,
  format,
  color,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  color: string;
  onChange: (v: number) => void;
}) {
  return (
    <div style={{ marginBottom: 22, position: 'relative' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 10 }}>
        <span
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 13,
            color: 'rgba(245,244,239,0.6)',
          }}
        >
          {label}
        </span>
        <motion.span
          key={value}
          initial={{ scale: 1.15, color: '#D97757' }}
          animate={{ scale: 1, color }}
          transition={{ duration: 0.3 }}
          style={{
            fontFamily: "'JetBrains Mono', monospace",
            fontSize: 13,
            fontWeight: 600,
          }}
        >
          {format(value)}
        </motion.span>
      </div>
      <div
        style={{
          position: 'relative',
          height: 4,
          borderRadius: 4,
          background: 'rgba(255,255,255,0.08)',
        }}
      >
        <motion.div
          style={{ height: '100%', borderRadius: 4, background: color, originX: 0 }}
          animate={{ width: `${((value - min) / (max - min)) * 100}%` }}
          transition={{ type: 'spring', stiffness: 200, damping: 25 }}
        />
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{
          position: 'absolute',
          width: '100%',
          opacity: 0,
          height: 20,
          cursor: 'pointer',
          marginTop: -12,
          left: 0,
        }}
      />
    </div>
  );
}
