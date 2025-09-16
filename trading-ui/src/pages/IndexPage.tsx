import { Link, useLocation } from "wouter";
import { useEffect, useState } from "react";
import { useUserStore } from "../stores/userStore";
import { getMarkets } from "../utils/api";
import type { MarketAsset } from "../types/api";
import DefaultLayout from "../components/DefaultLayout";

export default function IndexPage() {
  const [, _setLocation] = useLocation();
  const { user: _user } = useUserStore();
  const [markets, setMarkets] = useState<MarketAsset[]>([]);
  const [loading, setLoading] = useState(true);

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
    <DefaultLayout>
      <div className="mb-8">
        <h2 className="text-3xl font-bold text-white mb-2">
          Available Markets
        </h2>
        <p className="text-zinc-400">Click on an asset to start trading</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {loading ? (
          <div className="col-span-2 text-center text-zinc-400">
            Loading markets...
          </div>
        ) : (
          markets.map((asset) => (
            <Link
              key={asset.id}
              to={`/trade/${asset.id}`}
              className="w-full bg-zinc-800 rounded-lg p-6 cursor-pointer hover:bg-zinc-700 transition-colors border border-zinc-700 hover:border-zinc-600 text-left"
            >
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center space-x-3">
                  <img src={asset.icon} alt={asset.name} className="w-8 h-8" />
                  <div>
                    <h3 className="text-xl font-semibold text-white">
                      {asset.name}
                    </h3>
                    <p className="text-zinc-400 text-sm">{asset.symbol}</p>
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

              <div className="flex items-center justify-between text-zinc-400 text-sm">
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
    </DefaultLayout>
  );
}
