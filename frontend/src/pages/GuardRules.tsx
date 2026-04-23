import { useWallet } from '@solana/wallet-adapter-react';
import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { DEMO_MODE } from '../api';
import { Card, EmptyState, ProtocolBadge, Toggle } from '../components/ui';
import { useGuardRules, useUpsertGuardRule } from '../hooks';
import { MOCK_GUARD_RULES, MOCK_WALLET_FULL } from '../mockData';
import type { ActionKind, GuardRule, GuardRuleWire, Protocol, TriggerKind } from '../types';
import { fmtUsd, guardRuleWireToRule, timeAgo } from '../utils';

const STEPS = ['Trigger', 'Action', 'Guardrails', 'Review'] as const;

interface RuleForm {
  trigger_kind: TriggerKind;
  trigger_value: number;
  action_kind: ActionKind;
  action_token: string;
  action_amount_usd: number;
  max_usd_per_action: number;
  daily_limit_usd: number;
  cooldown_seconds: number;
  protocol: Protocol | null;
  is_active: boolean;
}

export function GuardRules() {
  const { publicKey } = useWallet();
  const wallet = publicKey?.toBase58() ?? null;
  const useLive = !DEMO_MODE && !!wallet;
  const walletAddr = wallet ?? MOCK_WALLET_FULL;

  const rulesQ = useGuardRules(useLive ? wallet : null);
  const upsert = useUpsertGuardRule();

  const [localRules, setLocalRules] = useState<GuardRule[]>(MOCK_GUARD_RULES);
  const [showModal, setShowModal] = useState(false);
  const isPremium = false;

  useEffect(() => {
    if (useLive && rulesQ.data) {
      setLocalRules(rulesQ.data.map(guardRuleWireToRule));
    } else if (!useLive) {
      setLocalRules(MOCK_GUARD_RULES);
    }
  }, [useLive, rulesQ.data]);

  const rules = localRules;

  const ruleToWire = (r: GuardRule): GuardRuleWire => ({
    id: r.id.startsWith('rule-') || r.id.startsWith('gr') ? undefined : r.id,
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

  const toggleRule = (id: string) => {
    const rule = rules.find((r) => r.id === id);
    if (!rule) return;
    const next = { ...rule, is_active: !rule.is_active };
    setLocalRules((r) => r.map((x) => (x.id === id ? next : x)));
    if (useLive) upsert.mutate(ruleToWire(next));
  };

  return (
    <div style={{ padding: '88px 28px 60px', maxWidth: 860, margin: '0 auto' }}>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'flex-end',
          marginBottom: 28,
        }}
      >
        <div>
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
            Guard Rules
          </h2>
          <p
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 14,
              color: 'rgba(245,244,239,0.4)',
            }}
          >
            Automated actions that execute when thresholds are breached.
          </p>
        </div>
        <button
          onClick={() => setShowModal(true)}
          style={{
            background: '#D97757',
            border: 'none',
            cursor: 'pointer',
            padding: '11px 22px',
            borderRadius: 100,
            fontFamily: "'Inter', sans-serif",
            fontSize: 13,
            fontWeight: 600,
            color: '#1F1E1D',
            flexShrink: 0,
          }}
        >
          + New Rule
        </button>
      </div>

      {rules.length === 0 ? (
        <EmptyState text="No guard rules yet. Create one to stay protected." />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          {rules.map((rule) => (
            <RuleCard
              key={rule.id}
              rule={rule}
              isPremium={isPremium}
              onToggle={() => toggleRule(rule.id)}
            />
          ))}
        </div>
      )}

      {!isPremium && (
        <div
          style={{
            marginTop: 28,
            background: 'rgba(217,119,87,0.06)',
            border: '1px solid rgba(217,119,87,0.2)',
            borderRadius: 18,
            padding: '20px 24px',
            display: 'flex',
            gap: 16,
            alignItems: 'center',
            boxShadow: '0 0 32px rgba(217,119,87,0.08)',
          }}
        >
          <div style={{ fontSize: 22 }}>🔒</div>
          <div style={{ flex: 1 }}>
            <div
              style={{
                fontFamily: "'Fraunces', serif",
                fontSize: 16,
                fontWeight: 500,
                color: '#F5F4EF',
                marginBottom: 4,
              }}
            >
              Unlock autonomous execution
            </div>
            <div
              style={{
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                color: 'rgba(245,244,239,0.45)',
              }}
            >
              Upgrade to Aegis Pro to enable AddCollateral, RepayDebt, and Deleverage actions.
            </div>
          </div>
          <button
            style={{
              background: '#D97757',
              border: 'none',
              cursor: 'pointer',
              padding: '10px 20px',
              borderRadius: 100,
              fontFamily: "'Inter', sans-serif",
              fontSize: 13,
              fontWeight: 600,
              color: '#1F1E1D',
              flexShrink: 0,
            }}
          >
            Upgrade
          </button>
        </div>
      )}

      {showModal && (
        <NewRuleModal
          isPremium={isPremium}
          onClose={() => setShowModal(false)}
          onSave={(form) => {
            const newRule: GuardRule = {
              id: `gr${Date.now()}`,
              wallet: walletAddr,
              protocol: form.protocol,
              trigger_kind: form.trigger_kind,
              trigger_value: form.trigger_value,
              action_kind: form.action_kind,
              action_token: form.action_kind === 'NotifyOnly' ? null : form.action_token,
              action_amount_usd:
                form.action_kind === 'NotifyOnly' ? null : form.action_amount_usd,
              max_usd_per_action: form.max_usd_per_action,
              daily_limit_usd: form.daily_limit_usd,
              cooldown_seconds: form.cooldown_seconds,
              is_active: form.is_active,
              last_fired_at: null,
            };
            setLocalRules((r) => [...r, newRule]);
            if (useLive) upsert.mutate(ruleToWire(newRule));
            setShowModal(false);
          }}
        />
      )}
    </div>
  );
}

