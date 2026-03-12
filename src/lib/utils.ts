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
