import { useWallet } from '@solana/wallet-adapter-react';
import { motion } from 'framer-motion';
import { useState } from 'react';
import type { ReactNode } from 'react';
import { DEMO_MODE } from '../api';
import { ShieldIcon } from '../components/ShieldIcon';
import { Card, SectionLabel } from '../components/ui';
import { useStatus } from '../hooks';
import { MOCK_STATUS, MOCK_WALLET_FULL } from '../mockData';
import { truncAddr } from '../utils';

interface Props {
  onDisconnect: () => void;
}

export function Settings({ onDisconnect }: Props) {
  const { publicKey, wallet: walletAdapter } = useWallet();
  const connectedAddr = publicKey?.toBase58() ?? null;
  const walletName = walletAdapter?.adapter.name ?? 'Wallet';
  const displayAddr = connectedAddr ?? MOCK_WALLET_FULL;
  const statusQ = useStatus();
  const status = !DEMO_MODE && statusQ.data ? statusQ.data : MOCK_STATUS;

  const [telegramId, setTelegramId] = useState('');
  const [email, setEmail] = useState('');
  const [savedTg, setSavedTg] = useState(false);
  const [savedEmail, setSavedEmail] = useState(false);

  const saveTg = () => {
    setSavedTg(true);
    setTimeout(() => setSavedTg(false), 2000);
  };
  const saveEmail = () => {
    setSavedEmail(true);
    setTimeout(() => setSavedEmail(false), 2000);
  };

  return (
    <div style={{ padding: '88px 28px 60px', maxWidth: 640, margin: '0 auto' }}>
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
        Settings
      </h2>
      <p
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 14,
          color: 'rgba(245,244,239,0.4)',
          marginBottom: 32,
        }}
      >
        Manage your wallet, notifications, and subscription.
      </p>

      <SettingsSection title="Wallet">
        <div
          style={{
            display: 'flex',
            gap: 14,
            alignItems: 'center',
            padding: '14px 0',
            borderBottom: '1px solid rgba(255,255,255,0.06)',
          }}
        >
          <div
            style={{
              width: 36,
              height: 36,
              borderRadius: '50%',
              background: 'rgba(217,119,87,0.15)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <ShieldIcon size={18} />
          </div>
          <div style={{ flex: 1 }}>
            <div
              style={{
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: 13,
                color: '#F5F4EF',
                marginBottom: 2,
              }}
            >
              {truncAddr(displayAddr)}
            </div>
            <div
              style={{
                fontFamily: "'Inter', sans-serif",
                fontSize: 11,
                color: 'rgba(245,244,239,0.35)',
              }}
            >
              {walletName} · {connectedAddr ? 'Connected' : 'Demo'}
            </div>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              onClick={() => void navigator.clipboard?.writeText(displayAddr)}
              style={{
                background: 'rgba(255,255,255,0.06)',
                border: '1px solid rgba(255,255,255,0.1)',
                borderRadius: 100,
                padding: '6px 14px',
                cursor: 'pointer',
                fontFamily: "'Inter', sans-serif",
                fontSize: 12,
                color: 'rgba(245,244,239,0.5)',
              }}
            >
              Copy
            </button>
            <button
              onClick={onDisconnect}
              style={{
                background: 'rgba(217,96,78,0.1)',
                border: '1px solid rgba(217,96,78,0.25)',
                borderRadius: 100,
                padding: '6px 14px',
                cursor: 'pointer',
                fontFamily: "'Inter', sans-serif",
                fontSize: 12,
                color: '#D9604E',
              }}
            >
              Disconnect
            </button>
          </div>
        </div>
        <div style={{ paddingTop: 14 }}>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 12,
              color: 'rgba(245,244,239,0.35)',
              marginBottom: 10,
            }}
          >
            Monitored wallets
          </div>
          <div
            style={{
              fontFamily: "'JetBrains Mono', monospace",
              fontSize: 12,
              color: 'rgba(245,244,239,0.5)',
              background: 'rgba(0,0,0,0.2)',
              padding: '10px 14px',
              borderRadius: 10,
            }}
          >
            {truncAddr(displayAddr)} · {status.positions_cached} positions · monitoring {status.wallets_monitored > 0 ? 'active' : 'idle'}
          </div>
        </div>
      </SettingsSection>

      <SettingsSection title="Notifications">
        <div style={{ marginBottom: 20 }}>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 13,
              color: 'rgba(245,244,239,0.6)',
              marginBottom: 8,
            }}
          >
            Telegram
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <input
              value={telegramId}
              onChange={(e) => setTelegramId(e.target.value)}
              placeholder="Chat ID (e.g. 123456789)"
              style={{
                flex: 1,
                background: 'rgba(0,0,0,0.3)',
                border: '1px solid rgba(255,255,255,0.1)',
                borderRadius: 12,
                padding: '10px 14px',
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: 13,
                color: '#F5F4EF',
                outline: 'none',
              }}
            />
            <button
              onClick={saveTg}
              style={{
                background: savedTg ? '#7DA87B' : '#D97757',
                border: 'none',
                borderRadius: 12,
                padding: '10px 18px',
                cursor: 'pointer',
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                fontWeight: 600,
                color: '#1F1E1D',
                transition: 'background 0.3s',
                flexShrink: 0,
              }}
            >
              {savedTg ? 'Saved ✓' : 'Save'}
            </button>
          </div>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 11,
              color: 'rgba(245,244,239,0.25)',
              marginTop: 6,
            }}
          >
            Start a chat with @AegisAlertBot and send /start to get your chat ID.
          </div>
        </div>
        <div>
          <div
            style={{
              fontFamily: "'Inter', sans-serif",
              fontSize: 13,
              color: 'rgba(245,244,239,0.6)',
              marginBottom: 8,
            }}
          >
            Email
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <input
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="your@email.com"
              type="email"
              style={{
                flex: 1,
                background: 'rgba(0,0,0,0.3)',
                border: '1px solid rgba(255,255,255,0.1)',
                borderRadius: 12,
                padding: '10px 14px',
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                color: '#F5F4EF',
                outline: 'none',
              }}
            />
            <button
              onClick={saveEmail}
              style={{
                background: savedEmail ? '#7DA87B' : '#D97757',
                border: 'none',
                borderRadius: 12,
                padding: '10px 18px',
                cursor: 'pointer',
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                fontWeight: 600,
                color: '#1F1E1D',
                transition: 'background 0.3s',
                flexShrink: 0,
              }}
            >
              {savedEmail ? 'Saved ✓' : 'Save'}
            </button>
          </div>
        </div>
      </SettingsSection>

      <SettingsSection title="Subscription">
        <div style={{ display: 'flex', gap: 16, alignItems: 'center', marginBottom: 20 }}>
          <div style={{ flex: 1 }}>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 4 }}>
              <span
                style={{
                  fontFamily: "'Fraunces', serif",
                  fontSize: 18,
                  fontWeight: 600,
                  color: '#F5F4EF',
                }}
              >
                Free tier
              </span>
              <span
                style={{
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 11,
                  color: 'rgba(245,244,239,0.4)',
                  background: 'rgba(255,255,255,0.07)',
                  padding: '2px 8px',
                  borderRadius: 100,
                }}
              >
                Active
              </span>
            </div>
            <div
              style={{
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                color: 'rgba(245,244,239,0.4)',
              }}
            >
              Unified view, AI alerts, notify-only rules.
            </div>
          </div>
        </div>
        <div
          style={{
            background: 'rgba(217,119,87,0.06)',
            border: '1px solid rgba(217,119,87,0.18)',
            borderRadius: 16,
            padding: '18px 20px',
            display: 'flex',
            gap: 14,
            alignItems: 'center',
            boxShadow: '0 0 24px rgba(217,119,87,0.06)',
          }}
        >
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
              Aegis Pro
            </div>
            <div
              style={{
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                color: 'rgba(245,244,239,0.45)',
              }}
            >
              Autonomous execution · Unlimited rules · Priority alerts
            </div>
            <div
              style={{
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: 14,
                color: '#D97757',
                marginTop: 6,
                fontWeight: 600,
              }}
            >
              $19 / month
            </div>
          </div>
          <button
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
            Upgrade
          </button>
        </div>
      </SettingsSection>

      <SettingsSection title="API Key">
        <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
          <div
            style={{
              flex: 1,
              background: 'rgba(0,0,0,0.2)',
              border: '1px solid rgba(255,255,255,0.06)',
              borderRadius: 10,
              padding: '10px 14px',
              fontFamily: "'JetBrains Mono', monospace",
              fontSize: 12,
              color: 'rgba(245,244,239,0.25)',
              letterSpacing: '0.05em',
            }}
          >
            ••••••••••••••••••••••••  (coming soon)
          </div>
          <button
            disabled
            style={{
              background: 'rgba(255,255,255,0.05)',
              border: '1px solid rgba(255,255,255,0.08)',
              borderRadius: 10,
              padding: '10px 16px',
              cursor: 'not-allowed',
              fontFamily: "'Inter', sans-serif",
              fontSize: 12,
              color: 'rgba(245,244,239,0.25)',
            }}
          >
            Generate
          </button>
        </div>
      </SettingsSection>

      <SettingsSection title="System Status">
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          {Object.entries(status).map(([k, v]) => (
            <div
              key={k}
              style={{
                background: 'rgba(0,0,0,0.2)',
                borderRadius: 10,
                padding: '10px 14px',
              }}
            >
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
                {k.replace(/_/g, ' ')}
              </div>
              <div
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: 14,
                  color: '#7DA87B',
                  fontWeight: 600,
                }}
              >
                {v}
              </div>
            </div>
          ))}
        </div>
      </SettingsSection>
    </div>
  );
}

function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4 }}
      style={{ marginBottom: 28 }}
    >
      <SectionLabel>{title}</SectionLabel>
      <Card style={{ padding: '20px 22px' }}>{children}</Card>
    </motion.div>
  );
}
