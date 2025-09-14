import { Link, useLocation } from "wouter";
import { useEffect, useState } from "react";
import { useUserStore } from "../stores/userStore";
import { getMarkets } from "../utils/api";
import type { MarketAsset } from "../types/api";

export default function IndexPage() {
  const [, setLocation] = useLocation();
  const { user, clearUser } = useUserStore();
  const [markets, setMarkets] = useState<MarketAsset[]>([]);
  const [loading, setLoading] = useState(true);

  const handleLogout = () => {
    clearUser();
    setLocation("/login");
  };

  useEffect(() => {
    const fetchMarkets = async () => {
      try {
        const markets = await getMarkets();
        setMarkets(markets);
      } catch (error) {
        console.error("Failed to fetch markets:", error);
      } finally {
        setLoading(false);
      }
    };

    fetchMarkets();
  }, []);

  return (
    <div className="min-h-screen bg-gray-900">
      {/* Header */}
      <header className="bg-gray-800 border-b border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <h1 className="text-2xl font-bold text-white">Trading Engine</h1>
            <div className="flex items-center space-x-4">
              <span className="text-gray-300">Welcome, {user?.email}</span>
              <button
                onClick={handleLogout}
                className="bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-md text-sm font-medium transition-colors"
              >
                Logout
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Funds Display */}
      {user && (
        <div className="bg-gray-800 border-b border-gray-700">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
            <h2 className="text-lg font-semibold text-white mb-4">Portfolio</h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="bg-gray-700 rounded-lg p-4">
                <div className="text-sm text-gray-400">Bitcoin</div>
                <div className="text-xl font-bold text-orange-400">
                  {user.funds.btc.toFixed(8)} BTC
                </div>
              </div>
              <div className="bg-gray-700 rounded-lg p-4">
                <div className="text-sm text-gray-400">Solana</div>
                <div className="text-xl font-bold text-purple-400">
                  {user.funds.sol.toFixed(2)} SOL
                </div>
              </div>
              <div className="bg-gray-700 rounded-lg p-4">
                <div className="text-sm text-gray-400">USD</div>
                <div className="text-xl font-bold text-green-400">
                  ${user.funds.usd.toLocaleString()} USD
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-8">
          <h2 className="text-3xl font-bold text-white mb-2">
            Available Markets
          </h2>
          <p className="text-gray-400">Click on an asset to start trading</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {loading ? (
            <div className="col-span-2 text-center text-gray-400">
              Loading markets...
            </div>
          ) : (
            markets.map((asset) => (
              <Link
                key={asset.id}
                to={`/trade/${asset.id}`}
                className="w-full bg-gray-800 rounded-lg p-6 cursor-pointer hover:bg-gray-700 transition-colors border border-gray-700 hover:border-gray-600 text-left"
              >
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center space-x-3">
                    <img
                      src={asset.icon}
                      alt={asset.name}
                      className="w-8 h-8"
                    />
                    <div>
                      <h3 className="text-xl font-semibold text-white">
                        {asset.name}
                      </h3>
                      <p className="text-gray-400 text-sm">{asset.symbol}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-2xl font-bold text-white">
                      ${asset.price.toLocaleString()}
                    </div>
                    <div
                      className={`text-sm ${
                        asset.change24h >= 0 ? "text-green-400" : "text-red-400"
                      }`}
                    >
                      {asset.change24h >= 0 ? "+" : ""}
                      {asset.change24h}%
                    </div>
                  </div>
                </div>

                <div className="flex items-center justify-between text-gray-400 text-sm">
                  <span>Click to trade</span>
                  <svg
                    className="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 5l7 7-7 7"
                    />
                  </svg>
                </div>
              </Link>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