function RuleCard({
  rule,
  isPremium,
  onToggle,
}: {
  rule: GuardRule;
  isPremium: boolean;
  onToggle: () => void;
}) {
  const isAutoAction = rule.action_kind !== 'NotifyOnly';
  const locked = isAutoAction && !isPremium;

  const triggerLabel: string =
    rule.trigger_kind === 'HealthBelow'
      ? `Health drops below ${rule.trigger_value}`
      : rule.trigger_kind === 'LtvAbove'
        ? `LTV exceeds ${(rule.trigger_value * 100).toFixed(0)}%`
        : `Debt exceeds ${fmtUsd(rule.trigger_value)}`;

  const actionLabel: string =
    rule.action_kind === 'NotifyOnly'
      ? 'Send notification'
      : rule.action_kind === 'AddCollateral'
        ? `Add ${rule.action_token ?? 'collateral'} (${fmtUsd(rule.action_amount_usd)})`
        : rule.action_kind === 'RepayDebt'
          ? `Repay ${rule.action_amount_usd ? fmtUsd(rule.action_amount_usd) : ''} ${rule.action_token ?? 'debt'}`
          : 'Deleverage position';

  return (
    <Card style={{ padding: '20px 24px', opacity: rule.is_active ? 1 : 0.6 }}>
      <div style={{ display: 'flex', gap: 16, alignItems: 'flex-start' }}>
        <Toggle checked={rule.is_active && !locked} onChange={onToggle} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              display: 'flex',
              gap: 8,
              alignItems: 'center',
              marginBottom: 10,
              flexWrap: 'wrap',
            }}
          >
            {rule.protocol && <ProtocolBadge protocol={rule.protocol} />}
            {locked && (
              <span
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 11,
                  color: '#D97757',
                  background: 'rgba(217,119,87,0.12)',
                  border: '1px solid rgba(217,119,87,0.25)',
                  borderRadius: 100,
                  padding: '2px 9px',
                }}
              >
                🔒 Pro
              </span>
            )}
          </div>

          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '1fr 1fr',
              gap: '10px 24px',
              marginBottom: 14,
            }}
          >
            <div>
              <Label>Trigger</Label>
              <div
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 13,
                  color: '#F5F4EF',
                }}
              >
                When {triggerLabel}
              </div>
            </div>
            <div>
              <Label>Action</Label>
              <div
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 13,
                  color: locked ? '#D97757' : '#F5F4EF',
                }}
              >
                {actionLabel}
              </div>
            </div>
          </div>

          <div
            style={{
              display: 'flex',
              gap: 20,
              flexWrap: 'wrap',
              borderTop: '1px solid rgba(255,255,255,0.06)',
              paddingTop: 12,
            }}
          >
            {[
              {
                label: 'Max/action',
                value: rule.max_usd_per_action > 0 ? fmtUsd(rule.max_usd_per_action) : '—',
              },
              {
                label: 'Daily limit',
                value: rule.daily_limit_usd > 0 ? fmtUsd(rule.daily_limit_usd) : '—',
              },
              {
                label: 'Cooldown',
                value:
                  rule.cooldown_seconds >= 3600
                    ? `${rule.cooldown_seconds / 3600}h`
                    : `${rule.cooldown_seconds / 60}m`,
              },
              {
                label: 'Last fired',
                value: rule.last_fired_at ? timeAgo(rule.last_fired_at) : 'Never',
              },
            ].map(({ label, value }) => (
              <div key={label}>
                <Label>{label}</Label>
                <div
                  style={{
                    fontFamily: "'JetBrains Mono', monospace",
                    fontSize: 12,
                    color: 'rgba(245,244,239,0.6)',
                  }}
                >
                  {value}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </Card>
  );
}

function Label({ children }: { children: ReactNode }) {
  return (
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
      {children}
    </div>
  );
}

function NewRuleModal({
  isPremium,
  onClose,
  onSave,
}: {
  isPremium: boolean;
  onClose: () => void;
  onSave: (form: RuleForm) => void;
}) {
  const [step, setStep] = useState(0);
  const [form, setForm] = useState<RuleForm>({
    trigger_kind: 'HealthBelow',
    trigger_value: 60,
    action_kind: 'NotifyOnly',
    action_token: 'USDC',
    action_amount_usd: 300,
    max_usd_per_action: 300,
    daily_limit_usd: 600,
    cooldown_seconds: 3600,
    protocol: null,
    is_active: true,
  });

  const set = <K extends keyof RuleForm>(k: K, v: RuleForm[K]) =>
    setForm((f) => ({ ...f, [k]: v }));
  const isAutoAction = form.action_kind !== 'NotifyOnly';
  const locked = isAutoAction && !isPremium;

  const triggerSummary =
    form.trigger_kind === 'HealthBelow'
      ? `Health < ${form.trigger_value}`
      : form.trigger_kind === 'LtvAbove'
        ? `LTV > ${(form.trigger_value * 100).toFixed(0)}%`
        : `Debt > ${fmtUsd(form.trigger_value)}`;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 200,
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.6)',
        backdropFilter: 'blur(8px)',
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: '#2A2826',
          borderRadius: '24px 24px 0 0',
          border: '1px solid rgba(255,255,255,0.1)',
          width: '100%',
          maxWidth: 600,
          padding: '28px 28px 40px',
          boxShadow: '0 -16px 64px rgba(0,0,0,0.5)',
          animation: 'slideUp 0.3s cubic-bezier(0.34,1.56,0.64,1) both',
        }}
      >
        <div style={{ display: 'flex', gap: 6, marginBottom: 28 }}>
          {STEPS.map((s, i) => (
            <div
              key={s}
              style={{
                flex: 1,
                height: 3,
                borderRadius: 3,
                background: i <= step ? '#D97757' : 'rgba(255,255,255,0.1)',
                transition: 'background 0.3s',
              }}
            />
          ))}
        </div>

        <div
          style={{
            fontFamily: "'Fraunces', serif",
            fontSize: 22,
            fontWeight: 600,
            color: '#F5F4EF',
            marginBottom: 4,
          }}
        >
          {STEPS[step]}
        </div>
        <div
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 13,
            color: 'rgba(245,244,239,0.4)',
            marginBottom: 24,
          }}
        >
          {
            [
              'Define when this rule fires',
              'What should Aegis do?',
              'Set spending limits',
              'Review and activate',
            ][step]
          }
        </div>

        {step === 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <WizardSelect
              label="Trigger type"
              value={form.trigger_kind}
              onChange={(v) => set('trigger_kind', v as TriggerKind)}
              options={[
                ['HealthBelow', 'Health drops below'],
                ['LtvAbove', 'LTV exceeds'],
                ['DebtAboveUsd', 'Debt exceeds USD'],
              ]}
            />
            <WizardSlider
              label={
                form.trigger_kind === 'HealthBelow'
                  ? 'Health threshold'
                  : form.trigger_kind === 'LtvAbove'
                    ? 'LTV threshold (%)'
                    : 'Debt threshold (USD)'
              }
              value={form.trigger_value}
              min={
                form.trigger_kind === 'DebtAboveUsd'
                  ? 100
                  : form.trigger_kind === 'LtvAbove'
                    ? 0.5
                    : 10
              }
              max={
                form.trigger_kind === 'DebtAboveUsd'
                  ? 10000
                  : form.trigger_kind === 'LtvAbove'
                    ? 0.95
                    : 90
              }
              step={
                form.trigger_kind === 'DebtAboveUsd'
                  ? 50
                  : form.trigger_kind === 'LtvAbove'
                    ? 0.01
                    : 1
              }
              format={(v) =>
                form.trigger_kind === 'LtvAbove'
                  ? `${(v * 100).toFixed(0)}%`
                  : form.trigger_kind === 'DebtAboveUsd'
                    ? fmtUsd(v)
                    : String(v)
              }
              onChange={(v) => set('trigger_value', v)}
            />
            <WizardSelect
              label="Protocol (optional)"
              value={form.protocol ?? ''}
              onChange={(v) => set('protocol', v === '' ? null : (v as Protocol))}
              options={[
                ['', 'All protocols'],
                ['Kamino', 'Kamino'],
                ['Save', 'Save'],
                ['Marginfi', 'Marginfi'],
              ]}
            />
          </div>
        )}

        {step === 1 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <div>
              <div
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 12,
                  color: 'rgba(245,244,239,0.5)',
                  marginBottom: 10,
                }}
              >
                Action type
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                {(
                  [
                    ['NotifyOnly', 'Notify only'],
                    ['AddCollateral', 'Add collateral'],
                    ['RepayDebt', 'Repay debt'],
                    ['Deleverage', 'Deleverage'],
                  ] as [ActionKind, string][]
                ).map(([val, lbl]) => {
                  const needsPro = val !== 'NotifyOnly' && !isPremium;
                  return (
                    <button
                      key={val}
                      onClick={() => set('action_kind', val)}
                      style={{
                        padding: '14px 16px',
                        borderRadius: 14,
                        border: `1px solid ${form.action_kind === val ? '#D97757' : 'rgba(255,255,255,0.1)'}`,
                        background:
                          form.action_kind === val
                            ? 'rgba(217,119,87,0.15)'
                            : 'rgba(255,255,255,0.03)',
                        cursor: 'pointer',
                        textAlign: 'left',
                        position: 'relative',
                      }}
                    >
                      <div
                        style={{
                          fontFamily: "'Inter', sans-serif",
                          fontSize: 13,
                          fontWeight: 600,
                          color: needsPro ? 'rgba(245,244,239,0.4)' : '#F5F4EF',
                        }}
                      >
                        {lbl}
                      </div>
                      {needsPro && (
                        <span
                          style={{
                            fontFamily: "'Inter', sans-serif",
                            fontSize: 10,
                            color: '#D97757',
                          }}
                        >
                          🔒 Pro
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
            {form.action_kind !== 'NotifyOnly' && (
              <>
                <WizardSelect
                  label="Token"
                  value={form.action_token}
                  onChange={(v) => set('action_token', v)}
                  options={[
                    ['USDC', 'USDC'],
                    ['SOL', 'SOL'],
                    ['mSOL', 'mSOL'],
                    ['USDT', 'USDT'],
                  ]}
                />
                <WizardSlider
                  label="Amount (USD)"
                  value={form.action_amount_usd}
                  min={50}
                  max={5000}
                  step={50}
                  format={(v) => fmtUsd(v)}
                  onChange={(v) => set('action_amount_usd', v)}
                />
              </>
            )}
          </div>
        )}

        {step === 2 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <WizardSlider
              label="Max per action (USD)"
              value={form.max_usd_per_action}
              min={50}
              max={5000}
              step={50}
              format={(v) => fmtUsd(v)}
              onChange={(v) => set('max_usd_per_action', v)}
            />
            <WizardSlider
              label="Daily limit (USD)"
              value={form.daily_limit_usd}
              min={100}
              max={20000}
              step={100}
              format={(v) => fmtUsd(v)}
              onChange={(v) => set('daily_limit_usd', v)}
            />
            <WizardSlider
              label="Cooldown"
              value={form.cooldown_seconds}
              min={300}
              max={86400}
              step={300}
              format={(v) => (v >= 3600 ? `${v / 3600}h` : `${v / 60}m`)}
              onChange={(v) => set('cooldown_seconds', v)}
            />
          </div>
        )}

        {step === 3 && (
          <div
            style={{
              background: 'rgba(0,0,0,0.2)',
              borderRadius: 16,
              padding: '18px 20px',
              display: 'flex',
              flexDirection: 'column',
              gap: 12,
            }}
          >
            {(
              [
                ['Trigger', triggerSummary],
                ['Protocol', form.protocol ?? 'All'],
                ['Action', form.action_kind],
                ...(form.action_kind !== 'NotifyOnly'
                  ? ([
                      ['Token', form.action_token],
                      ['Amount', fmtUsd(form.action_amount_usd)],
                    ] as [string, string][])
                  : []),
                ['Max/action', fmtUsd(form.max_usd_per_action)],
                ['Daily limit', fmtUsd(form.daily_limit_usd)],
                [
                  'Cooldown',
                  form.cooldown_seconds >= 3600
                    ? `${form.cooldown_seconds / 3600}h`
                    : `${form.cooldown_seconds / 60}m`,
                ],
              ] as [string, string][]
            ).map(([k, v]) => (
              <div key={k} style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span
                  style={{
                    fontFamily: "'Inter', sans-serif",
                    fontSize: 13,
                    color: 'rgba(245,244,239,0.4)',
                  }}
                >
                  {k}
                </span>
                <span
                  style={{
                    fontFamily: "'JetBrains Mono', monospace",
                    fontSize: 13,
                    color: '#F5F4EF',
                  }}
                >
                  {v}
                </span>
              </div>
            ))}
          </div>
        )}

        <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 28 }}>
          <button
            onClick={step === 0 ? onClose : () => setStep((s) => s - 1)}
            style={{
              background: 'rgba(255,255,255,0.06)',
              border: '1px solid rgba(255,255,255,0.1)',
              borderRadius: 100,
              padding: '10px 22px',
              cursor: 'pointer',
              fontFamily: "'Inter', sans-serif",
              fontSize: 13,
              color: 'rgba(245,244,239,0.6)',
            }}
          >
            {step === 0 ? 'Cancel' : 'Back'}
          </button>
          <button
            onClick={step === 3 ? () => onSave(form) : () => setStep((s) => s + 1)}
            disabled={locked && step === 1}
            style={{
              background: locked && step === 1 ? 'rgba(217,119,87,0.3)' : '#D97757',
              border: 'none',
              borderRadius: 100,
              padding: '10px 28px',
              cursor: locked && step === 1 ? 'not-allowed' : 'pointer',
              fontFamily: "'Inter', sans-serif",
              fontSize: 13,
              fontWeight: 600,
              color: '#1F1E1D',
            }}
          >
            {step === 3 ? 'Activate Rule' : 'Continue'}
          </button>
        </div>
      </div>
    </div>
  );
}

function WizardSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: [string, string][];
}) {
  return (
    <div>
      <div
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 12,
          color: 'rgba(245,244,239,0.5)',
          marginBottom: 8,
        }}
      >
        {label}
      </div>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        style={{
          width: '100%',
          background: 'rgba(0,0,0,0.3)',
          border: '1px solid rgba(255,255,255,0.12)',
          borderRadius: 12,
          padding: '11px 14px',
          fontFamily: "'Inter', sans-serif",
          fontSize: 14,
          color: '#F5F4EF',
          appearance: 'none',
          cursor: 'pointer',
        }}
      >
        {options.map(([v, l]) => (
          <option key={v} value={v} style={{ background: '#2A2826' }}>
            {l}
          </option>
        ))}
      </select>
    </div>
  );
}

function WizardSlider({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
        <span
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: 12,
            color: 'rgba(245,244,239,0.5)',
          }}
        >
          {label}
        </span>
        <span
          style={{
            fontFamily: "'JetBrains Mono', monospace",
            fontSize: 13,
            color: '#D97757',
          }}
        >
          {format(value)}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        style={{ width: '100%', accentColor: '#D97757', cursor: 'pointer' }}
      />
    </div>
  );
}
