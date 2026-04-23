import { useState } from 'react';
import { ShieldIcon } from '../components/ShieldIcon';

interface Props {
  onConnect: () => void;
}

const WALLETS = ['Phantom', 'Backpack', 'Solflare'];

const FEATURES: { icon: string; title: string; desc: string }[] = [
  { icon: '⬡', title: 'Unified view', desc: 'All protocols. One number.' },
  { icon: '◈', title: 'AI alerts', desc: 'LLM-powered risk summaries.' },
  { icon: '◎', title: 'Autonomous guardrails', desc: 'Rules that act before you can.' },
];

export function Landing({ onConnect }: Props) {
  const [hovering, setHovering] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [walletMenu, setWalletMenu] = useState(false);

  const handleConnect = () => {
    setWalletMenu(false);
    setConnecting(true);
    setTimeout(() => {
      setConnecting(false);
      onConnect();
    }, 1400);
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        background: '#1F1E1D',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '80px 24px 60px',
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          position: 'absolute',
          top: '20%',
          left: '50%',
          transform: 'translateX(-50%)',
          width: 600,
          height: 600,
          borderRadius: '50%',
          background:
            'radial-gradient(circle, rgba(217,119,87,0.08) 0%, transparent 70%)',
          pointerEvents: 'none',
        }}
      />

      <div style={{ marginBottom: 32, animation: 'fadeUp 0.7s ease both' }}>
        <ShieldIcon size={52} />
      </div>

      <div
        style={{
          fontFamily: "'Fraunces', serif",
          fontSize: 15,
          fontWeight: 400,
          color: 'rgba(245,244,239,0.4)',
          letterSpacing: '0.22em',
          textTransform: 'uppercase',
          marginBottom: 28,
          animation: 'fadeUp 0.7s 0.05s ease both',
        }}
      >
        Aegis
      </div>

      <h1
        style={{
          fontFamily: "'Fraunces', serif",
          fontSize: 'clamp(36px, 6vw, 68px)',
          fontWeight: 600,
          color: '#F5F4EF',
          textAlign: 'center',
          lineHeight: 1.1,
          letterSpacing: '-0.03em',
          maxWidth: 700,
          margin: '0 0 20px',
          animation: 'fadeUp 0.7s 0.1s ease both',
        }}
      >
        Never get liquidated
        <br />
        <span style={{ color: '#D97757' }}>in your sleep.</span>
      </h1>

      <p
        style={{
          fontFamily: "'Inter', sans-serif",
          fontSize: 17,
          color: 'rgba(245,244,239,0.45)',
          textAlign: 'center',
          maxWidth: 460,
          lineHeight: 1.6,
          margin: '0 0 52px',
          fontWeight: 400,
          animation: 'fadeUp 0.7s 0.15s ease both',
        }}
      >
        Unified Solana lending dashboard. Live risk scores across Kamino, Save & Marginfi.
        Autonomous guard rails that act while you sleep.
      </p>

      <div style={{ position: 'relative', animation: 'fadeUp 0.7s 0.2s ease both' }}>
        <button
          onClick={() => setWalletMenu(!walletMenu)}
          onMouseEnter={() => setHovering(true)}
          onMouseLeave={() => setHovering(false)}
          disabled={connecting}
          style={{
            background: connecting ? 'rgba(217,119,87,0.6)' : '#D97757',
            border: 'none',
            cursor: connecting ? 'default' : 'pointer',
            padding: '16px 40px',
            borderRadius: 100,
            fontFamily: "'Inter', sans-serif",
            fontSize: 15,
            fontWeight: 600,
            color: '#1F1E1D',
            letterSpacing: '0.01em',
            transition: 'all 0.2s',
            transform: hovering && !connecting ? 'translateY(-1px)' : 'none',
            boxShadow:
              hovering && !connecting
                ? '0 8px 32px rgba(217,119,87,0.35)'
                : '0 4px 16px rgba(217,119,87,0.2)',
          }}
        >
          {connecting ? 'Connecting…' : 'Connect Wallet'}
        </button>

        {walletMenu && !connecting && (
          <div
            style={{
              position: 'absolute',
              top: 'calc(100% + 10px)',
              left: '50%',
              transform: 'translateX(-50%)',
              background: '#2A2826',
              border: '1px solid rgba(255,255,255,0.1)',
              borderRadius: 16,
              padding: 8,
              boxShadow: '0 16px 48px rgba(0,0,0,0.5)',
              minWidth: 200,
              zIndex: 10,
            }}
          >
            {WALLETS.map((w) => (
              <button
                key={w}
                onClick={handleConnect}
                style={{
                  display: 'block',
                  width: '100%',
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  padding: '11px 16px',
                  borderRadius: 10,
                  textAlign: 'left',
                  fontFamily: "'Inter', sans-serif",
                  fontSize: 14,
                  fontWeight: 500,
                  color: '#F5F4EF',
                  transition: 'background 0.15s',
                }}
                onMouseEnter={(e) =>
                  (e.currentTarget.style.background = 'rgba(255,255,255,0.06)')
                }
                onMouseLeave={(e) => (e.currentTarget.style.background = 'none')}
              >
                {w}
              </button>
            ))}
          </div>
        )}
      </div>

      <div
        style={{
          display: 'flex',
          gap: 16,
          marginTop: 72,
          flexWrap: 'wrap',
          justifyContent: 'center',
          animation: 'fadeUp 0.7s 0.3s ease both',
        }}
      >
        {FEATURES.map(({ icon, title, desc }) => (
          <div
            key={title}
            style={{
              background: 'rgba(255,255,255,0.03)',
              border: '1px solid rgba(255,255,255,0.07)',
              borderRadius: 20,
              padding: '22px 28px',
              width: 220,
              flexShrink: 0,
            }}
          >
            <div style={{ fontSize: 22, marginBottom: 10, color: '#D97757' }}>{icon}</div>
            <div
              style={{
                fontFamily: "'Fraunces', serif",
                fontSize: 16,
                fontWeight: 500,
                color: '#F5F4EF',
                marginBottom: 6,
              }}
            >
              {title}
            </div>
            <div
              style={{
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                color: 'rgba(245,244,239,0.4)',
                lineHeight: 1.5,
              }}
            >
              {desc}
            </div>
          </div>
        ))}
      </div>

      <p
        style={{
          marginTop: 56,
          fontFamily: "'Fraunces', serif",
          fontStyle: 'italic',
          fontSize: 14,
          color: 'rgba(245,244,239,0.2)',
          letterSpacing: '0.02em',
          animation: 'fadeUp 0.7s 0.4s ease both',
        }}
      >
        "A quiet shield for loud markets."
      </p>
    </div>
  );
}
