import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useTobHint(
	executedAt: string,
	quantity: string,
	unitPrice: string,
	listingCurrency: string,
	tobRate?: string,
) {
	const { data: fxRate } = useQuery({
		enabled: Boolean(listingCurrency && executedAt),
		queryKey: queryKeys.fxRateFor(listingCurrency, executedAt),
		queryFn: async () => {
			const res = await commands.getRate(
				listingCurrency,
				executedAt || new Date().toISOString(),
			);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}
			return res.data;
		},
	});

	const tobHint = useMemo(() => {
		if (!tobRate) return null;

		const qty = Number(quantity);
		const price = Number(unitPrice);
		const rate = Number(tobRate);
		const fx = Number(fxRate);

		if (![qty, price, rate, fx].every((n) => Number.isFinite(n) && n > 0))
			return null;

		const hint = (qty * price * rate * fx).toFixed(2);
		return hint === "0.00" ? null : hint;
	}, [tobRate, quantity, unitPrice, fxRate]);

	return { tobHint, fxRate };
}
