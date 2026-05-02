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

	const resolvePeriodCtx = (ctx: PeriodContext, useEur: boolean) => ({
		start: {
			value: ctx.start_value ? toNum(cur(ctx.start_value, useEur)) : null,
			unit_price: ctx.start_unit_price
				? toNum(cur(ctx.start_unit_price, useEur))
				: null,
		},
		perf: resolvePerf(ctx, useEur),
	});

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
		all_time: resolvePeriodCtx(h.all_time, useEur),
		period: resolvePeriodCtx(h.period, useEur),
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
