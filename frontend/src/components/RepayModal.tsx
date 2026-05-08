import { useWallet } from '@solana/wallet-adapter-react';
import { VersionedTransaction } from '@solana/web3.js';
import { useEffect, useMemo, useState } from 'react';
import { useCreateRepayIntent, useSubmitIntent } from '../hooks';
import type { Position } from '../types';
import { fmtUsd, healthColor } from '../utils';

interface Props {
  position: Position;
  /** Total debt USD on the wallet — used to project new LTV/health. */
  totalDebtUsd: number;
  totalCollateralUsd: number;
  liquidationThreshold: number;
  onClose: () => void;
}

const QUICK_FILLS: { label: string; pct: number }[] = [
  { label: '25%', pct: 0.25 },
  { label: '50%', pct: 0.5 },
  { label: 'Max', pct: 1 },
];

// Format a token amount with a sensible decimal count per symbol — 0.500500
// reads as 0.5005 with trailing zeros stripped.
function fmtTokenAmount(n: number, symbol: string): string {
  const stable = ['USDC', 'USDT', 'PYUSD', 'USDS', 'USDH'].includes(symbol.toUpperCase());
  const decimals = stable ? 4 : 6;
  if (!Number.isFinite(n)) return '0';
  return Number(n.toFixed(decimals))
    .toString()
    .replace(/(\.\d*?[1-9])0+$/, '$1')
    .replace(/\.0+$/, '');
}

const PROTOCOL_COPY: Record<string, { authority: string; tone: string }> = {
  Kamino: {
    authority: 'Permissionless repay — anyone can repay this obligation.',
    tone: 'You sign with your wallet to fund the repay; obligation authority is not required.',
  },
  Save: {
    authority: 'Permissionless repay — anyone can repay this obligation.',
    tone: 'You sign with your wallet to fund the repay; obligation authority is not required.',
  },
  Marginfi: {
    authority: 'Authority-only repay — your wallet must be the marginfi account authority.',
    tone: 'If signing fails, the connected wallet does not own this account.',
  },
};

const isBase64Tx = (s: string) => /^[A-Za-z0-9+/=]+$/.test(s);

const b64decode = (s: string): Uint8Array => {
  const bin = atob(s);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
};

const b64encode = (bytes: Uint8Array): string => {
  let s = '';
  for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
  return btoa(s);
};

