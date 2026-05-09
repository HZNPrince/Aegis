// Main React application — handles wallet connection, routing, and page rendering.
// Manages user session, wallet linking, and navigation between dashboard, positions, alerts, etc.

import { useWallet } from '@solana/wallet-adapter-react';
import { useWalletModal } from '@solana/wallet-adapter-react-ui';
import { AnimatePresence, motion } from 'framer-motion';
import { useEffect } from 'react';
import type { ReactNode } from 'react';
import { Navigate, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import { Nav } from './components/Nav';
import { useLinkWallet } from './hooks';
import { Alerts } from './pages/Alerts';
import { Dashboard } from './pages/Dashboard';
import { GuardRules } from './pages/GuardRules';
import { Landing } from './pages/Landing';
import { Pitch } from './pages/Pitch';
import { ProtocolDetail } from './pages/PositionDetail';
import { Positions } from './pages/Positions';
import { Settings } from './pages/Settings';
import { Simulator } from './pages/Simulator';

function App() {
  const { connected, publicKey, disconnect } = useWallet();
  const { setVisible } = useWalletModal();
  const location = useLocation();
  const navigate = useNavigate();
  const linkWallet = useLinkWallet();

  useEffect(() => {
    if (!connected || !publicKey) return;
    const wallet = publicKey.toBase58();
    if (location.pathname === '/') navigate('/dashboard');
    // Only backfill once per wallet per browser session — refreshing the
    // page should not retrigger a full RPC backfill on the server.
    const key = `aegis-linked:${wallet}`;
    if (sessionStorage.getItem(key)) return;
    linkWallet.mutate(wallet, {
      onSuccess: () => sessionStorage.setItem(key, String(Date.now())),
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connected, publicKey?.toBase58()]);

  const openConnect = () => setVisible(true);
  const handleDisconnect = async () => {
    await disconnect().catch(() => {});
    navigate('/');
  };

  const isLanding = location.pathname === '/';
  const isPitch = location.pathname === '/pitch';

  return (
    <div style={{ minHeight: '100vh', background: 'var(--paper)', color: 'var(--ink)' }}>
      {!isPitch && <Nav connected={connected} onConnect={openConnect} />}
      <div style={{ paddingTop: isLanding || isPitch ? 0 : 60 }}>
        <AnimatePresence mode="wait">
          <motion.div
            key={location.pathname}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0, transition: { duration: 0.22, ease: [0.2, 0.8, 0.2, 1] } }}
            exit={{ opacity: 0, y: -4, transition: { duration: 0.12 } }}
          >
            <Routes location={location}>
              <Route path="/" element={<Landing onConnect={openConnect} />} />
              <Route path="/pitch" element={<Pitch />} />
              <Route path="/dashboard" element={<Protected connected={connected}><Dashboard /></Protected>} />
              <Route path="/positions" element={<Protected connected={connected}><Positions /></Protected>} />
              <Route path="/protocol/:protocolName" element={<Protected connected={connected}><ProtocolDetail /></Protected>} />
              <Route path="/alerts" element={<Protected connected={connected}><Alerts /></Protected>} />
              <Route path="/rules" element={<Protected connected={connected}><GuardRules /></Protected>} />
              <Route path="/telegram" element={<Navigate to="/settings" replace />} />
              <Route path="/simulator" element={<Protected connected={connected}><Simulator /></Protected>} />
              <Route path="/settings" element={<Protected connected={connected}><Settings onDisconnect={handleDisconnect} /></Protected>} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

function Protected({ connected, children }: { connected: boolean; children: ReactNode }) {
  if (!connected) return <Navigate to="/" replace />;
  return <>{children}</>;
}

export default App;
