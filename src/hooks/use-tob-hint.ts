import { useMemo } from "react";

export function useTobHint(
	quantity: string,
	unitPrice: string,
	tobRate?: string,
	fxRate?: string,
) {
	return useMemo(() => {
		if (!tobRate) return null;

		const qty = Number(quantity || 0);
		const price = Number(unitPrice || 0);
		const rate = Number(tobRate || 0);
		const fx = Number(fxRate || 1);

		if (![qty, price, rate, fx].every((n) => Number.isFinite(n) && n > 0))
			return null;

		const hint = (qty * price * rate * fx).toFixed(2);
		return hint === "0.00" ? null : hint;
	}, [tobRate, quantity, unitPrice, fxRate]);
}
