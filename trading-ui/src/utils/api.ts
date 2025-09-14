import type { DepthResponse, MarketAsset } from "../types/api";

export interface ApiResponse<T = unknown> {
  success: boolean;
  message: string;
  data?: T;
}

export class ApiError extends Error {
  public status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
    this.name = "ApiError";
  }
}

// Generic API call function
export async function apiCall<T = unknown>(
  url: string,
  options: RequestInit = {}
): Promise<T> {
  const response = await fetch(url, {
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
    ...options,
  });

  if (!response.ok) {
    throw new ApiError(
      response.status,
      `HTTP ${response.status}: ${response.statusText}`
    );
  }

  return response.json();
}

// API call with authentication
export async function authenticatedApiCall<T = unknown>(
  endpoint: string,
  sessionId: string,
  options: RequestInit = {}
): Promise<T> {
  return apiCall<T>(endpoint, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionId}`,
      ...options.headers,
    },
  });
}

// Check if session is valid
export async function validateSession(sessionId: string): Promise<boolean> {
  try {
    const res = await fetch("/api/profile", {
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionId}`,
      },
    });
    return res.ok;
  } catch (_err) {
    return false;
  }
}

// Get orderbook depth
export async function getDepth(
  symbol: string,
  levels: number = 20,
  sessionId: string
): Promise<DepthResponse> {
  const params = new URLSearchParams({
    symbol,
    levels: levels.toString(),
  });

  return authenticatedApiCall<DepthResponse>(`/api/depth?${params}`, sessionId);
}

// Get markets/assets
export async function getMarkets(): Promise<MarketAsset[]> {
  return apiCall<MarketAsset[]>("/api/markets");
}
