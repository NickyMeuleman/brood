import type { HeldPosition, TobRateHint } from "@/bindings";

export interface SellableHolding
	extends Omit<HeldPosition, "quantity" | "broker_id"> {
	// allow quantity to be unknown (when a fresh HeldPosition is fetched)
	quantity: string | null;
	// allow null for unheld listings
	broker_id: number | null;
	tob_rate_hint: TobRateHint | null;
}
