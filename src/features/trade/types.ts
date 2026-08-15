import type { HeldPosition, TobRateHint } from "@/bindings";

export interface SellableHolding extends Omit<HeldPosition, "quantity"> {
  // allow quantity to be unknown (when a fresh HeldPosition is fetched)
	quantity: string | null;
	tob_rate_hint: TobRateHint | null;
}
