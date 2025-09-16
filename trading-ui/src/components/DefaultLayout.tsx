import { type ReactNode } from "react";
import { useLocation } from "wouter";
import { useUserStore } from "../stores/userStore";

interface DefaultLayoutProps {
  children: ReactNode;
  showBackButton?: boolean;
  backButtonAction?: () => void;
  title?: ReactNode;
}

export default function DefaultLayout({
  children,
  showBackButton = false,
  backButtonAction,
  title,
}: DefaultLayoutProps) {
  const [, setLocation] = useLocation();
  const { user, clearUser } = useUserStore();

  const handleLogout = () => {
    clearUser();
    setLocation("/login");
  };

  const handleBackToAssets = () => {
    setLocation("/");
  };

  const defaultBackAction = backButtonAction || handleBackToAssets;

  return (
    <div className="min-h-screen bg-zinc-950">
      {/* Header */}
      <header className="bg-zinc-950 border-b border-zinc-900">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <div className="flex items-center space-x-4">
              {showBackButton && (
                <button
                  onClick={defaultBackAction}
                  className="text-zinc-400 hover:text-white transition-colors"
                >
                  <svg
                    className="w-6 h-6"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 19l-7-7 7-7"
                    />
                  </svg>
                </button>
              )}
              <div className="flex items-center space-x-3">
                {title ? (
                  title
                ) : (
                  <h1 className="text-2xl font-bold text-white">
                    Trading Engine
                  </h1>
                )}
              </div>
            </div>
            <div className="flex items-center space-x-4">
              <span className="text-zinc-300">Welcome, {user?.email}</span>
              <button
                onClick={handleLogout}
                className="bg-zinc-100 text-black px-4 py-2 rounded-md text-sm font-medium transition-colors hover:cursor-pointer"
              >
                Logout
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Funds Display */}
      {user && (
        <div className="bg-zinc-950 border-b border-zinc-900">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
            <h2 className="text-lg font-semibold text-white mb-4">Portfolio</h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="bg-zinc-900 rounded-lg p-4">
                <div className="text-sm text-zinc-400">Bitcoin</div>
                <div className="text-xl font-medium text-white">
                  {user.funds.btc.toFixed(8)} BTC
                </div>
              </div>
              <div className="bg-zinc-900 rounded-lg p-4">
                <div className="text-sm text-zinc-400">Solana</div>
                <div className="text-xl font-medium text-white">
                  {user.funds.sol.toFixed(2)} SOL
                </div>
              </div>
              <div className="bg-zinc-900 rounded-lg p-4">
                <div className="text-sm text-zinc-400">USD</div>
                <div className="text-xl font-medium text-white">
                  ${user.funds.usd.toLocaleString()} USD
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {children}
      </div>
    </div>
  );
}
