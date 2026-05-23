import type { Period } from "@/bindings";

export const queryKeys = {
	holdings: ["holdings"] as const,
	holdingsByPeriod: (period: Period) => ["holdings", period] as const,
};
