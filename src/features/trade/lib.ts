import type { MoneyInput } from "@/bindings";

export interface TradeSummary {
	// quantity × unit price
	base: { amount: number; currency: string };
	baseEur: number | null;
	brokerFee: { amount: number; currency: string };
	brokerFeeEur: number | null;
	tobFee: { amount: number; currency: string };
}

// Display-only arithmetic.
// Authoritative figures come from the backend.
export function computeTradeSummary(args: {
	quantity: string;
	unitPrice: MoneyInput;
	listingCurrency: string;
	brokerFee: MoneyInput | null;
	tobFee: MoneyInput | null;
	listingFx: string | undefined;
	brokerFeeFx: string | undefined;
}): TradeSummary {
	const { listingCurrency, brokerFee, tobFee, listingFx, brokerFeeFx } = args;

	const baseAmount = Number(args.quantity) * Number(args.unitPrice.amount);
	const baseCurrency = listingCurrency;

	let baseEur: number | null;
	if (listingCurrency === "EUR") {
		baseEur = baseAmount;
	} else if (listingFx) {
		baseEur = baseAmount * Number(listingFx);
	} else {
		baseEur = null;
	}

	const brokerFeeAmount = Number(brokerFee?.amount || "0");
	const brokerFeeCurrency = brokerFee?.currency ?? "EUR";

	let brokerFeeEur: number | null;
	if (brokerFeeCurrency === "EUR") {
		brokerFeeEur = brokerFeeAmount;
	} else if (brokerFeeFx) {
		brokerFeeEur = brokerFeeAmount * Number(brokerFeeFx);
	} else {
		brokerFeeEur = null;
	}

	const tobFeeAmount = Number(tobFee?.amount || "0");
	if (tobFee?.currency && tobFee?.currency !== "EUR") {
		throw new Error("TOB currency was not EUR");
	}
	const tobFeeCurrency = tobFee?.currency || "EUR";

	return {
		base: { amount: baseAmount, currency: baseCurrency },
		baseEur,
		brokerFee: { amount: brokerFeeAmount, currency: brokerFeeCurrency },
		brokerFeeEur,
		tobFee: { amount: tobFeeAmount, currency: tobFeeCurrency },
	};
}
