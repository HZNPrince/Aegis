import { useWallet } from '@solana/wallet-adapter-react';
import { motion } from 'framer-motion';
import { useEffect, useState } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { SonarLogo } from './SonarLogo';
import { Button, tokens } from './sonar';

function NavItem({ to, label, active }: { to: string; label: string; active: boolean }) {
  const [hover, setHover] = useState(false);
  return (
    <Link
      to={to}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        position: 'relative',
        padding: '0 14px',
        height: 60,
        display: 'flex',
        alignItems: 'center',
        fontFamily: 'var(--sans)',
        fontSize: 13,
        fontWeight: 500,
        color: active || hover ? 'var(--nav-text)' : 'var(--nav-text-dim)',
        textDecoration: 'none',
        letterSpacing: '0.005em',
        transition: 'color 0.18s ease',
      }}
    >
      {/* Hover background pill */}
      {hover && !active && (
        <motion.span
          layoutId="nav-hover-pill"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ type: 'spring', stiffness: 380, damping: 32 }}
          style={{
            position: 'absolute',
            left: 6,
            right: 6,
            top: 14,
            bottom: 14,
            borderRadius: 8,
            background: 'rgba(246, 244, 238, 0.07)',
            zIndex: -1,
          }}
        />
      )}
      <span style={{ position: 'relative', zIndex: 1 }}>{label}</span>
      {active && (
        <motion.span
          layoutId="nav-underline"
          style={{
            position: 'absolute',
            left: 14,
            right: 14,
            bottom: 0,
            height: 2,
            background: 'var(--nav-text)',
            borderRadius: 2,
          }}
          transition={{ type: 'spring', stiffness: 380, damping: 30 }}
        />
      )}
    </Link>
  );
}

function useTheme(): [string, (e?: { clientX: number; clientY: number }) => void] {
  const [theme, setTheme] = useState<string>(() => {
    if (typeof window === 'undefined') return 'light';
    return localStorage.getItem('aegis-theme') || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  });
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('aegis-theme', theme);
  }, [theme]);

  const toggle = (e?: { clientX: number; clientY: number }) => {
    const next = theme === 'dark' ? 'light' : 'dark';
    const root = document.documentElement;
    const w = window.innerWidth;
    const h = window.innerHeight;
    const x = e?.clientX ?? w;
    const y = e?.clientY ?? 0;
    const r = Math.hypot(Math.max(x, w - x), Math.max(y, h - y));
    root.style.setProperty('--theme-x', `${x}px`);
    root.style.setProperty('--theme-y', `${y}px`);
    root.style.setProperty('--theme-r', `${r}px`);

    // Use View Transitions API where available — Chrome/Edge/Safari TP.
    if (typeof document.startViewTransition === 'function' && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      document.startViewTransition(() => setTheme(next));
    } else {
      setTheme(next);
    }
  };
  return [theme, toggle];
}

interface Props {
  connected: boolean;
  onConnect: () => void;
}

const LINKS: { to: string; label: string }[] = [
  { to: '/dashboard', label: 'Dashboard' },
  { to: '/positions', label: 'Positions' },
  { to: '/alerts', label: 'Alerts' },
  { to: '/rules', label: 'Guardrails' },
  { to: '/telegram', label: 'Telegram' },
];

function shortAddr(w: string) {
  return w.length > 10 ? `${w.slice(0, 4)}…${w.slice(-4)}` : w;
}

export function Nav({ connected, onConnect }: Props) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const { publicKey } = useWallet();
  const walletAddr = publicKey?.toBase58() ?? '';
  const [theme, toggleTheme] = useTheme();

  return (
    <nav
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        zIndex: 100,
        height: 60,
        background: 'var(--nav-bg)',
        backdropFilter: 'blur(18px)',
        WebkitBackdropFilter: 'blur(18px)',
        borderBottom: '1px solid var(--nav-line)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 28px',
      }}
    >
      {/* Logo — Sonar logo only, clickable */}
      <Link
        to="/"
        style={{
          display: 'flex',
          alignItems: 'center',
          marginRight: 36,
          textDecoration: 'none',
        }}
      >
        <SonarLogo
          size={32}
          ink="#F6F4EE"
          accent="#3B5BDB"
          dot="#C44536"
        />
      </Link>

      {connected && (
        <div style={{ display: 'flex', height: '100%', alignItems: 'center' }}>
          {LINKS.map((l) => {
            const active = pathname === l.to || pathname.startsWith(l.to + '/');
            return <NavItem key={l.to} to={l.to} label={l.label} active={active} />;
          })}
        </div>
      )}

      <div style={{ flex: 1 }} />

      <button
        onClick={(e) => toggleTheme({ clientX: e.clientX, clientY: e.clientY })}
        aria-label="Toggle theme"
        title={theme === 'dark' ? 'Switch to light' : 'Switch to dark'}
        style={{
          width: 34,
          height: 34,
          marginRight: 12,
          borderRadius: 999,
          border: '1px solid rgba(246, 244, 238, 0.16)',
          background: 'rgba(246, 244, 238, 0.06)',
          color: 'var(--nav-text)',
          cursor: 'pointer',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          transition: 'background 0.15s, transform 0.15s',
        }}
        onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(246, 244, 238, 0.12)')}
        onMouseLeave={(e) => (e.currentTarget.style.background = 'rgba(246, 244, 238, 0.06)')}
      >
        {theme === 'dark' ? (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
        )}
      </button>

      {connected ? (
        <button
          onClick={() => navigate('/settings')}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '6px 12px',
            background: 'rgba(246, 244, 238, 0.08)',
            border: '1px solid rgba(246, 244, 238, 0.12)',
            borderRadius: 999,
            fontFamily: tokens.mono,
            fontSize: 11.5,
            color: 'var(--nav-text)',
            cursor: 'pointer',
            letterSpacing: '0.04em',
          }}
        >
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              background: tokens.moss,
              display: 'inline-block',
            }}
          />
          {shortAddr(walletAddr)}
        </button>
      ) : (
        <Button variant="accent" onClick={onConnect}>
          Connect wallet
        </Button>
      )}
    </nav>
  );
}
