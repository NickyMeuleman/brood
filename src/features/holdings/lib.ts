import type { EnvelopeHolding, PeriodContext, Snapshot } from "@/bindings";

export function normalizeHolding(
	h: EnvelopeHolding,
	includeFees: boolean,
	displayInEur: boolean,
) {
	const is_converted = displayInEur && h.currency_code !== "EUR";
	const display_currency = is_converted ? "EUR" : h.currency_code;

	const pick = <T>(ng: { gross: T; net: T }): T =>
		includeFees ? ng.net : ng.gross;
	const cur = <T>(pair: { local: T; eur: T }, useEur: boolean): T =>
		useEur ? pair.eur : pair.local;

	const toNum = (v: string | null): number | null =>
		v === null ? null : Number(v);

	const resolvePerf = (ctx: PeriodContext, useEur: boolean) => {
		const perf = cur(ctx.perf, useEur);
		const lens = pick(perf);
		return {
			cost: toNum(lens.cost),
			gain: toNum(lens.gain),
			pct_gain: toNum(lens.pct_gain),
			fees: toNum(perf.fees),
			fee_drag: toNum(ctx.perf.eur.fee_drag), // Drag is always EUR, FX cancels out
		};
	};

	const resolveCurrent = (snap: Snapshot, useEur: boolean) => {
		return {
			quantity: toNum(snap.quantity),
			unit_price: toNum(cur(snap.unit_price, useEur)),
			value: toNum(cur(snap.value, useEur)),
			unit_price_basis: toNum(pick(cur(snap.unit_price_basis, useEur))),
		};
	};

	const resolve = (useEur: boolean) => ({
		current: resolveCurrent(h.current, useEur),
		all_time: resolvePerf(h.all_time, useEur),
		period: resolvePerf(h.period, useEur),
	});

	return {
		...h,
		is_converted,
		display_currency,
		display: resolve(is_converted),
		local: resolve(false),
		eur: resolve(true),
	};
}

export type NormalizedHolding = ReturnType<typeof normalizeHolding>;
