import { useMemo } from "react";
import type { TobRateHint } from "@/bindings";

export function useTobHint(
	quantity: string,
	unitPrice: string,
	side: "buy" | "sell",
	tobRate?: TobRateHint | null,
	fxRate?: string,
) {
	return useMemo(() => {
		if (!tobRate) return null;

		const qty = Number(quantity || 0);
		const price = Number(unitPrice || 0);
		const rate = Number(tobRate[side] || 0);
		const fx = Number(fxRate || 1);

		if (![qty, price, rate, fx].every((n) => Number.isFinite(n) && n > 0))
			return null;

		const hint = (qty * price * rate * fx).toFixed(2);
		return hint === "0.00" ? null : hint;
	}, [quantity, unitPrice, side, tobRate, fxRate]);
}
