import { useWallet } from '@solana/wallet-adapter-react';
import { useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { DEMO_MODE } from '../api';
import { EmptyState, ProtocolBadge, SeverityBadge } from '../components/ui';
import { useAlerts } from '../hooks';
import { MOCK_ALERTS } from '../mockData';
import type { Alert, Protocol, Severity } from '../types';
import { alertWireToAlert, fmtPct, healthColor, severityColor, timeAgo } from '../utils';

const SEVERITIES: (Severity | 'All')[] = ['All', 'Info', 'Warning', 'Critical'];
const PROTOCOLS: (Protocol | 'All')[] = ['All', 'Kamino', 'Save', 'Marginfi'];

export function Alerts() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;
  const alertsQ = useAlerts(useLive ? wallet : null);

  const source = useMemo(
    () => (useLive && alertsQ.data ? alertsQ.data.map(alertWireToAlert) : MOCK_ALERTS),
    [useLive, alertsQ.data],
  );

  const [severity, setSeverity] = useState<Severity | 'All'>('All');
  const [protocol, setProtocol] = useState<Protocol | 'All'>('All');
  const [expanded, setExpanded] = useState<string | null>(null);

  const filtered = source.filter((a) => {
    if (severity !== 'All' && a.severity !== severity) return false;
    if (protocol !== 'All' && a.metadata?.protocol !== protocol) return false;
    return true;
  });

  return (
    <div style={{ padding: '88px 28px 60px', maxWidth: 860, margin: '0 auto' }}>
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
        Alerts
      </h2>
      <p
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 14,
          color: 'rgba(245,244,239,0.4)',
          marginBottom: 28,
        }}
      >
        AI-generated risk summaries for your positions.
      </p>

      <div
        style={{
          display: 'flex',
          gap: 10,
          marginBottom: 28,
          flexWrap: 'wrap',
          alignItems: 'center',
        }}
      >
        <span
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 12,
            color: 'rgba(245,244,239,0.35)',
            marginRight: 4,
          }}
        >
          Severity
        </span>
        {SEVERITIES.map((s) => (
          <FilterChip
            key={s}
            label={s}
            active={severity === s}
            color={s === 'All' ? null : severityColor(s)}
            onClick={() => setSeverity(s)}
          />
        ))}
        <div
          style={{
            width: 1,
            height: 20,
            background: 'rgba(255,255,255,0.1)',
            margin: '0 6px',
          }}
        />
        <span
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 12,
            color: 'rgba(245,244,239,0.35)',
            marginRight: 4,
          }}
        >
          Protocol
        </span>
        {PROTOCOLS.map((p) => (
          <FilterChip
            key={p}
            label={p}
            active={protocol === p}
            color={null}
            onClick={() => setProtocol(p)}
          />
        ))}
      </div>

      {filtered.length === 0 ? (
        <EmptyState />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {filtered.map((alert, idx) => (
            <AlertCard
              key={alert.id}
              alert={alert}
              expanded={expanded === alert.id}
              onToggle={() => setExpanded(expanded === alert.id ? null : alert.id)}
              isFirst={idx === 0}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function FilterChip({
  label,
  active,
  color,
  onClick,
}: {
  label: string;
  active: boolean;
  color: string | null;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        background: active
          ? color
            ? `${color}22`
            : 'rgba(217,119,87,0.15)'
          : 'rgba(255,255,255,0.05)',
        border: `1px solid ${active ? (color ?? '#D97757') + '66' : 'rgba(255,255,255,0.1)'}`,
        borderRadius: 100,
        padding: '5px 14px',
        cursor: 'pointer',
        fontFamily: "'Inter', sans-serif",
        fontSize: 12,
        fontWeight: 500,
        color: active ? (color ?? '#D97757') : 'rgba(245,244,239,0.5)',
        transition: 'all 0.15s',
      }}
    >
      {label}
    </button>
  );
}

function AlertCard({
  alert,
  expanded,
  onToggle,
  isFirst,
}: {
  alert: Alert;
  expanded: boolean;
  onToggle: () => void;
  isFirst: boolean;
}) {
  const c = severityColor(alert.severity);
  const proto = alert.metadata?.protocol;
  return (
    <div
      style={{
        background: '#2A2826',
        borderRadius: 18,
        border: '1px solid rgba(255,255,255,0.07)',
        borderLeft: `3px solid ${c}`,
        overflow: 'hidden',
        boxShadow:
          isFirst && alert.severity === 'Warning' ? `0 0 24px ${c}18` : 'none',
      }}
    >
      <div
        onClick={onToggle}
        style={{
          padding: '18px 22px',
          cursor: 'pointer',
          display: 'flex',
          gap: 14,
          alignItems: 'flex-start',
        }}
      >
        <div style={{ paddingTop: 1 }}>
          <SeverityBadge severity={alert.severity} />
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              display: 'flex',
              gap: 8,
              alignItems: 'center',
              marginBottom: 4,
              flexWrap: 'wrap',
            }}
          >
            <span
              style={{
                fontFamily: "'Inter', sans-serif",
                fontSize: 14,
                fontWeight: 600,
                color: '#F5F4EF',
              }}
            >
              {alert.title}
            </span>
            {proto && <ProtocolBadge protocol={proto} />}
          </div>
          <p
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 13,
              color: 'rgba(245,244,239,0.5)',
              margin: 0,
              lineHeight: 1.5,
              display: expanded ? 'block' : '-webkit-box',
              WebkitLineClamp: 2,
              WebkitBoxOrient: 'vertical',
              overflow: expanded ? 'visible' : 'hidden',
            }}
          >
            {alert.message}
          </p>
        </div>
        <div style={{ flexShrink: 0, textAlign: 'right' }}>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 11,
              color: 'rgba(245,244,239,0.25)',
              marginBottom: 6,
            }}
          >
            {timeAgo(alert.created_at)}
          </div>
          <span style={{ color: 'rgba(245,244,239,0.25)', fontSize: 12 }}>
            {expanded ? '▲' : '▼'}
          </span>
        </div>
      </div>

      {expanded && (
        <div
          style={{
            padding: '0 22px 20px',
            borderTop: '1px solid rgba(255,255,255,0.06)',
          }}
        >
          <div
            style={{
              paddingTop: 16,
              display: 'flex',
              gap: 24,
              flexWrap: 'wrap',
              marginBottom: 16,
            }}
          >
            <Metric
              label="Health at alert"
              value={alert.health_score}
              color={healthColor(alert.health_score)}
              mono
            />
            <Metric
              label="LTV at alert"
              value={fmtPct(alert.ltv)}
              color={alert.ltv > 0.7 ? '#D9604E' : '#E4A853'}
              mono
            />
            <Metric
              label="Time"
              value={new Date(alert.created_at).toLocaleString('en-US', {
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
              })}
            />
          </div>

          {alert.suggested_actions.length > 0 && (
            <div>
              <div
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 11,
                  color: 'rgba(245,244,239,0.3)',
                  letterSpacing: '0.07em',
                  textTransform: 'uppercase',
                  marginBottom: 10,
                }}
              >
                Suggested Actions
              </div>
              <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                {alert.suggested_actions.map((action) => (
                  <button
                    key={action}
                    style={{
                      background: 'rgba(217,119,87,0.12)',
                      border: '1px solid rgba(217,119,87,0.3)',
                      borderRadius: 100,
                      padding: '6px 14px',
                      cursor: 'pointer',
                      fontFamily: "'Inter', sans-serif",
                      fontSize: 12,
                      fontWeight: 500,
                      color: '#D97757',
                      transition: 'background 0.15s',
                    }}
                    onMouseEnter={(e) =>
                      (e.currentTarget.style.background = 'rgba(217,119,87,0.2)')
                    }
                    onMouseLeave={(e) =>
                      (e.currentTarget.style.background = 'rgba(217,119,87,0.12)')
                    }
                  >
                    {action}
                  </button>
                ))}
              </div>
            </div>
          )}

          {alert.metadata && Object.keys(alert.metadata).length > 0 && (
            <details style={{ marginTop: 14 }}>
              <summary
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 11,
                  color: 'rgba(245,244,239,0.3)',
                  cursor: 'pointer',
                  letterSpacing: '0.05em',
                }}
              >
                Raw metadata
              </summary>
              <pre
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: 11,
                  color: 'rgba(245,244,239,0.4)',
                  marginTop: 8,
                  background: 'rgba(0,0,0,0.2)',
                  padding: 12,
                  borderRadius: 8,
                  overflowX: 'auto',
                }}
              >
                {JSON.stringify(alert.metadata, null, 2)}
              </pre>
            </details>
          )}
        </div>
      )}
    </div>
  );
}

function Metric({
  label,
  value,
  color,
  mono,
}: {
  label: string;
  value: ReactNode;
  color?: string;
  mono?: boolean;
}) {
  return (
    <div>
      <div
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 10,
          color: 'rgba(245,244,239,0.3)',
          textTransform: 'uppercase',
          letterSpacing: '0.07em',
          marginBottom: 4,
        }}
      >
        {label}
      </div>
      <div
        style={{
          fontFamily: mono ? "'JetBrains Mono', monospace" : "'Inter', sans-serif",
          fontSize: 16,
          fontWeight: 600,
          color: color ?? '#F5F4EF',
        }}
      >
        {value}
      </div>
    </div>
  );
}
