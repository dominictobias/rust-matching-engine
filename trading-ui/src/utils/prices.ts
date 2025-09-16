// Helper functions to convert between decimal prices and ticks

export const priceToTick = (price: number, tickMultiplier: number): number => {
  return Math.round(price * tickMultiplier);
};

export const tickToPrice = (tick: number, tickMultiplier: number): number => {
  return tick / tickMultiplier;
};

// Get decimal places from tick_multiplier
export const getDecimalPlaces = (tickMultiplier: number): number => {
  return Math.log10(tickMultiplier);
};
