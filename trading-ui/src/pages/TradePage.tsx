import { useState, useEffect, useCallback } from "react";
import { useLocation, useParams } from "wouter";
import type { AddOrderRequest, DepthResponse, MarketAsset } from "../types/api";
import { useUserStore } from "../stores/userStore";
import { getDepth, getMarkets } from "../utils/api";
import { tickToPrice, getDecimalPlaces } from "../utils/prices";
import DefaultLayout from "../components/DefaultLayout";
import OrderForm from "../composites/OrderForm";
import { toast } from "react-hot-toast";

export default function TradePage() {
  const [, setLocation] = useLocation();
  const params = useParams<{ assetId: string }>();
  const { user } = useUserStore();

  const assetId = params.assetId;
  const [markets, setMarkets] = useState<MarketAsset[]>([]);
  const [loadingMarkets, setLoadingMarkets] = useState(true);
  const asset = markets.find((a) => a.id === assetId);

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

  const handleBackToAssets = () => {
    setLocation("/");
  };

  const fetchDepthData = useCallback(
    async (symbol: string) => {
      if (!user?.session_id) return;

      setIsLoadingDepth(true);
      try {
        const data = await getDepth(symbol, 20, user.session_id);
        setDepthData(data);
      } catch (error) {
        console.error("Failed to fetch depth data:", error);
      } finally {
        setIsLoadingDepth(false);
      }
    },
    [user?.session_id]
  );

  // Fetch depth data on component mount and when symbol changes
  useEffect(() => {
    if (user?.session_id && asset) {
      fetchDepthData(asset.symbol);
    }
  }, [user?.session_id, asset, fetchDepthData]);

  const handleOrderSuccess = (orderRequest: AddOrderRequest) => {
    if (asset) {
      toast.success(
        `Order Placed! ${orderRequest.side.toUpperCase()} ${asset.symbol} ${
          orderRequest.quantity
        } @ ${tickToPrice(
          orderRequest.price_tick,
          asset.tick_multiplier
        ).toFixed(getDecimalPlaces(asset.tick_multiplier))}`,
        {
          duration: 5000,
          icon: "🚀",
        }
      );

      // Refresh depth data after successful order
      fetchDepthData(asset.symbol);
    }
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
        <OrderForm asset={asset} onOrderSuccess={handleOrderSuccess} />

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
