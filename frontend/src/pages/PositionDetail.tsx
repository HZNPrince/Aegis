import { useWallet } from '@solana/wallet-adapter-react';
import { motion } from 'framer-motion';
import { useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { DEMO_MODE } from '../api';
import { Button, Card, Chip, Eyebrow, ProtocolBadge, Reveal, TemperatureBar, tokens } from '../components/sonar';
import { RepayModal } from '../components/RepayModal';
import { useHealth, useTicker } from '../hooks';
import { MOCK_HEALTH } from '../mockData';
import type { Position } from '../types';
import { fmtUsd, timeAgo, truncAddr, walletRiskToHealth } from '../utils';

/** Per-protocol detail view — replaces the old per-asset PositionDetail. */
export function ProtocolDetail() {
  const { protocolName } = useParams();
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;
  const healthQ = useHealth(useLive ? wallet : null);
  const tickerQ = useTicker();
  const [repayPosition, setRepayPosition] = useState<Position | null>(null);

  const data = useMemo(
    () => (useLive && healthQ.data ? walletRiskToHealth(healthQ.data) : MOCK_HEALTH),
    [healthQ.data, useLive],
  );

  // Find the matching protocol
  const protocol = data.protocol_ltvs.find(
    (p) => p.protocol.toLowerCase() === (protocolName ?? '').toLowerCase(),
  );
  const protocolPositions = data.positions.filter(
    (p) => p.protocol.toLowerCase() === (protocolName ?? '').toLowerCase(),
  );
  const totalCollateral = data.positions.filter((p) => p.side === 'Collateral').reduce((s, p) => s + p.value_usd, 0);
  const totalBorrow = data.positions.filter((p) => p.side === 'Borrow').reduce((s, p) => s + p.value_usd, 0);
  const avgThreshold = data.protocol_ltvs.reduce((s, p) => s + p.liquidation_threshold * p.total_collateral_usd, 0) / Math.max(totalCollateral, 1);

  if (!protocol) {
    return (
      <main style={{ padding: '80px 28px', maxWidth: 860, margin: '0 auto' }}>
        <Card pad={40}>
          <Eyebrow>Protocol not found</Eyebrow>
          <h1 style={{ fontFamily: tokens.serif, fontSize: 42, fontWeight: 500, marginTop: 10 }}>No data for "{protocolName}".</h1>
          <Link to="/positions" style={{ display: 'inline-flex', marginTop: 24, textDecoration: 'none' }}>
            <Button variant="primary">Back to positions</Button>
          </Link>
        </Card>
      </main>
    );
  }

  const ltvRatio = protocol.ltv / protocol.liquidation_threshold;
  const displayName = protocol.protocol;

  return (
    <main style={{ padding: '64px 28px 72px', maxWidth: 1100, margin: '0 auto' }}>
      <Reveal>
        <Link to="/positions" style={{ color: tokens.ink2, fontFamily: tokens.sans, fontSize: 13, textDecoration: 'none' }}>
          ← Positions
        </Link>
        <div style={{ display: 'flex', alignItems: 'start', justifyContent: 'space-between', gap: 24, marginTop: 18, marginBottom: 28 }}>
          <div>
            <Eyebrow>{displayName} Protocol</Eyebrow>
            <h1 style={{ fontFamily: tokens.serif, fontSize: 52, fontWeight: 500, letterSpacing: '-0.02em', marginTop: 8 }}>
              {displayName}{' '}
              <span style={{ fontFamily: tokens.serifI, fontStyle: 'italic', color: tokens.cobalt }}>overview</span>
            </h1>
          </div>
          <ProtocolBadge protocol={displayName} size={48} />
        </div>
      </Reveal>

      {/* ─── Stats grid ─── */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 14, marginBottom: 24 }}>
        <StatCard label="Total Collateral" value={fmtUsd(protocol.total_collateral_usd)} />
        <StatCard label="Total Borrowed" value={fmtUsd(protocol.total_borrow_usd)} />
        <StatCard label="Current LTV" value={`${(protocol.ltv * 100).toFixed(1)}%`} tone={ltvRatio > 0.9 ? 'critical' : ltvRatio > 0.75 ? 'watch' : 'healthy'} />
        <StatCard label="Liquidation Threshold" value={`${(protocol.liquidation_threshold * 100).toFixed(0)}%`} />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 0.8fr', gap: 20 }}>
        {/* ─── 30-day collateral/borrow chart ─── */}
        <Card pad={24}>
          <Eyebrow>Collateral vs Borrow · 30d</Eyebrow>
          <CollateralBorrowChart
            collateral={protocol.total_collateral_usd}
            borrow={protocol.total_borrow_usd}
          />
        </Card>

        {/* ─── Temperature bar & risk ─── */}
        <Card pad={24}>
          <Eyebrow>Risk Temperature</Eyebrow>
          <div style={{ marginTop: 28, marginBottom: 20 }}>
            <TemperatureBar
              value={ltvRatio}
              threshold={1.0}
              height={12}
            />
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontFamily: tokens.mono, fontSize: 11, color: 'color-mix(in oklab, var(--ink) 55%, transparent)' }}>
            <span>Safe</span>
            <span style={{ fontWeight: 600, color: ltvRatio > 0.85 ? 'var(--rust)' : ltvRatio > 0.6 ? '#c69423' : 'var(--moss)' }}>
              {(ltvRatio * 100).toFixed(1)}% of threshold
            </span>
            <span>Danger</span>
          </div>

          <div style={{ marginTop: 28, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
            <MetricCard label="Position Legs" value={String(protocolPositions.length)} />
            <MetricCard label="Buffer" value={fmtUsd(protocol.total_collateral_usd * protocol.liquidation_threshold - protocol.total_borrow_usd)} />
          </div>
        </Card>
      </div>

      {/* ─── Position legs table ─── */}
      <Card pad={0} style={{ overflow: 'hidden', marginTop: 22 }}>
        <div style={{ padding: '18px 20px', borderBottom: `1px solid ${tokens.lineSoft}` }}>
          <Eyebrow>Position Legs</Eyebrow>
        </div>
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', minWidth: 640 }}>
            <thead>
              <tr>
                {['Asset', 'Side', 'Amount', 'Value', 'Price', 'Updated', ''].map((h) => (
                  <th key={h} style={{ textAlign: 'left', padding: '13px 16px', borderBottom: `1px solid ${tokens.lineSoft}` }}>
                    <Eyebrow>{h}</Eyebrow>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {protocolPositions.map((pos) => {
                const tick = pos.asset_mint ? tickerQ.data?.[pos.asset_mint] : undefined;
                return (
                  <tr key={pos.id} style={{ borderBottom: `1px solid ${tokens.lineSoft}` }}>
                    <td style={{ padding: '15px 16px' }}>
                      <div style={{ fontFamily: tokens.mono, fontSize: 14, fontWeight: 600 }}>{pos.asset_symbol}</div>
                      <div style={{ fontFamily: tokens.mono, fontSize: 11, color: 'color-mix(in oklab, var(--ink) 48%, transparent)', marginTop: 2 }}>
                        {truncAddr(pos.asset_mint || pos.obligation_address)}
                      </div>
                    </td>
                    <td style={{ padding: '15px 16px' }}>
                      <Chip tone={pos.side === 'Borrow' ? 'watch' : 'healthy'}>{pos.side}</Chip>
                    </td>
                    <td style={{ padding: '15px 16px', fontFamily: tokens.mono }}>{pos.amount.toLocaleString('en-US', { maximumFractionDigits: 4 })}</td>
                    <td style={{ padding: '15px 16px', fontFamily: tokens.mono, fontWeight: 600 }}>{fmtUsd(pos.value_usd)}</td>
                    <td style={{ padding: '15px 16px', fontFamily: tokens.mono }}>
                      {tick ? `$${tick.price.toLocaleString('en-US', { maximumFractionDigits: 4 })}` : '—'}
                    </td>
                    <td style={{ padding: '15px 16px', color: tokens.ink2, fontSize: 13 }}>{timeAgo(pos.updated_at)}</td>
                    <td style={{ padding: '15px 16px', textAlign: 'right' }}>
                      {pos.side === 'Borrow' && pos.reserve_or_bank && useLive ? (
                        <Button size="sm" variant="danger" onClick={() => setRepayPosition(pos)}>Repay</Button>
                      ) : null}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Card>

      {repayPosition && (
        <RepayModal
          position={repayPosition}
          totalDebtUsd={totalBorrow}
          totalCollateralUsd={totalCollateral}
          liquidationThreshold={avgThreshold || 0.85}
          onClose={() => setRepayPosition(null)}
        />
      )}
    </main>
  );
}

/** 30-day collateral vs borrow area chart with two overlapping areas */
function CollateralBorrowChart({ collateral, borrow }: { collateral: number; borrow: number }) {
  // Generate mock 30 day data with slight variance
  const days = 30;
  const collateralData: number[] = [];
  const borrowData: number[] = [];
  for (let i = 0; i < days; i++) {
    const t = i / (days - 1);
    const noise1 = Math.sin(i * 0.8) * 0.08 + Math.sin(i * 1.7) * 0.05;
    const noise2 = Math.sin(i * 1.2 + 1) * 0.06 + Math.sin(i * 0.5) * 0.04;
    collateralData.push(collateral * (0.85 + t * 0.15 + noise1));
    borrowData.push(borrow * (0.92 + t * 0.08 + noise2));
  }

  const maxVal = Math.max(...collateralData, ...borrowData) * 1.1;
  const w = 400;
  const h = 160;
  const pad = { t: 10, b: 24, l: 0, r: 0 };
  const cw = w - pad.l - pad.r;
  const ch = h - pad.t - pad.b;

  const toPath = (data: number[]) => {
    return data.map((v, i) => {
      const x = pad.l + (i / (days - 1)) * cw;
      const y = pad.t + ch - (v / maxVal) * ch;
      return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${y.toFixed(1)}`;
    }).join(' ');
  };

  const toArea = (data: number[]) => {
    const linePath = toPath(data);
    const lastX = pad.l + cw;
    const firstX = pad.l;
    return `${linePath} L ${lastX} ${pad.t + ch} L ${firstX} ${pad.t + ch} Z`;
  };

  const collateralPath = toPath(collateralData);
  const borrowPath = toPath(borrowData);
  const collateralArea = toArea(collateralData);
  const borrowArea = toArea(borrowData);

  // End points for pulsing dots
  const cEndX = pad.l + cw;
  const cEndY = pad.t + ch - (collateralData[days - 1] / maxVal) * ch;
  const bEndX = pad.l + cw;
  const bEndY = pad.t + ch - (borrowData[days - 1] / maxVal) * ch;

  return (
    <div style={{ marginTop: 16 }}>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} fill="none" style={{ display: 'block' }}>
        <defs>
          <linearGradient id="collGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--moss)" stopOpacity="0.25" />
            <stop offset="100%" stopColor="var(--moss)" stopOpacity="0.03" />
          </linearGradient>
          <linearGradient id="borrowGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--rust)" stopOpacity="0.2" />
            <stop offset="100%" stopColor="var(--rust)" stopOpacity="0.02" />
          </linearGradient>
        </defs>

        {/* Grid lines */}
        {[0.25, 0.5, 0.75].map((frac) => (
          <line
            key={frac}
            x1={pad.l}
            y1={pad.t + ch * (1 - frac)}
            x2={pad.l + cw}
            y2={pad.t + ch * (1 - frac)}
            stroke="var(--ink)"
            strokeWidth="0.5"
            opacity="0.06"
          />
        ))}

        {/* Collateral area + line */}
        <motion.path
          d={collateralArea}
          fill="url(#collGrad)"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.6 }}
        />
        <motion.path
          d={collateralPath}
          stroke="var(--moss)"
          strokeWidth="2"
          strokeLinecap="round"
          fill="none"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 1.0, ease: 'easeOut' }}
        />

        {/* Borrow area + line */}
        <motion.path
          d={borrowArea}
          fill="url(#borrowGrad)"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.6, delay: 0.2 }}
        />
        <motion.path
          d={borrowPath}
          stroke="var(--rust)"
          strokeWidth="2"
          strokeLinecap="round"
          fill="none"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 1.0, ease: 'easeOut', delay: 0.2 }}
        />

        {/* Pulsing dot on collateral head */}
        <motion.circle
          cx={cEndX} cy={cEndY} r="5"
          fill="none" stroke="var(--moss)" strokeWidth="1.5"
          initial={{ r: 3, opacity: 0.6 }}
          animate={{ r: 10, opacity: 0 }}
          transition={{ duration: 1.5, repeat: Infinity, ease: 'easeOut' }}
        />
        <circle cx={cEndX} cy={cEndY} r="3" fill="var(--moss)" />

        {/* Pulsing dot on borrow head */}
        <motion.circle
          cx={bEndX} cy={bEndY} r="5"
          fill="none" stroke="var(--rust)" strokeWidth="1.5"
          initial={{ r: 3, opacity: 0.6 }}
          animate={{ r: 10, opacity: 0 }}
          transition={{ duration: 1.5, repeat: Infinity, ease: 'easeOut' }}
        />
        <circle cx={bEndX} cy={bEndY} r="3" fill="var(--rust)" />

        {/* X-axis labels */}
        <text x={pad.l + 4} y={h - 4} fill="var(--ink)" opacity="0.35" fontSize="9" fontFamily="var(--mono)">30d ago</text>
        <text x={pad.l + cw - 28} y={h - 4} fill="var(--ink)" opacity="0.35" fontSize="9" fontFamily="var(--mono)">Today</text>
      </svg>

      {/* Legend */}
      <div style={{ display: 'flex', gap: 20, marginTop: 8 }}>
        <LegendDot color="var(--moss)" label="Collateral" />
        <LegendDot color="var(--rust)" label="Borrowed" />
      </div>
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontFamily: tokens.mono, fontSize: 11, color: 'color-mix(in oklab, var(--ink) 55%, transparent)' }}>
      <span style={{ width: 8, height: 8, borderRadius: '50%', background: color, display: 'inline-block' }} />
      {label}
    </div>
  );
}

function StatCard({ label, value, tone }: { label: string; value: string; tone?: 'healthy' | 'watch' | 'critical' }) {
  const borderColor = tone === 'critical' ? 'rgba(196,69,54,0.3)' : tone === 'watch' ? 'rgba(198,148,35,0.3)' : tokens.line;
  return (
    <Card pad={18} style={{ borderColor }}>
      <Eyebrow>{label}</Eyebrow>
      <div style={{ fontFamily: tokens.mono, fontSize: 22, fontWeight: 700, marginTop: 8 }}>{value}</div>
    </Card>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ border: `1px solid ${tokens.lineSoft}`, padding: 14, borderRadius: 6 }}>
      <Eyebrow>{label}</Eyebrow>
      <div style={{ fontFamily: tokens.mono, fontSize: 16, fontWeight: 600, marginTop: 6 }}>{value}</div>
    </div>
  );
}
