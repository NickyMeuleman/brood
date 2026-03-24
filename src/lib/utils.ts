import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export function formatCurrency(val: number, currency = "EUR") {
	return new Intl.NumberFormat("nl-BE", {
		style: "currency",
		currency,
	}).format(val);
}

export function formatPercentage(val: number) {
	return new Intl.NumberFormat("nl-BE", {
		style: "percent",
		maximumSignificantDigits: 3,
	}).format(val);
}

const symbolCache = new Map<string, string>();
export const getCurrencySymbol = (currency: string) => {
	if (symbolCache.has(currency)) return symbolCache.get(currency)!;
	const symbol =
		new Intl.NumberFormat("en", {
			style: "currency",
			currency,
			minimumFractionDigits: 0,
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
