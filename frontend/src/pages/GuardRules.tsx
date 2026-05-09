import { useWallet } from '@solana/wallet-adapter-react';
import { useEffect, useState } from 'react';
import { DEMO_MODE } from '../api';
import { Button, Card, Chip, Eyebrow, ProtocolBadge, Skeleton, tokens } from '../components/sonar';
import { useDeleteGuardRule, useGuardRules, useUpsertGuardRule } from '../hooks';
import { MOCK_GUARD_RULES, MOCK_WALLET_FULL } from '../mockData';
import type { GuardRule, GuardRuleWire, Protocol, TriggerKind } from '../types';
import { fmtUsd, guardRuleWireToRule, timeAgo } from '../utils';

export function GuardRules() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const walletAddr = wallet ?? MOCK_WALLET_FULL;
  const useLive = !DEMO_MODE && !!wallet;
  const rulesQ = useGuardRules(useLive ? wallet : null);
  const upsert = useUpsertGuardRule();
  const deleteRule = useDeleteGuardRule();
  const [rules, setRules] = useState<GuardRule[]>(MOCK_GUARD_RULES);
  const [showModal, setShowModal] = useState(false);

  useEffect(() => {
    if (useLive && rulesQ.data) setRules(rulesQ.data.map(guardRuleWireToRule));
    if (!useLive) setRules(MOCK_GUARD_RULES);
  }, [rulesQ.data, useLive]);

  const toWire = (r: GuardRule): GuardRuleWire => ({
    id: r.id.startsWith('gr') ? undefined : r.id,
    wallet: r.wallet,
    protocol: r.protocol,
    trigger_kind: r.trigger_kind,
    trigger_value: r.trigger_value,
    action_kind: r.action_kind,
    action_token: r.action_token,
    action_amount_usd: r.action_amount_usd,
    max_usd_per_action: r.max_usd_per_action,
    daily_limit_usd: r.daily_limit_usd,
    cooldown_seconds: r.cooldown_seconds,
    is_active: r.is_active,
    last_fired_at: r.last_fired_at,
  });

  const toggle = (rule: GuardRule) => {
    const next = { ...rule, is_active: !rule.is_active };
    setRules((rows) => rows.map((r) => (r.id === rule.id ? next : r)));
    if (useLive) upsert.mutate(toWire(next));
  };

  return (
    <main style={{ padding: '64px 28px 72px', maxWidth: 980, margin: '0 auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'end', gap: 20, marginBottom: 28 }}>
        <div>
          <h1 style={{ fontFamily: tokens.sans, fontSize: 34, fontWeight: 700, letterSpacing: '-0.025em', margin: 0 }}>Guardrails</h1>
          <p style={{ fontFamily: tokens.sans, fontSize: 16, color: 'color-mix(in oklab, var(--ink) 57%, transparent)', marginTop: 8 }}>
            Automated thresholds for alerts, repayments, and position defense.
          </p>
        </div>
        <Button variant="danger" onClick={() => setShowModal(true)}>+ New Rule</Button>
      </div>

      {rulesQ.isLoading && useLive ? (
        <div style={{ display: 'grid', gap: 12 }}>{[1, 2, 3].map((i) => <Skeleton key={i} height={120} />)}</div>
      ) : (
        <div style={{ display: 'grid', gap: 14 }}>
          {rules.map((rule) => (
            <RuleCard
              key={rule.id}
              rule={rule}
              onToggle={() => toggle(rule)}
              onDelete={() => {
                setRules((rows) => rows.filter((r) => r.id !== rule.id));
                if (useLive) deleteRule.mutate(rule.id);
              }}
            />
          ))}
        </div>
      )}

      <Card pad={20} style={{ marginTop: 22, background: 'rgba(196,69,54,0.05)', borderColor: 'rgba(196,69,54,0.22)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 16, alignItems: 'center' }}>
          <div>
            <strong style={{ fontFamily: tokens.sans }}>Autonomous execution</strong>
            <p style={{ fontFamily: tokens.sans, color: 'color-mix(in oklab, var(--ink) 57%, transparent)', marginTop: 4 }}>
              Repay and deleverage actions require Pro and wallet approval safeguards.
            </p>
          </div>
          <Button>Upgrade</Button>
        </div>
      </Card>

      {showModal ? (
        <NewRuleModal
          wallet={walletAddr}
          onClose={() => setShowModal(false)}
          onSave={(rule) => {
            setRules((rows) => [rule, ...rows]);
            if (useLive) upsert.mutate(toWire(rule));
            setShowModal(false);
          }}
        />
      ) : null}
    </main>
  );
}

function RuleCard({ rule, onToggle, onDelete }: { rule: GuardRule; onToggle: () => void; onDelete: () => void }) {
  const trigger = rule.trigger_kind === 'HealthBelow'
    ? `When health drops below ${rule.trigger_value}`
    : rule.trigger_kind === 'LtvAbove'
      ? `When LTV exceeds ${(rule.trigger_value * 100).toFixed(0)}%`
      : rule.trigger_kind === 'DebtAboveUsd'
        ? `When debt exceeds ${fmtUsd(rule.trigger_value)}`
        : `When health drops ${(rule.trigger_value * 100).toFixed(0)}%`;
  const action = rule.action_kind === 'NotifyOnly'
    ? 'Send notification'
    : rule.action_kind === 'RepayDebt'
      ? `Repay ${fmtUsd(rule.action_amount_usd)} ${rule.action_token ?? ''}`
      : rule.action_kind === 'AddCollateral'
        ? `Add ${rule.action_token ?? 'collateral'} (${fmtUsd(rule.action_amount_usd)})`
        : 'Deleverage position';
  const locked = rule.action_kind !== 'NotifyOnly';

  return (
    <Card pad={22} style={{ background: 'var(--surface-1)', opacity: rule.is_active ? 1 : 0.62 }}>
      <div style={{ display: 'grid', gridTemplateColumns: '76px 1fr auto', gap: 20, alignItems: 'start' }}>
        <div style={{ display: 'grid', justifyItems: 'center', gap: 12 }}>
          <Switch checked={rule.is_active} onClick={onToggle} />
          <button type="button" onClick={onDelete} aria-label="Delete rule" style={{ border: 0, background: 'transparent', color: 'color-mix(in oklab, var(--ink) 35%, transparent)', cursor: 'pointer', fontSize: 20 }}>×</button>
        </div>
        <div>
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 14 }}>
            {rule.protocol ? <ProtocolBadge protocol={rule.protocol} size={22} /> : <Chip>All protocols</Chip>}
            {locked ? <Chip tone="watch">Pro</Chip> : null}
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20 }}>
            <Block label="Trigger" value={trigger} />
            <Block label="Action" value={action} accent={locked} />
          </div>
          <div style={{ height: 1, background: tokens.lineSoft, margin: '18px 0' }} />
          <div style={{ display: 'flex', gap: 34, flexWrap: 'wrap' }}>
            <Metric label="Max/action" value={rule.max_usd_per_action > 0 ? fmtUsd(rule.max_usd_per_action) : '—'} />
            <Metric label="Daily cap" value={rule.daily_limit_usd > 0 ? `${rule.daily_limit_usd} alerts` : '—'} />
            <Metric label="Cooldown" value={rule.cooldown_seconds >= 3600 ? `${rule.cooldown_seconds / 3600}h` : `${rule.cooldown_seconds / 60}m`} />
            <Metric label="Last fired" value={rule.last_fired_at ? timeAgo(rule.last_fired_at) : 'Never'} />
          </div>
        </div>
      </div>
    </Card>
  );
}

