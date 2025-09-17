import { useState, useEffect } from "react";
import type {
  AddOrderRequest,
  AddOrderResponse,
  MarketAsset,
} from "../types/api";
import { useUserStore } from "../stores/userStore";
import { authenticatedApiCall } from "../utils/api";
import { priceToTick, tickToPrice, getDecimalPlaces } from "../utils/prices";

interface OrderFormProps {
  asset: MarketAsset;
  onOrderSuccess: () => void;
}

export default function OrderForm({ asset, onOrderSuccess }: OrderFormProps) {
  const { sessionId, refreshProfile } = useUserStore();

  const [orderForm, setOrderForm] = useState<AddOrderRequest>({
    symbol: "",
    price_tick: 0,
    quantity: 1,
    side: "bid",
    time_in_force: "GTC",
  });
  const [isSubmittingOrder, setIsSubmittingOrder] = useState(false);
  const [orderMessage, setOrderMessage] = useState<string | null>(null);

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

  const handleOrderSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmittingOrder(true);
    setOrderMessage(null);

    if (!sessionId) {
      setOrderMessage("No session found. Please login again.");
      setIsSubmittingOrder(false);
      return;
    }

    try {
      const data: AddOrderResponse =
        await authenticatedApiCall<AddOrderResponse>("/api/orders", sessionId, {
          method: "POST",
          body: JSON.stringify(orderForm),
        });

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
        // Refresh user profile to update balances
        await refreshProfile();
        // Notify parent component of successful order
        onOrderSuccess();
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

  return (
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
  );
}
