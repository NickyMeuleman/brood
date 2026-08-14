import type { HeldPosition, TobRateHint } from "@/bindings";

export interface SellableHolding extends HeldPosition {
	tob_rate_hint: TobRateHint | null;
}
