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
import { Settings } from './pages/Settings';
import { Simulator } from './pages/Simulator';

function App() {
  const { connected, publicKey, disconnect } = useWallet();
  const { setVisible } = useWalletModal();
  const location = useLocation();
  const navigate = useNavigate();
  const linkWallet = useLinkWallet();

  // On first connect, redirect to /dashboard and link wallet on backend.
  useEffect(() => {
    if (connected && publicKey) {
      if (location.pathname === '/') navigate('/dashboard');
      linkWallet.mutate(publicKey.toBase58());
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connected, publicKey?.toBase58()]);

  const openConnect = () => setVisible(true);

  const handleDisconnect = async () => {
    await disconnect().catch(() => {});
    navigate('/');
  };

  return (
    <div style={{ minHeight: '100vh', background: '#1F1E1D', color: '#F5F4EF' }}>
      <Nav connected={connected} onConnect={openConnect} />
      <AnimatePresence mode="wait">
        <motion.div
          key={location.pathname}
          initial={{ opacity: 0, y: 14 }}
          animate={{
            opacity: 1,
            y: 0,
            transition: { duration: 0.32, ease: [0.25, 0.46, 0.45, 0.94] },
          }}
          exit={{ opacity: 0, y: -8, transition: { duration: 0.18, ease: 'easeIn' } }}
        >
          <Routes location={location}>
            <Route path="/" element={<Landing onConnect={openConnect} />} />
            <Route
              path="/dashboard"
              element={
                <Protected connected={connected}>
                  <Dashboard />
                </Protected>
              }
            />
            <Route
              path="/alerts"
              element={
                <Protected connected={connected}>
                  <Alerts />
                </Protected>
              }
            />
            <Route
              path="/rules"
              element={
                <Protected connected={connected}>
                  <GuardRules />
                </Protected>
              }
            />
            <Route
              path="/simulator"
              element={
                <Protected connected={connected}>
                  <Simulator />
                </Protected>
              }
            />
            <Route
              path="/settings"
              element={
                <Protected connected={connected}>
                  <Settings onDisconnect={handleDisconnect} />
                </Protected>
              }
            />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </motion.div>
      </AnimatePresence>
    </div>
  );
}

function Protected({ connected, children }: { connected: boolean; children: ReactNode }) {
  if (!connected) return <Navigate to="/" replace />;
  return <>{children}</>;
}

export default App;
