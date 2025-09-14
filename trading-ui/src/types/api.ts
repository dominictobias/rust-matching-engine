// API Types based on the Rust backend

export interface RegisterRequest {
  email: string;
  password: string;
}

export interface RegisterResponse {
  success: boolean;
  message: string;
  session_id?: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface LoginResponse {
  success: boolean;
  message: string;
  session_id?: string;
  user?: AuthenticatedUser;
}

export interface LogoutResponse {
  success: boolean;
  message: string;
}

export interface UserProfileResponse {
  success: boolean;
  user?: AuthenticatedUser;
  message: string;
}

export interface UserFunds {
  btc: number;
  sol: number;
  usd: number;
}

export interface AuthenticatedUser {
  session_id: string;
  email: string;
  funds: UserFunds;
}

export interface AddOrderRequest {
  symbol: string;
  price_tick: number;
  quantity: number;
  side: "bid" | "ask";
  time_in_force: "GTC" | "IOC" | "FOK";
}

export interface AddOrderResponse {
  order?: OrderResponse;
  trades: TradeResponse[];
  success: boolean;
  message: string;
}

export interface CancelOrderRequest {
  symbol: string;
  price_tick: number;
  side: "bid" | "ask";
}

export interface CancelOrderResponse {
  success: boolean;
  message: string;
}

export interface OrderResponse {
  id: number;
  symbol: string;
  price_tick: number;
  quantity: number;
  quantity_filled: number;
  side: "bid" | "ask";
  time_in_force: "GTC" | "IOC" | "FOK";
  timestamp: number;
  is_cancelled: boolean;
}

export interface TradeResponse {
  id: number;
  symbol: string;
  taker_order_id: number;
  maker_order_id: number;
  quantity: number;
  price_tick: number;
  timestamp: number;
}

export interface DepthLevelResponse {
  price_tick: number;
  quantity: number;
}

export interface DepthResponse {
  symbol: string;
  bids: DepthLevelResponse[];
  asks: DepthLevelResponse[];
}

export interface MarketAsset {
  id: string;
  symbol: string;
  name: string;
  icon: string;
  price: number;
  change24h: number;
}
