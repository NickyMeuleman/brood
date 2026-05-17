import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { FXSyncOutcome, Period, PriceSyncOutcome } from "@/bindings";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function formatCurrency(
	val: number,
	currency: string = "EUR",
	opt: Intl.NumberFormatOptions = {},
) {
	return new Intl.NumberFormat("nl-BE", {
		style: "currency",
		currency,
		...opt,
	}).format(val);
}

export function formatPercentage(
	val: number,
	opt: Intl.NumberFormatOptions = {},
) {
	return new Intl.NumberFormat("nl-BE", {
		style: "percent",
		maximumSignificantDigits: 3,
		...opt,
	}).format(val);
}

const symbolCache = new Map<string, string>();
export const getCurrencySymbol = (currency: string) => {
	if (symbolCache.has(currency)) return symbolCache.get(currency)!;
	const symbol =
		new Intl.NumberFormat("nl-BE", {
			style: "currency",
			currency,
		})
			.formatToParts(0)
			.find((p) => p.type === "currency")?.value ?? currency;
	symbolCache.set(currency, symbol);
	return symbol;
};

export const MIC_LABEL: Record<string, string> = {
	XAMS: "Amsterdam",
	XETR: "Xetra",
	XPAR: "Paris",
	XLON: "London",
	XBRU: "Brussels",
	XMIL: "Milan",
	XNAS: "Nasdaq",
	NYSE: "New York",
	XSTU: "Stuttgart",
	XSWX: "SIX",
};

export const PERIOD_LABEL: Record<Period, string> = {
	FiveDays: "5d",
	OneMonth: "1m",
	SixMonths: "6m",
	OneYear: "1y",
	FiveYears: "5y",
	Ytd: "ytd",
	AllTime: "all",
} as const;
export const PERIODS = Object.keys(PERIOD_LABEL) as Period[];

export function unwrapPriceOutcome(outcome: PriceSyncOutcome) {
	if (outcome.status === "error") {
		throw new Error(`${outcome.ticker}: ${outcome.message}`);
	}
	return outcome;
}

export function unwrapFxOutcome(outcome: FXSyncOutcome) {
	if (outcome.status === "error") {
		throw new Error(`${outcome.currency}: ${outcome.message}`);
	}
	return outcome;
}
