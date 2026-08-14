import type { InstrumentType, Period } from "@/bindings";

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
	brokerFee: ["brokerFee"] as const,
	brokerFeeFor: (
		broker: string,
		quantity: string,
		unitPrice: string,
		instrumentType: InstrumentType,
		mic: string,
		fxRate: string,
	) =>
		[
			"brokerFee",
			broker,
			quantity,
			unitPrice,
			instrumentType,
			mic,
			fxRate,
		] as const,
	supportedMics: ["supportedMics"] as const,
	instruments: ["instruments"] as const,
	instrumentByIsin: (isin: string) => ["instrument", isin] as const,
	listingCandidates: (isin: string) => ["listingCandidates", isin] as const,
	listingMeta: (mic: string, ticker: string) =>
		["listingMeta", mic, ticker] as const,
	sellPreview: ["sellPreview"] as const,
	sellPreviewFor: (
		listingId: number,
		quantity: string,
		unitPrice: string,
		executedAt: string,
		brokerFee: string,
		tobFee: string,
	) =>
		[
			"sellPreview",
			listingId,
			quantity,
			unitPrice,
			executedAt,
			brokerFee,
			tobFee,
		] as const,
};
