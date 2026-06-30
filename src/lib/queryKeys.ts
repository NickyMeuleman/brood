import type { Period } from "@/bindings";

export const queryKeys = {
	holdings: ["holdings"] as const,
	holdingsByPeriod: (period: Period) => ["holdings", period] as const,
	portfolioHistory: ["portfolioHistory"] as const,
	portfolioHistoryByPeriod: (period: Period) =>
		["portfolioHistory", period] as const,
	listings: ["listings"] as const,
	fxRate: ["fxRate"] as const,
	fxRateFor: (currency: string, date: string) =>
		["fxRate", currency, date] as const,
	price: ["price"] as const,
	priceFor: (listingId: number, date: string) =>
		["price", listingId, date] as const,
};