function Switch({ checked, onClick }: { checked: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={checked}
      style={{
        width: 46,
        height: 26,
        border: 0,
        borderRadius: 999,
        background: checked ? '#d27a5c' : '#e6e2d8',
        padding: 3,
        cursor: 'pointer',
        display: 'flex',
        justifyContent: checked ? 'flex-end' : 'flex-start',
      }}
    >
      <span style={{ width: 20, height: 20, borderRadius: '50%', background: 'var(--surface-1)', boxShadow: '0 1px 4px rgba(0,0,0,.16)' }} />
    </button>
  );
}

function Block({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <Eyebrow>{label}</Eyebrow>
      <div style={{ fontFamily: tokens.sans, fontSize: 16, color: accent ? '#c66f53' : tokens.ink, marginTop: 6 }}>{value}</div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <Eyebrow>{label}</Eyebrow>
      <div style={{ fontFamily: tokens.mono, fontSize: 14, color: 'color-mix(in oklab, var(--ink) 62%, transparent)', marginTop: 5 }}>{value}</div>
    </div>
  );
}

type TriggerOption = { kind: TriggerKind; label: string; min: number; max: number; step: number; default: number; suffix: string; format: (v: number) => string; toStored: (v: number) => number; fromStored: (v: number) => number };

const TRIGGERS: TriggerOption[] = [
  { kind: 'HealthBelow', label: 'Health drops below', min: 10, max: 95, step: 1, default: 60, suffix: '', format: (v) => `${v}`, toStored: (v) => v, fromStored: (v) => v },
  { kind: 'LtvAbove', label: 'LTV rises above', min: 30, max: 95, step: 1, default: 75, suffix: '%', format: (v) => `${v}%`, toStored: (v) => v / 100, fromStored: (v) => Math.round(v * 100) },
  { kind: 'DebtAboveUsd', label: 'Debt exceeds', min: 100, max: 100000, step: 100, default: 5000, suffix: '$', format: (v) => `$${v.toLocaleString('en-US')}`, toStored: (v) => v, fromStored: (v) => v },
  { kind: 'HealthDropped', label: 'Health drops by', min: 5, max: 50, step: 1, default: 15, suffix: '%', format: (v) => `${v}%`, toStored: (v) => v / 100, fromStored: (v) => Math.round(v * 100) },
];

const PROTOCOL_OPTIONS: Array<{ value: Protocol | ''; label: string }> = [
  { value: '', label: 'All protocols' },
  { value: 'Kamino', label: 'Kamino' },
  { value: 'Save', label: 'Save' },
  { value: 'Marginfi', label: 'Marginfi' },
];

