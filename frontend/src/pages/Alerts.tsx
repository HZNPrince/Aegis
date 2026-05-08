import { useWallet } from '@solana/wallet-adapter-react';
import { useMemo, useState } from 'react';
import { DEMO_MODE } from '../api';
import { Button, Card, Chip, Eyebrow, ProtocolBadge, Skeleton, tokens } from '../components/sonar';
import { useAlerts } from '../hooks';
import { MOCK_ALERTS } from '../mockData';
import type { Alert, Protocol, Severity } from '../types';
import { alertWireToAlert, fmtPct, timeAgo } from '../utils';

const SEVERITIES: Array<Severity | 'All'> = ['All', 'Info', 'Warning', 'Critical'];
const PROTOCOLS: Array<Protocol | 'All'> = ['All', 'Kamino', 'Save', 'Marginfi'];

export function Alerts() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;
  const alertsQ = useAlerts(useLive ? wallet : null);
  const source = useMemo(
    () => (useLive && alertsQ.data ? alertsQ.data.map(alertWireToAlert) : MOCK_ALERTS),
    [alertsQ.data, useLive],
  );
  const [severity, setSeverity] = useState<Severity | 'All'>('All');
  const [protocol, setProtocol] = useState<Protocol | 'All'>('All');
  const [expanded, setExpanded] = useState<string | null>(null);

  const filtered = source.filter((a) => {
    if (severity !== 'All' && a.severity !== severity) return false;
    if (protocol !== 'All' && a.metadata.protocol !== protocol) return false;
    return true;
  });

  return (
    <main style={{ padding: '64px 28px 72px', maxWidth: 980, margin: '0 auto' }}>
      <PageHead title="Alerts" sub="AI risk summaries, liquidation warnings, and protocol events." />

      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginBottom: 22 }}>
        {SEVERITIES.map((s) => (
          <Filter key={s} active={severity === s} onClick={() => setSeverity(s)}>{s}</Filter>
        ))}
        <span style={{ width: 1, height: 36, background: tokens.lineSoft }} />
        {PROTOCOLS.map((p) => (
          <Filter key={p} active={protocol === p} onClick={() => setProtocol(p)}>{p}</Filter>
        ))}
      </div>

      {alertsQ.isLoading && useLive ? (
        <div style={{ display: 'grid', gap: 12 }}>
          {[1, 2, 3].map((i) => <Skeleton key={i} height={88} />)}
        </div>
      ) : filtered.length === 0 ? (
        <Card pad={48} style={{ textAlign: 'center', background: 'var(--surface-1)' }}>
          <h2 style={{ fontFamily: tokens.sans, fontSize: 20, margin: 0 }}>No alerts match this filter.</h2>
        </Card>
      ) : (
        <div style={{ display: 'grid', gap: 12 }}>
          {filtered.map((alert) => (
            <AlertRow
              key={alert.id}
              alert={alert}
              expanded={expanded === alert.id}
              onToggle={() => setExpanded(expanded === alert.id ? null : alert.id)}
            />
          ))}
        </div>
      )}
    </main>
  );
}

function PageHead({ title, sub }: { title: string; sub: string }) {
  return (
    <div style={{ marginBottom: 28 }}>
      <h1 style={{ fontFamily: tokens.sans, fontSize: 34, fontWeight: 700, letterSpacing: '-0.025em', margin: 0 }}>{title}</h1>
      <p style={{ fontFamily: tokens.sans, fontSize: 16, color: 'color-mix(in oklab, var(--ink) 57%, transparent)', marginTop: 8 }}>{sub}</p>
    </div>
  );
}

function Filter({ active, onClick, children }: { active: boolean; onClick: () => void; children: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        minHeight: 36,
        padding: '0 16px',
        borderRadius: 999,
        border: `1px solid ${active ? tokens.ink : tokens.lineSoft}`,
        background: active ? tokens.ink : 'var(--surface-1)',
        color: active ? tokens.paper : tokens.ink,
        fontFamily: tokens.sans,
        fontSize: 14,
        cursor: 'pointer',
        transition: 'background 0.15s, transform 0.12s',
      }}
      onMouseEnter={(e) => { if (!active) e.currentTarget.style.background = 'var(--surface-2)'; }}
      onMouseLeave={(e) => { if (!active) e.currentTarget.style.background = 'var(--surface-1)'; }}
    >
      {children}
    </button>
  );
}

function AlertRow({ alert, expanded, onToggle }: { alert: Alert; expanded: boolean; onToggle: () => void }) {
  const tone = alert.severity === 'Critical' ? 'critical' : alert.severity === 'Warning' ? 'watch' : 'neutral';
  const color = alert.severity === 'Critical' ? tokens.rust : alert.severity === 'Warning' ? '#d99b2b' : '#64748b';
  const proto = alert.metadata.protocol;
  return (
    <Card pad={0} style={{ overflow: 'hidden', background: 'var(--surface-1)', borderLeft: `4px solid ${color}` }}>
      <button
        type="button"
        onClick={onToggle}
        style={{
          width: '100%',
          border: 0,
          background: 'transparent',
          padding: '18px 22px',
          display: 'grid',
          gridTemplateColumns: '140px 1fr auto',
          gap: 14,
          alignItems: 'start',
          textAlign: 'left',
          cursor: 'pointer',
        }}
      >
        <Chip tone={tone}>{alert.severity}</Chip>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <strong style={{ fontFamily: tokens.sans, fontSize: 16 }}>{alert.title}</strong>
            {proto ? <ProtocolBadge protocol={proto} size={22} /> : null}
          </div>
          <p style={{ fontFamily: tokens.sans, fontSize: 14, color: 'color-mix(in oklab, var(--ink) 57%, transparent)', lineHeight: 1.5, marginTop: 6 }}>
            {alert.message}
          </p>
        </div>
        <div style={{ fontFamily: tokens.sans, color: 'color-mix(in oklab, var(--ink) 46%, transparent)', fontSize: 13, whiteSpace: 'nowrap' }}>
          {timeAgo(alert.created_at)} {expanded ? '▲' : '▼'}
        </div>
      </button>
      {expanded ? (
        <div style={{ borderTop: `1px solid ${tokens.lineSoft}`, padding: '18px 22px', display: 'grid', gap: 16 }}>
          <div style={{ display: 'flex', gap: 28, flexWrap: 'wrap' }}>
            <Metric label="Health at alert" value={String(alert.health_score)} />
            <Metric label="LTV at alert" value={fmtPct(alert.ltv)} />
            <Metric label="Created" value={new Date(alert.created_at).toLocaleString()} />
          </div>
          {alert.suggested_actions.length ? (
            <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
              {alert.suggested_actions.map((action) => <Button key={action} size="sm">{action}</Button>)}
            </div>
          ) : null}
        </div>
      ) : null}
    </Card>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <Eyebrow>{label}</Eyebrow>
      <div style={{ fontFamily: tokens.mono, fontSize: 15, fontWeight: 600, marginTop: 4 }}>{value}</div>
    </div>
  );
}
