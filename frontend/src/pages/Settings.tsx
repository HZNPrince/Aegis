import { useWallet } from '@solana/wallet-adapter-react';
import { motion } from 'framer-motion';
import { useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { DEMO_MODE } from '../api';
import { Button, Card, Chip, Eyebrow, Reveal, tokens } from '../components/sonar';
import {
  useCreateTelegramLinkCode,
  useStatus,
  useUnlinkTelegram,
  useWalletSettings,
} from '../hooks';
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

  const settingsQ = useWalletSettings(connectedAddr);

  // Telegram bot connect
  const createCode = useCreateTelegramLinkCode();
  const unlink = useUnlinkTelegram();
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (connectedAddr && !DEMO_MODE && !settingsQ.data?.telegram_chat_id && !createCode.data && !createCode.isPending) {
      createCode.mutate(connectedAddr);
    }
  }, [createCode, settingsQ.data?.telegram_chat_id, connectedAddr]);

  const isBotConnected = !!settingsQ.data?.telegram_chat_id;
  const code = useMemo(() => createCode.data?.code ?? 'AEG-7K3M-9Q2X', [createCode.data]);
  const deepLink = createCode.data?.deep_link ?? 'https://t.me/AegisBot';

  const copyCode = () => {
    void navigator.clipboard?.writeText(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };


  return (
    <main style={{ padding: '64px 28px 72px', maxWidth: 720, margin: '0 auto' }}>
      <Reveal>
        <h1 style={{ fontFamily: tokens.sans, fontSize: 32, fontWeight: 700, letterSpacing: '-0.025em', margin: 0 }}>
          Settings
        </h1>
        <p style={{ fontFamily: tokens.sans, fontSize: 15, color: tokens.ink2, marginTop: 6, marginBottom: 32 }}>
          Manage your wallet, notifications, and integrations.
        </p>
      </Reveal>

      {/* ─── Wallet ─── */}
      <SettingsSection title="Wallet">
        <div style={{ display: 'flex', gap: 14, alignItems: 'center', paddingBottom: 16, borderBottom: `1px solid ${tokens.lineSoft}` }}>
          <div style={{ width: 36, height: 36, borderRadius: '50%', background: 'rgba(90,107,71,0.12)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 1L14 4V12L8 15L2 12V4L8 1Z" stroke="var(--moss)" strokeWidth="1.5" fill="none"/></svg>
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontFamily: tokens.mono, fontSize: 13, fontWeight: 500 }}>{truncAddr(displayAddr)}</div>
            <div style={{ fontFamily: tokens.sans, fontSize: 11, color: tokens.ink2, marginTop: 2 }}>
              {walletName} · {connectedAddr ? 'Connected' : 'Demo'}
            </div>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <Button size="sm" onClick={() => void navigator.clipboard?.writeText(displayAddr)}>Copy</Button>
            <Button size="sm" variant="danger" onClick={onDisconnect}>Disconnect</Button>
          </div>
        </div>
        <div style={{ paddingTop: 14, fontFamily: tokens.mono, fontSize: 12, color: tokens.ink2 }}>
          {truncAddr(displayAddr)} · {status.positions_cached} positions · monitoring {status.wallets_monitored > 0 ? 'active' : 'idle'}
        </div>
      </SettingsSection>

      {/* ─── Telegram Bot ─── */}
      <SettingsSection title="Telegram Bot">
        <div style={{ display: 'flex', alignItems: 'start', justifyContent: 'space-between', gap: 16, marginBottom: isBotConnected ? 0 : 20 }}>
          <div>
            <div style={{ fontFamily: tokens.sans, fontSize: 15, fontWeight: 600 }}>@AegisBot</div>
            <p style={{ fontFamily: tokens.sans, fontSize: 13, color: tokens.ink2, marginTop: 4, lineHeight: 1.5 }}>
              {isBotConnected
                ? `Linked — alerts for ${truncAddr(displayAddr)} are sent to your Telegram.`
                : `Open the bot and send the one-time code to link your wallet (${truncAddr(displayAddr)}).`}
            </p>
          </div>
          <Chip tone={isBotConnected ? 'healthy' : 'neutral'}>
            {isBotConnected ? 'Connected' : 'Not connected'}
          </Chip>
        </div>

        {isBotConnected ? (
          <div style={{ marginTop: 16, display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
            <a href={deepLink} target="_blank" rel="noreferrer" style={{ textDecoration: 'none' }}>
              <Button size="sm">Open bot</Button>
            </a>
            {connectedAddr ? (
              <Button variant="danger" size="sm" disabled={unlink.isPending} onClick={() => unlink.mutate(connectedAddr)}>
                {unlink.isPending ? 'Unlinking…' : 'Unlink'}
              </Button>
            ) : null}
          </div>
        ) : (
          <>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr auto auto', gap: 10 }}>
              <div style={{
                minHeight: 44,
                display: 'flex',
                alignItems: 'center',
                padding: '0 14px',
                border: `1px solid ${tokens.lineSoft}`,
                borderRadius: 8,
                background: tokens.paper2,
                fontFamily: tokens.mono,
                fontSize: 16,
                letterSpacing: '0.04em',
              }}>
                {createCode.isPending ? 'Generating…' : code}
              </div>
              <Button onClick={copyCode}>{copied ? 'Copied' : 'Copy'}</Button>
              <a href={deepLink} target="_blank" rel="noreferrer" style={{ textDecoration: 'none' }}>
                <Button variant="accent">Open bot</Button>
              </a>
            </div>
            <p style={{ fontFamily: tokens.sans, color: tokens.ink2, fontSize: 12, marginTop: 14 }}>
              {createCode.error
                ? 'Preview code shown because the API did not return a live code.'
                : 'Codes expire automatically. This page polls until Telegram confirms the link.'}
            </p>
          </>
        )}
      </SettingsSection>

      {/* ─── System Status ─── */}
      <SettingsSection title="System Status">
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          {Object.entries(status).map(([k, v]) => (
            <div key={k} style={{ background: tokens.paper2, borderRadius: 8, padding: '12px 14px' }}>
              <Eyebrow>{k.replace(/_/g, ' ')}</Eyebrow>
              <div style={{ fontFamily: tokens.mono, fontSize: 16, fontWeight: 600, marginTop: 6, color: tokens.moss }}>
                {v}
              </div>
            </div>
          ))}
        </div>
      </SettingsSection>
    </main>
  );
}

function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4 }}
      style={{ marginBottom: 24 }}
    >
      <Eyebrow style={{ marginBottom: 12 }}>{title}</Eyebrow>
      <Card pad={22}>{children}</Card>
    </motion.div>
  );
}
