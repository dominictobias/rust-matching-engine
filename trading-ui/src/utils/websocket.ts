import toast from "react-hot-toast";

// Notification types matching the backend
export interface TradeNotification {
  id: number;
  taker_order_id: number;
  maker_order_id: number;
  taker_user_id: number;
  maker_user_id: number;
  quantity: number;
  price_tick: number;
  timestamp: number;
  is_taker: boolean;
}

export type NotificationType =
  | {
      type: "trade_fill";
      trade: TradeNotification;
      symbol: string;
    }
  | {
      type: "order_cancelled";
      order_id: number;
      symbol: string;
      reason: string;
    }
  | {
      type: "connection_established";
      user_id: number;
      message: string;
    };

export class NotificationWebSocket {
  private ws: WebSocket | null = null;
  private sessionId: string | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000; // Start with 1 second
  private isConnecting = false;
  private shouldReconnect = true;

  constructor() {
    // Arrow functions automatically bind 'this'
  }

  connect(sessionId: string): void {
    console.log("Connecting to notifications");
    if (
      this.isConnecting ||
      (this.ws && this.ws.readyState === WebSocket.CONNECTING)
    ) {
      console.log("Already connecting or connected");
      return;
    }

    this.sessionId = sessionId;
    this.shouldReconnect = true;
    this.isConnecting = true;

    try {
      const wsUrl = "/api/notifications";

      this.ws = new WebSocket(wsUrl);
      this.ws.addEventListener("open", this.handleOpen);
      this.ws.addEventListener("message", this.handleMessage);
      this.ws.addEventListener("close", this.handleClose);
      this.ws.addEventListener("error", this.handleError);
    } catch (error) {
      console.error("Failed to create WebSocket connection:", error);
      this.isConnecting = false;
      this.scheduleReconnect();
    }
  }

  disconnect(): void {
    this.shouldReconnect = false;
    this.sessionId = null;

    if (this.ws) {
      this.ws.removeEventListener("open", this.handleOpen);
      this.ws.removeEventListener("message", this.handleMessage);
      this.ws.removeEventListener("close", this.handleClose);
      this.ws.removeEventListener("error", this.handleError);

      if (
        this.ws.readyState === WebSocket.OPEN ||
        this.ws.readyState === WebSocket.CONNECTING
      ) {
        this.ws.close();
      }

      this.ws = null;
    }

    this.isConnecting = false;
    this.reconnectAttempts = 0;
  }

  private handleOpen = (): void => {
    console.log("WebSocket connected to notifications");
    this.isConnecting = false;
    this.reconnectAttempts = 0;
    this.reconnectDelay = 1000; // Reset delay

    // Send authentication message
    if (this.sessionId && this.ws) {
      this.ws.send(JSON.stringify({ sessionId: this.sessionId }));
    }
  };

  private handleMessage = (event: MessageEvent): void => {
    try {
      const notification: NotificationType = JSON.parse(event.data);
      this.showToastNotification(notification);
    } catch (error) {
      console.error("Failed to parse WebSocket message:", error);
    }
  };

  private handleClose = (event: CloseEvent): void => {
    console.log("WebSocket connection closed:", event.code, event.reason);
    this.isConnecting = false;
    this.ws = null;

    if (this.shouldReconnect && this.sessionId) {
      this.scheduleReconnect();
    }
  };

  private handleError = (event: Event): void => {
    console.error("WebSocket error:", event);
    this.isConnecting = false;
  };

  private scheduleReconnect(): void {
    if (
      !this.shouldReconnect ||
      this.reconnectAttempts >= this.maxReconnectAttempts
    ) {
      console.log("Max reconnection attempts reached or reconnection disabled");
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
      30000
    );

    console.log(
      `Scheduling WebSocket reconnection attempt ${this.reconnectAttempts} in ${delay}ms`
    );

    setTimeout(() => {
      if (this.shouldReconnect && this.sessionId) {
        this.connect(this.sessionId);
      }
    }, delay);
  }

  private showToastNotification(notification: NotificationType): void {
    switch (notification.type) {
      case "connection_established":
        if (notification.user_id > 0) {
          toast.success(notification.message, {
            duration: 3000,
            icon: "🔗",
          });
        } else {
          toast.error(notification.message, {
            duration: 5000,
            icon: "❌",
          });
        }
        break;

      case "trade_fill": {
        const { trade, symbol } = notification;
        const side = trade.is_taker ? "Taker" : "Maker";
        const priceFormatted = (trade.price_tick / 100).toFixed(2); // Assuming price is in cents
        const quantityFormatted = trade.quantity.toLocaleString();

        toast.success(
          `Trade Filled: ${side} ${quantityFormatted} ${symbol} @ $${priceFormatted}`,
          {
            duration: 5000,
            icon: "💰",
          }
        );
        break;
      }

      case "order_cancelled":
        toast.error(
          `Order Cancelled: ${notification.symbol} order #${notification.order_id}`,
          {
            duration: 4000,
            icon: "🚫",
          }
        );
        break;

      default:
        console.log("Unknown notification type:", notification);
    }
  }

  // Get current connection status
  get isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  get connectionState(): string {
    if (!this.ws) return "disconnected";

    switch (this.ws.readyState) {
      case WebSocket.CONNECTING:
        return "connecting";
      case WebSocket.OPEN:
        return "connected";
      case WebSocket.CLOSING:
        return "closing";
      case WebSocket.CLOSED:
        return "closed";
      default:
        return "unknown";
    }
  }
}

// Create a singleton instance
export const notificationWebSocket = new NotificationWebSocket();