export function RepayModal({
  position,
  totalDebtUsd,
  totalCollateralUsd,
  liquidationThreshold,
  onClose,
}: Props) {
  const { publicKey, signTransaction } = useWallet();
  const createIntent = useCreateRepayIntent();
  const submit = useSubmitIntent();

  const [amount, setAmount] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [phase, setPhase] = useState<'idle' | 'building' | 'signing' | 'submitting' | 'done'>(
    'idle',
  );
  const [signature, setSignature] = useState<string | null>(null);

  useEffect(() => {
    setAmount(fmtTokenAmount(position.amount, position.asset_symbol));
  }, [position.amount, position.asset_symbol]);

  const amountNum = Number.parseFloat(amount || '0');
  const validAmount = Number.isFinite(amountNum) && amountNum > 0 && amountNum <= position.amount;

  // Convert UI amount → native (pre-decimal) units. We don't carry decimals on
  // Position, so derive from amount_native/amount_ui. amount_native is u64 native units.
  const amountNative = useMemo(() => {
    if (!validAmount) return 0n;
    if (!position.amount_native || position.amount === 0) return 0n;
    const ratio = amountNum / position.amount;
    // Round to nearest u64 native unit; clamp at the leg's native debt.
    const native = BigInt(Math.round(Number(position.amount_native) * ratio));
    const cap = BigInt(position.amount_native);
    return native > cap ? cap : native;
  }, [amountNum, position.amount, position.amount_native, validAmount]);

  // Project resulting health using the same threshold used elsewhere.
  const projection = useMemo(() => {
    if (!validAmount || totalCollateralUsd <= 0) return null;
    const usdRepaid = (amountNum / position.amount) * position.value_usd;
    const newDebt = Math.max(0, totalDebtUsd - usdRepaid);
    const newLtv = newDebt / totalCollateralUsd;
    const newHealth = Math.max(
      0,
      Math.min(100, (1 - newLtv / liquidationThreshold) * 100),
    );
    const oldLtv = totalDebtUsd / totalCollateralUsd;
    const oldHealth = Math.max(
      0,
      Math.min(100, (1 - oldLtv / liquidationThreshold) * 100),
    );
    return { usdRepaid, newDebt, newLtv, newHealth, oldHealth };
  }, [amountNum, totalDebtUsd, totalCollateralUsd, liquidationThreshold, position, validAmount]);

  const protoCopy = PROTOCOL_COPY[position.protocol] ?? PROTOCOL_COPY.Kamino;

  const onConfirm = async () => {
    setError(null);
    if (!publicKey) {
      setError('Connect a wallet first.');
      return;
    }
    if (!signTransaction) {
      setError('Connected wallet does not support transaction signing.');
      return;
    }
    if (!position.reserve_or_bank || !position.asset_mint) {
      setError('Position is missing on-chain identifiers; cannot build a repay.');
      return;
    }
    if (!validAmount || amountNative === 0n) {
      setError('Enter a positive amount no greater than your outstanding debt.');
      return;
    }

    try {
      setPhase('building');
      const intent = await createIntent.mutateAsync({
        wallet: publicKey.toBase58(),
        position_pubkey: position.obligation_address,
        protocol: position.protocol,
        reserve_or_bank: position.reserve_or_bank,
        mint: position.asset_mint,
        amount_native: amountNative.toString(),
      });

      if (!isBase64Tx(intent.tx_base64)) {
        throw new Error('Backend returned an invalid transaction blob.');
      }

      setPhase('signing');
      const txBytes = b64decode(intent.tx_base64);
      const tx = VersionedTransaction.deserialize(txBytes);
      const signed = await signTransaction(tx);
      const signedBytes = signed.serialize();
      const signedB64 = b64encode(signedBytes);

      setPhase('submitting');
      const { signature: sig } = await submit.mutateAsync({
        intentId: intent.intent_id,
        signedTxBase64: signedB64,
      });
      setSignature(sig);
      setPhase('done');
    } catch (e) {
      setPhase('idle');
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const busy = phase !== 'idle' && phase !== 'done';
  const phaseLabel: Record<typeof phase, string> = {
    idle: 'Sign & Submit',
    building: 'Building transaction…',
    signing: 'Awaiting signature…',
    submitting: 'Submitting to network…',
    done: 'Submitted ✓',
  };

  return (
    <div
      onClick={onClose}
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
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'linear-gradient(180deg, var(--surface-2) 0%, var(--surface-1) 100%)',
          borderRadius: '20px 20px 0 0',
          border: `1px solid ${'color-mix(in oklab, var(--ink) 14%, transparent)'}`,
          borderBottom: 'none',
          width: '100%',
          maxWidth: 560,
          padding: '28px 28px 36px',
          boxShadow: 'var(--shadow-lg)',
          animation: 'slideUp 0.32s cubic-bezier(0.34,1.56,0.64,1) both',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 22 }}>
          <div>
            <div
              style={{
                fontFamily: 'var(--sans)',
                fontSize: 20,
                fontWeight: 700,
                color: 'var(--ink)',
                letterSpacing: '-0.015em',
                display: 'inline-flex',
                alignItems: 'center',
                gap: 8,
              }}
            >
              Repay
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 6,
                  padding: '3px 10px',
                  borderRadius: 999,
                  background: 'color-mix(in oklab, var(--rust) 14%, transparent)',
                  border: '1px solid color-mix(in oklab, var(--rust) 36%, transparent)',
                  color: 'var(--rust)',
                  fontFamily: 'var(--mono)',
                  fontSize: 12,
                  fontWeight: 600,
                  letterSpacing: '0.02em',
                }}
              >
                {position.asset_symbol}
              </span>
            </div>
            <div
              style={{
                fontFamily: 'var(--mono)',
                fontSize: 12,
                color: 'color-mix(in oklab, var(--ink) 55%, transparent)',
                marginTop: 8,
                letterSpacing: '0.02em',
              }}
            >
              {position.protocol} · outstanding {fmtTokenAmount(position.amount, position.asset_symbol)} {position.asset_symbol} · {fmtUsd(position.value_usd)}
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={busy}
            aria-label="Close"
            style={{
              background: 'color-mix(in oklab, var(--ink) 6%, transparent)',
              border: '1px solid color-mix(in oklab, var(--ink) 14%, transparent)',
              borderRadius: 999,
              width: 32,
              height: 32,
              cursor: busy ? 'not-allowed' : 'pointer',
              color: 'color-mix(in oklab, var(--ink) 60%, transparent)',
              fontSize: 14,
              flexShrink: 0,
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              transition: 'background 0.15s, color 0.15s',
            }}
            onMouseEnter={(e) => { if (!busy) { e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 12%, transparent)'; e.currentTarget.style.color = 'var(--ink)'; } }}
            onMouseLeave={(e) => { e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 6%, transparent)'; e.currentTarget.style.color = 'color-mix(in oklab, var(--ink) 60%, transparent)'; }}
          >
            ✕
          </button>
        </div>

        {phase === 'done' && signature ? (
          <div
            style={{
              padding: 24,
              borderRadius: 16,
              background: 'color-mix(in oklab, var(--moss) 14%, transparent)',
              border: '1px solid color-mix(in oklab, var(--moss) 45%, transparent)',
              textAlign: 'center',
            }}
          >
            <div
              style={{
                fontFamily: "var(--serif)",
                fontSize: 18,
                color: "var(--ink)",
                marginBottom: 8,
              }}
            >
              Repay submitted
            </div>
            <div
              style={{
                fontFamily: "var(--sans)",
                fontSize: 13,
                color: 'color-mix(in oklab, var(--ink) 50%, transparent)',
                marginBottom: 16,
              }}
            >
              Network confirmation usually takes a few seconds.
            </div>
            <a
              href={`https://explorer.solana.com/tx/${signature}`}
              target="_blank"
              rel="noreferrer"
              style={{
                fontFamily: "var(--mono)",
                fontSize: 12,
                color: "var(--rust)",
                textDecoration: 'none',
                wordBreak: 'break-all',
              }}
            >
              {signature.slice(0, 12)}…{signature.slice(-12)} ↗
            </a>
            <div style={{ marginTop: 18 }}>
              <button
                onClick={onClose}
                style={{
                  background: "var(--rust)",
                  border: 'none',
                  borderRadius: 100,
                  padding: '11px 28px',
                  cursor: 'pointer',
                  fontFamily: "var(--sans)",
                  fontSize: 13,
                  fontWeight: 600,
                  color: '#1F1E1D',
                }}
              >
                Done
              </button>
            </div>
          </div>
        ) : (
          <>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 8 }}>
              <Label>Amount to repay ({position.asset_symbol})</Label>
              <button
                type="button"
                onClick={() => setAmount(fmtTokenAmount(position.amount, position.asset_symbol))}
                disabled={busy}
                style={{
                  background: 'transparent',
                  border: 'none',
                  fontFamily: "var(--mono)",
                  fontSize: 11,
                  color: 'color-mix(in oklab, var(--ink) 55%, transparent)',
                  cursor: busy ? 'not-allowed' : 'pointer',
                  padding: 0,
                  marginBottom: 8,
                }}
              >
                Bal&nbsp;{fmtTokenAmount(position.amount, position.asset_symbol)}
              </button>
            </div>
            <input
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              type="number"
              step="any"
              min="0"
              max={position.amount}
              disabled={busy}
              style={{
                width: '100%',
                background: 'color-mix(in oklab, var(--ink) 4%, transparent)',
                border: `1px solid ${validAmount ? 'color-mix(in oklab, var(--ink) 16%, transparent)' : 'color-mix(in oklab, var(--rust) 50%, transparent)'}`,
                borderRadius: 12,
                padding: '14px 16px',
                fontFamily: 'var(--mono)',
                fontSize: 18,
                fontWeight: 600,
                color: 'var(--ink)',
                outline: 'none',
                marginBottom: 10,
                letterSpacing: '0.005em',
                transition: 'border-color 0.15s, box-shadow 0.15s',
              }}
              onFocus={(e) => { e.currentTarget.style.boxShadow = '0 0 0 3px color-mix(in oklab, var(--cobalt) 20%, transparent)'; e.currentTarget.style.borderColor = 'color-mix(in oklab, var(--cobalt) 50%, transparent)'; }}
              onBlur={(e) => { e.currentTarget.style.boxShadow = 'none'; e.currentTarget.style.borderColor = validAmount ? 'color-mix(in oklab, var(--ink) 16%, transparent)' : 'color-mix(in oklab, var(--rust) 50%, transparent)'; }}
            />

            {/* Utilization meter — visualizes how much of the debt is being repaid */}
            <div style={{ height: 4, borderRadius: 999, background: 'color-mix(in oklab, var(--ink) 8%, transparent)', overflow: 'hidden', marginBottom: 14 }}>
              <div
                style={{
                  height: '100%',
                  width: `${Math.min(100, Math.max(0, (amountNum / Math.max(position.amount, 1e-9)) * 100))}%`,
                  background: 'linear-gradient(90deg, #D97757 0%, #E8A47C 100%)',
                  transition: 'width 0.18s ease',
                }}
              />
            </div>

            <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
              {QUICK_FILLS.map(({ label, pct }) => {
                const target = position.amount * pct;
                // active when the typed amount is within rounding distance of this preset
                const active = Math.abs(amountNum - target) / Math.max(position.amount, 1e-9) < 0.001;
                return (
                  <button
                    key={label}
                    onClick={() => setAmount(fmtTokenAmount(target, position.asset_symbol))}
                    disabled={busy}
                    style={{
                      flex: 1,
                      background: active ? 'color-mix(in oklab, var(--rust) 16%, transparent)' : 'color-mix(in oklab, var(--ink) 4%, transparent)',
                      border: `1px solid ${active ? 'color-mix(in oklab, var(--rust) 55%, transparent)' : 'color-mix(in oklab, var(--ink) 12%, transparent)'}`,
                      borderRadius: 10,
                      padding: '10px 0',
                      cursor: busy ? 'not-allowed' : 'pointer',
                      fontFamily: 'var(--sans)',
                      fontSize: 13,
                      fontWeight: active ? 600 : 500,
                      color: active ? 'var(--rust)' : 'color-mix(in oklab, var(--ink) 75%, transparent)',
                      transition: 'background 0.15s, border-color 0.15s, color 0.15s',
                    }}
                    onMouseEnter={(e) => { if (!busy && !active) e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 8%, transparent)'; }}
                    onMouseLeave={(e) => { if (!active) e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 4%, transparent)'; }}
                  >
                    {label}
                  </button>
                );
              })}
            </div>

            {projection && (
              <div
                style={{
                  background: 'color-mix(in oklab, var(--ink) 5%, transparent)',
                  border: '1px solid color-mix(in oklab, var(--ink) 10%, transparent)',
                  borderRadius: 14,
                  padding: '16px 18px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 10,
                  marginBottom: 16,
                }}
              >
                <Row label="USD repaid" value={fmtUsd(projection.usdRepaid)} />
                <Row label="New total debt" value={fmtUsd(projection.newDebt)} />
                <Row label="New LTV" value={`${(projection.newLtv * 100).toFixed(1)}%`} />
                <Row
                  label="Projected health"
                  value={
                    <span
                      style={{
                        color: healthColor(projection.newHealth),
                        fontWeight: 700,
                      }}
                    >
                      {projection.oldHealth.toFixed(0)} → {projection.newHealth.toFixed(0)}
                    </span>
                  }
                />
              </div>
            )}

            <div
              style={{
                fontFamily: 'var(--sans)',
                fontSize: 12,
                color: 'color-mix(in oklab, var(--ink) 50%, transparent)',
                lineHeight: 1.55,
                marginBottom: 18,
                paddingLeft: 12,
                borderLeft: '2px solid color-mix(in oklab, var(--ink) 16%, transparent)',
              }}
            >
              <div style={{ color: 'color-mix(in oklab, var(--ink) 78%, transparent)', marginBottom: 4, fontWeight: 600 }}>
                {protoCopy.authority}
              </div>
              {protoCopy.tone}
            </div>

            {error && (
              <div
                style={{
                  background: 'color-mix(in oklab, var(--rust) 10%, transparent)',
                  border: '1px solid color-mix(in oklab, var(--rust) 40%, transparent)',
                  borderRadius: 10,
                  padding: '10px 14px',
                  fontFamily: 'var(--mono)',
                  fontSize: 11,
                  color: 'var(--rust)',
                  marginBottom: 14,
                  whiteSpace: 'pre-wrap',
                  maxHeight: 200,
                  overflow: 'auto',
                  lineHeight: 1.5,
                }}
              >
                {error}
              </div>
            )}

            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 10 }}>
              <button
                onClick={onClose}
                disabled={busy}
                style={{
                  background: 'color-mix(in oklab, var(--ink) 5%, transparent)',
                  border: '1px solid color-mix(in oklab, var(--ink) 14%, transparent)',
                  borderRadius: 999,
                  padding: '11px 22px',
                  cursor: busy ? 'not-allowed' : 'pointer',
                  fontFamily: 'var(--sans)',
                  fontSize: 13,
                  fontWeight: 500,
                  color: 'color-mix(in oklab, var(--ink) 70%, transparent)',
                  transition: 'background 0.15s',
                }}
                onMouseEnter={(e) => { if (!busy) e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 10%, transparent)'; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = 'color-mix(in oklab, var(--ink) 5%, transparent)'; }}
              >
                Cancel
              </button>
              <button
                onClick={onConfirm}
                disabled={busy || !validAmount}
                style={{
                  background:
                    busy || !validAmount
                      ? 'color-mix(in oklab, var(--rust) 40%, transparent)'
                      : 'var(--rust)',
                  border: '1px solid var(--rust-ink)',
                  borderRadius: 999,
                  padding: '11px 28px',
                  cursor: busy || !validAmount ? 'not-allowed' : 'pointer',
                  fontFamily: 'var(--sans)',
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--paper)',
                  boxShadow: busy || !validAmount ? 'none' : '0 4px 14px color-mix(in oklab, var(--rust) 35%, transparent)',
                  transition: 'background 0.15s, box-shadow 0.15s, transform 0.12s',
                }}
                onMouseEnter={(e) => { if (!busy && validAmount) e.currentTarget.style.transform = 'translateY(-1px)'; }}
                onMouseLeave={(e) => { e.currentTarget.style.transform = 'translateY(0)'; }}
              >
                {phaseLabel[phase]}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function Label({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        fontFamily: "var(--sans)",
        fontSize: 11,
        color: 'color-mix(in oklab, var(--ink) 40%, transparent)',
        letterSpacing: '0.07em',
        textTransform: 'uppercase',
        marginBottom: 8,
      }}
    >
      {children}
    </div>
  );
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
      <span
        style={{
          fontFamily: "var(--sans)",
          fontSize: 12,
          color: 'color-mix(in oklab, var(--ink) 45%, transparent)',
        }}
      >
        {label}
      </span>
      <span
        style={{
          fontFamily: "var(--mono)",
          fontSize: 13,
          color: "var(--ink)",
        }}
      >
        {value}
      </span>
    </div>
  );
}
