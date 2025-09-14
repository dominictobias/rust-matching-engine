import { useState, useEffect, useCallback } from "react";
import { useLocation, useParams } from "wouter";
import type {
  AddOrderRequest,
  AddOrderResponse,
  DepthResponse,
} from "../types/api";
import { useUserStore } from "../stores/userStore";
import { authenticatedApiCall, getDepth } from "../utils/api";

// Asset configuration
const ASSETS = [
  {
    id: "BTCUSD",
    symbol: "BTC-USD",
    name: "Bitcoin",
    icon: "₿",
    color: "orange",
    defaultPrice: 50000,
  },
  {
    id: "SOLUSD",
    symbol: "SOL-USD",
    name: "Solana",
    icon: "◎",
    color: "purple",
    defaultPrice: 100,
  },
] as const;

export default function TradePage() {
  const [, setLocation] = useLocation();
  const params = useParams<{ assetId: string }>();
  const { user, session_id, clearUser } = useUserStore();

  const assetId = params.assetId;
  const asset = ASSETS.find((a) => a.id === assetId);

  const [orderForm, setOrderForm] = useState<AddOrderRequest>({
    symbol: asset?.symbol || "BTC-USD",
    price_tick: asset?.defaultPrice || 50000,
    quantity: 1,
    side: "bid",
    time_in_force: "GTC",
  });
  const [isSubmittingOrder, setIsSubmittingOrder] = useState(false);
  const [orderMessage, setOrderMessage] = useState<string | null>(null);
  const [depthData, setDepthData] = useState<DepthResponse | null>(null);
  const [isLoadingDepth, setIsLoadingDepth] = useState(false);

  // Redirect to home if asset not found
  useEffect(() => {
    if (!asset) {
      setLocation("/");
    }
  }, [asset, setLocation]);

  // Update order form symbol when asset changes
  useEffect(() => {
    if (asset) {
      setOrderForm((prev) => ({
        ...prev,
        symbol: asset.symbol,
        price_tick: asset.defaultPrice,
      }));
    }
  }, [asset]);

  const handleLogout = () => {
    clearUser();
    setLocation("/login");
  };

  const handleBackToAssets = () => {
    setLocation("/");
  };

  const fetchDepthData = useCallback(
    async (symbol: string) => {
      if (!session_id) return;

      setIsLoadingDepth(true);
      try {
        const data = await getDepth(symbol, 20, session_id);
        setDepthData(data);
      } catch (error) {
        console.error("Failed to fetch depth data:", error);
      } finally {
        setIsLoadingDepth(false);
      }
    },
    [session_id]
  );

  // Fetch depth data on component mount and when symbol changes
  useEffect(() => {
    if (session_id && asset) {
      fetchDepthData(asset.symbol);
    }
  }, [session_id, asset, fetchDepthData]);

  const handleOrderSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmittingOrder(true);
    setOrderMessage(null);

    if (!session_id) {
      setOrderMessage("No session found. Please login again.");
      setIsSubmittingOrder(false);
      return;
    }

    try {
      const data: AddOrderResponse =
        await authenticatedApiCall<AddOrderResponse>(
          "/api/orders",
          session_id,
          {
            method: "POST",
            body: JSON.stringify(orderForm),
          }
        );

      if (data.success) {
        setOrderMessage(
          `Order ${data.order ? "placed" : "rejected"}: ${data.message}`
        );
        if (data.trades.length > 0) {
          setOrderMessage(
            (prev) => `${prev} ${data.trades.length} trade(s) executed.`
          );
        }
        // Reset form on success
        setOrderForm((prev) => ({ ...prev, quantity: 1 }));
        // Refresh depth data after successful order
        if (asset) {
          fetchDepthData(asset.symbol);
        }
      } else {
        setOrderMessage(`Error: ${data.message}`);
      }
    } catch {
      setOrderMessage("Network error. Please try again.");
    } finally {
      setIsSubmittingOrder(false);
    }
  };

  const handleInputChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>
  ) => {
    const { name, value } = e.target;
    setOrderForm((prev) => ({
      ...prev,
      [name]:
        name === "price_tick" || name === "quantity" ? Number(value) : value,
    }));
  };

  if (!asset) {
    return null;
  }

  return (
    <div className="min-h-screen bg-gray-900">
      {/* Header */}
      <header className="bg-gray-800 border-b border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center py-4">
            <div className="flex items-center space-x-4">
              <button
                onClick={handleBackToAssets}
                className="text-gray-400 hover:text-white transition-colors"
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
              <div className="flex items-center space-x-3">
                <div
                  className={`text-2xl ${
                    asset.color === "orange"
                      ? "text-orange-400"
                      : "text-purple-400"
                  }`}
                >
                  {asset.icon}
                </div>
                <div>
                  <h1 className="text-2xl font-bold text-white">
                    {asset.name}
                  </h1>
                  <p className="text-gray-400 text-sm">{asset.symbol}</p>
                </div>
              </div>
            </div>
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
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Order Form */}
          <div className="bg-gray-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold text-white mb-6">
              Place Order - {asset.symbol}
            </h2>

            <form onSubmit={handleOrderSubmit} className="space-y-4">
              <div>
                <label
                  htmlFor="side"
                  className="block text-sm font-medium text-gray-300 mb-2"
                >
                  Side
                </label>
                <select
                  id="side"
                  name="side"
                  value={orderForm.side}
                  onChange={handleInputChange}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="bid">Buy (Bid)</option>
                  <option value="ask">Sell (Ask)</option>
                </select>
              </div>

              <div>
                <label
                  htmlFor="price_tick"
                  className="block text-sm font-medium text-gray-300 mb-2"
                >
                  Price (ticks)
                </label>
                <input
                  type="number"
                  id="price_tick"
                  name="price_tick"
                  value={orderForm.price_tick}
                  onChange={handleInputChange}
                  min="1"
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label
                  htmlFor="quantity"
                  className="block text-sm font-medium text-gray-300 mb-2"
                >
                  Quantity
                </label>
                <input
                  type="number"
                  id="quantity"
                  name="quantity"
                  value={orderForm.quantity}
                  onChange={handleInputChange}
                  min="1"
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label
                  htmlFor="time_in_force"
                  className="block text-sm font-medium text-gray-300 mb-2"
                >
                  Time in Force
                </label>
                <select
                  id="time_in_force"
                  name="time_in_force"
                  value={orderForm.time_in_force}
                  onChange={handleInputChange}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="GTC">Good Till Cancelled</option>
                  <option value="IOC">Immediate or Cancel</option>
                  <option value="FOK">Fill or Kill</option>
                </select>
              </div>

              <button
                type="submit"
                disabled={isSubmittingOrder}
                className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSubmittingOrder ? (
                  <div className="flex items-center justify-center">
                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
                    Placing Order...
                  </div>
                ) : (
                  "Place Order"
                )}
              </button>
            </form>

            {orderMessage && (
              <div
                className={`mt-4 p-3 rounded-md ${
                  orderMessage.includes("Error")
                    ? "bg-red-900/50 text-red-200"
                    : "bg-green-900/50 text-green-200"
                }`}
              >
                {orderMessage}
              </div>
            )}
          </div>

          {/* Order Book */}
          <div className="bg-gray-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold text-white mb-6">
              Order Book - {asset.symbol}
            </h2>

            <div className="bg-gray-700 rounded-lg p-4">
              {isLoadingDepth ? (
                <div className="flex items-center justify-center py-8">
                  <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-white mr-2"></div>
                  <span className="text-gray-400">Loading orderbook...</span>
                </div>
              ) : depthData ? (
                <>
                  <div className="text-sm text-gray-400 mb-2">
                    Bids (Buy Orders)
                  </div>
                  <div className="space-y-1 text-xs max-h-48 overflow-y-auto">
                    {depthData.bids.length > 0 ? (
                      depthData.bids.map((bid, index) => (
                        <div
                          key={index}
                          className="flex justify-between text-green-400"
                        >
                          <span>{bid.price_tick.toLocaleString()}</span>
                          <span>{bid.quantity}</span>
                        </div>
                      ))
                    ) : (
                      <div className="text-gray-500 text-center py-2">
                        No bids
                      </div>
                    )}
                  </div>

                  <div className="text-sm text-gray-400 mb-2 mt-4">
                    Asks (Sell Orders)
                  </div>
                  <div className="space-y-1 text-xs max-h-48 overflow-y-auto">
                    {depthData.asks.length > 0 ? (
                      depthData.asks.map((ask, index) => (
                        <div
                          key={index}
                          className="flex justify-between text-red-400"
                        >
                          <span>{ask.price_tick.toLocaleString()}</span>
                          <span>{ask.quantity}</span>
                        </div>
                      ))
                    ) : (
                      <div className="text-gray-500 text-center py-2">
                        No asks
                      </div>
                    )}
                  </div>
                </>
              ) : (
                <div className="text-gray-500 text-center py-8">
                  Failed to load orderbook data
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