const SELECT_STYLE: React.CSSProperties = {
  width: '100%',
  padding: '10px 12px',
  borderRadius: 8,
  border: `1px solid ${tokens.line}`,
  background: 'var(--surface-2)',
  color: 'var(--ink)',
  fontFamily: tokens.sans,
  fontSize: 14,
  cursor: 'pointer',
};

function NewRuleModal({ wallet, onClose, onSave }: { wallet: string; onClose: () => void; onSave: (rule: GuardRule) => void }) {
  const [protocol, setProtocol] = useState<Protocol | ''>('');
  const [triggerIdx, setTriggerIdx] = useState(0);
  const trig = TRIGGERS[triggerIdx];
  const [value, setValue] = useState<number>(trig.default);
  const [dailyCap, setDailyCap] = useState(20);
  const [cooldownMin, setCooldownMin] = useState(30);

  const onTriggerChange = (idx: number) => {
    setTriggerIdx(idx);
    setValue(TRIGGERS[idx].default);
  };

  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, zIndex: 200, background: 'rgba(15,15,15,0.42)', display: 'grid', placeItems: 'center', padding: 20 }}>
      <Card pad={24} style={{ width: 'min(560px, 100%)', background: 'var(--surface-1)' }} onClick={(e) => e.stopPropagation()}>
        <h2 style={{ fontFamily: tokens.sans, fontSize: 22, margin: 0 }}>New guardrail</h2>
        <p style={{ color: 'color-mix(in oklab, var(--ink) 57%, transparent)', fontFamily: tokens.sans, marginTop: 6 }}>
          Create a notify-only rule. Autonomous actions can be enabled later.
        </p>

        <label style={{ display: 'grid', gap: 8, marginTop: 20, fontFamily: tokens.sans, fontSize: 13 }}>
          <span style={{ color: 'color-mix(in oklab, var(--ink) 70%, transparent)' }}>Scope</span>
          <select style={SELECT_STYLE} value={protocol} onChange={(e) => setProtocol(e.target.value as Protocol | '')}>
            {PROTOCOL_OPTIONS.map((p) => <option key={p.value} value={p.value}>{p.label}</option>)}
          </select>
        </label>

        <label style={{ display: 'grid', gap: 8, marginTop: 16, fontFamily: tokens.sans, fontSize: 13 }}>
          <span style={{ color: 'color-mix(in oklab, var(--ink) 70%, transparent)' }}>Trigger</span>
          <select style={SELECT_STYLE} value={triggerIdx} onChange={(e) => onTriggerChange(Number(e.target.value))}>
            {TRIGGERS.map((t, i) => <option key={t.kind} value={i}>{t.label}</option>)}
          </select>
        </label>

        <label style={{ display: 'grid', gap: 8, marginTop: 16, fontFamily: tokens.sans, fontSize: 13 }}>
          <span style={{ color: 'color-mix(in oklab, var(--ink) 70%, transparent)' }}>Threshold</span>
          <input type="range" min={trig.min} max={trig.max} step={trig.step} value={value} onChange={(e) => setValue(Number(e.target.value))} />
          <strong style={{ fontFamily: tokens.mono, fontSize: 16 }}>{trig.format(value)}</strong>
        </label>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14, marginTop: 16 }}>
          <label style={{ display: 'grid', gap: 8, fontFamily: tokens.sans, fontSize: 13 }}>
            <span style={{ color: 'color-mix(in oklab, var(--ink) 70%, transparent)' }}>Daily alert cap</span>
            <input type="range" min={1} max={50} value={dailyCap} onChange={(e) => setDailyCap(Number(e.target.value))} />
            <strong style={{ fontFamily: tokens.mono }}>{dailyCap} alerts</strong>
          </label>
          <label style={{ display: 'grid', gap: 8, fontFamily: tokens.sans, fontSize: 13 }}>
            <span style={{ color: 'color-mix(in oklab, var(--ink) 70%, transparent)' }}>Cooldown</span>
            <input type="range" min={5} max={240} step={5} value={cooldownMin} onChange={(e) => setCooldownMin(Number(e.target.value))} />
            <strong style={{ fontFamily: tokens.mono }}>{cooldownMin >= 60 ? `${(cooldownMin / 60).toFixed(cooldownMin % 60 ? 1 : 0)}h` : `${cooldownMin}m`}</strong>
          </label>
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10, marginTop: 24 }}>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="accent"
            onClick={() => onSave({
              id: `gr${Date.now()}`,
              wallet,
              protocol: protocol || null,
              trigger_kind: trig.kind,
              trigger_value: trig.toStored(value),
              action_kind: 'NotifyOnly',
              action_token: null,
              action_amount_usd: null,
              max_usd_per_action: 0,
              daily_limit_usd: dailyCap,
              cooldown_seconds: cooldownMin * 60,
              is_active: true,
              last_fired_at: null,
            })}
          >
            Save rule
          </Button>
        </div>
      </Card>
    </div>
  );
}
