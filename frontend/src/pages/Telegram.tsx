import { Navigate } from 'react-router-dom';

/**
 * Telegram page — bot preview moved to Landing page,
 * "Connect bot" moved to Settings.
 * This route now redirects to Settings.
 */
export function Telegram() {
  return <Navigate to="/settings" replace />;
}
