import { useState, useEffect, useCallback } from "react";
import { useLocation, useParams } from "wouter";
import type {
  AddOrderRequest,
  AddOrderResponse,
  DepthResponse,
  MarketAsset,
} from "../types/api";
import { useUserStore } from "../stores/userStore";
import { authenticatedApiCall, getDepth, getMarkets } from "../utils/api";
import { priceToTick, tickToPrice, getDecimalPlaces } from "../utils/prices";
import DefaultLayout from "../components/DefaultLayout";

export default function TradePage() {
  const [, setLocation] = useLocation();
  const params = useParams<{ assetId: string }>();
  const { session_id, refreshProfile } = useUserStore();

  const assetId = params.assetId;
  const [markets, setMarkets] = useState<MarketAsset[]>([]);
  const [loadingMarkets, setLoadingMarkets] = useState(true);
  const asset = markets.find((a) => a.id === assetId);

  const [orderForm, setOrderForm] = useState<AddOrderRequest>({
    symbol: "",
    price_tick: 0,
    quantity: 1,
    side: "bid",
    time_in_force: "GTC",
  });
  const [isSubmittingOrder, setIsSubmittingOrder] = useState(false);
  const [orderMessage, setOrderMessage] = useState<string | null>(null);
  const [depthData, setDepthData] = useState<DepthResponse | null>(null);
  const [isLoadingDepth, setIsLoadingDepth] = useState(false);

  // Fetch markets on component mount
  useEffect(() => {
    const fetchMarkets = async () => {
      try {
        const marketsData = await getMarkets();
        setMarkets(marketsData);
      } catch (error) {
        console.error("Failed to fetch markets:", error);
      } finally {
        setLoadingMarkets(false);
      }
    };

    fetchMarkets();
  }, []);

  // Redirect to home if asset not found
  useEffect(() => {
    if (!loadingMarkets && !asset) {
      setLocation("/");
    }
  }, [asset, loadingMarkets, setLocation]);

  // Update order form symbol when asset changes
  useEffect(() => {
    if (asset) {
      setOrderForm((prev) => ({
        ...prev,
        symbol: asset.symbol,
        price_tick: 0, // Will be set when user enters price
      }));
    }
  }, [asset]);

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
        // Refresh user profile to update balances
        await refreshProfile();
      } else {
        setOrderMessage(`Error: ${data.message}`);
      }
    } catch (err) {
      console.error("Failed to submit order:", err);
      setOrderMessage(`Failed to submit order: ${err}`);
    } finally {
      setIsSubmittingOrder(false);
    }
  };

  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    const decimalPrice = Number(value);
    if (asset) {
      const tickValue = priceToTick(decimalPrice, asset.tick_multiplier);
      setOrderForm((prev) => ({
        ...prev,
        price_tick: tickValue,
      }));
    }
  };

  const handleQuantityChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setOrderForm((prev) => ({
      ...prev,
      quantity: Number(value),
    }));
  };

  const handleSideChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    setOrderForm((prev) => ({
      ...prev,
      side: value as "bid" | "ask",
    }));
  };

  const handleTimeInForceChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    setOrderForm((prev) => ({
      ...prev,
      time_in_force: value as "GTC" | "IOC" | "FOK",
    }));
  };

  if (loadingMarkets) {
    return (
      <div className="min-h-screen bg-zinc-900 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-white mx-auto mb-4"></div>
          <p className="text-zinc-400">Loading markets...</p>
        </div>
      </div>
    );
  }

  if (!asset) {
    return (
      <div className="min-h-screen bg-zinc-900 flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-2xl font-bold text-white mb-4">
            Asset Not Found
          </h1>
          <p className="text-zinc-400 mb-6">
            The requested asset could not be found.
          </p>
          <button
            onClick={handleBackToAssets}
            className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-md transition-colors"
          >
            Back to Markets
          </button>
        </div>
      </div>
    );
  }

  return (
    <DefaultLayout
      showBackButton={true}
      backButtonAction={handleBackToAssets}
      title={
        <div className="flex items-center space-x-3">
          <img src={asset.icon} alt={asset.name} className="w-6 h-6" />
          <div>
            <h1 className="text-2xl font-bold text-white">{asset.name}</h1>
            <p className="text-zinc-400 text-sm">{asset.symbol}</p>
          </div>
        </div>
      }
    >
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Order Form */}
        <div className="bg-zinc-900 rounded-lg p-6">
          <h2 className="text-xl font-semibold text-white mb-6">
            Place Order - {asset.symbol}
          </h2>

          <form onSubmit={handleOrderSubmit} className="space-y-4">
            <div>
              <label
                htmlFor="side"
                className="block text-sm font-medium text-zinc-300 mb-2"
              >
                Side
              </label>
              <select
                id="side"
                name="side"
                value={orderForm.side}
                onChange={handleSideChange}
                className="w-full px-3 py-2 bg-zinc-700 border border-zinc-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="bid">Buy</option>
                <option value="ask">Sell</option>
              </select>
            </div>

            <div>
              <label
                htmlFor="price"
                className="block text-sm font-medium text-zinc-300 mb-2"
              >
                Price (USD)
              </label>
              <input
                type="number"
                id="price"
                name="price"
                value={tickToPrice(orderForm.price_tick, asset.tick_multiplier)}
                onChange={handlePriceChange}
                min={`0.${"0".repeat(
                  getDecimalPlaces(asset.tick_multiplier) - 1
                )}1`}
                step={`0.${"0".repeat(
                  getDecimalPlaces(asset.tick_multiplier) - 1
                )}1`}
                className="w-full px-3 py-2 bg-zinc-700 border border-zinc-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label
                htmlFor="quantity"
                className="block text-sm font-medium text-zinc-300 mb-2"
              >
                Quantity
              </label>
              <input
                type="number"
                id="quantity"
                name="quantity"
                value={orderForm.quantity}
                onChange={handleQuantityChange}
                min="1"
                className="w-full px-3 py-2 bg-zinc-700 border border-zinc-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label
                htmlFor="time_in_force"
                className="block text-sm font-medium text-zinc-300 mb-2"
              >
                Time in Force
              </label>
              <select
                id="time_in_force"
                name="time_in_force"
                value={orderForm.time_in_force}
                onChange={handleTimeInForceChange}
                className="w-full px-3 py-2 bg-zinc-700 border border-zinc-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
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
        <div className="bg-zinc-900 rounded-lg p-6">
          <h2 className="text-xl font-semibold text-white mb-6">
            Order Book - {asset.symbol}
          </h2>

          <div className="bg-zinc-800 rounded-lg p-4">
            {isLoadingDepth ? (
              <div className="flex items-center justify-center py-8">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-white mr-2"></div>
                <span className="text-zinc-400">Loading orderbook...</span>
              </div>
            ) : depthData ? (
              <>
                <div className="text-sm text-zinc-400 mb-2">
                  Bids (Buy Orders)
                </div>
                <div className="space-y-1 text-xs max-h-48 overflow-y-auto">
                  {depthData.bids.length > 0 ? (
                    depthData.bids.map((bid, index) => (
                      <div
                        key={index}
                        className="flex justify-between text-green-400"
                      >
                        <span>
                          $
                          {tickToPrice(
                            bid.price_tick,
                            asset.tick_multiplier
                          ).toFixed(getDecimalPlaces(asset.tick_multiplier))}
                        </span>
                        <span>{bid.quantity}</span>
                      </div>
                    ))
                  ) : (
                    <div className="text-zinc-500 text-center py-2">
                      No bids
                    </div>
                  )}
                </div>

                <div className="text-sm text-zinc-400 mb-2 mt-4">
                  Asks (Sell Orders)
                </div>
                <div className="space-y-1 text-xs max-h-48 overflow-y-auto">
                  {depthData.asks.length > 0 ? (
                    depthData.asks.map((ask, index) => (
                      <div
                        key={index}
                        className="flex justify-between text-red-400"
                      >
                        <span>
                          $
                          {tickToPrice(
                            ask.price_tick,
                            asset.tick_multiplier
                          ).toFixed(getDecimalPlaces(asset.tick_multiplier))}
                        </span>
                        <span>{ask.quantity}</span>
                      </div>
                    ))
                  ) : (
                    <div className="text-zinc-500 text-center py-2">
                      No asks
                    </div>
                  )}
                </div>
              </>
            ) : (
              <div className="text-zinc-500 text-center py-8">
                Failed to load orderbook data
              </div>
            )}
          </div>
        </div>
      </div>
    </DefaultLayout>
  );
}
