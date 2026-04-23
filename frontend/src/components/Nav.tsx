import { Link, useLocation, useNavigate } from 'react-router-dom';
import { ShieldIcon } from './ShieldIcon';
import { WalletChip } from './ui';

interface Props {
  connected: boolean;
  onConnect: () => void;
}

const LINKS: { to: string; label: string }[] = [
  { to: '/dashboard', label: 'Dashboard' },
  { to: '/alerts', label: 'Alerts' },
  { to: '/rules', label: 'Guard Rules' },
  { to: '/simulator', label: 'Simulator' },
  { to: '/settings', label: 'Settings' },
];

export function Nav({ connected, onConnect }: Props) {
  const { pathname } = useLocation();
  const navigate = useNavigate();

  return (
    <nav
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        zIndex: 100,
        height: 56,
        background: 'rgba(31,30,29,0.92)',
        backdropFilter: 'blur(20px)',
        borderBottom: '1px solid rgba(255,255,255,0.06)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 28px',
        gap: 0,
      }}
    >
      <Link
        to="/"
        style={{
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          marginRight: 32,
          padding: 0,
          textDecoration: 'none',
        }}
      >
        <ShieldIcon size={22} />
        <span
          style={{
            fontFamily: "'Fraunces', serif",
            fontSize: 18,
            fontWeight: 600,
            color: '#F5F4EF',
            letterSpacing: '-0.02em',
          }}
        >
          Aegis
        </span>
      </Link>
      {connected &&
        LINKS.map((l) => {
          const active = pathname === l.to;
          return (
            <Link
              key={l.to}
              to={l.to}
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                padding: '0 14px',
                height: 56,
                display: 'flex',
                alignItems: 'center',
                fontFamily: "'Inter', sans-serif",
                fontSize: 13,
                fontWeight: 500,
                color: active ? '#F5F4EF' : 'rgba(245,244,239,0.45)',
                borderBottom: active ? '2px solid #D97757' : '2px solid transparent',
                transition: 'color 0.2s, border-color 0.2s',
                letterSpacing: '0.01em',
                textDecoration: 'none',
              }}
            >
              {l.label}
            </Link>
          );
        })}
      <div style={{ flex: 1 }} />
      {connected ? (
        <WalletChip onClick={() => navigate('/settings')} />
      ) : (
        <button
          onClick={onConnect}
          style={{
            background: '#D97757',
            border: 'none',
            cursor: 'pointer',
            padding: '8px 18px',
            borderRadius: 100,
            fontFamily: "'Inter', sans-serif",
            fontSize: 13,
            fontWeight: 600,
            color: '#1F1E1D',
            letterSpacing: '0.01em',
          }}
        >
          Connect Wallet
        </button>
      )}
    </nav>
  );
}
