import { useStore } from "@tanstack/react-form";
import { useEffect, useMemo } from "react";
import { toast } from "sonner";
import type { MoneyInput } from "@/bindings";
import { QueryError } from "@/components/QueryError";
import { Badge } from "@/components/ui/badge";
import { FieldGroup } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import {
	buildSellSchema,
	sellFormOpts,
} from "@/features/trade/shared-form.tsx";
import { useAppForm } from "@/hooks/form";
import { useBrokerFee } from "@/hooks/use-broker-fee";
import { useBrokers } from "@/hooks/use-brokers";
import { useFieldHint } from "@/hooks/use-field-hint";
import { useFx } from "@/hooks/use-fx";
import { useHeldPositions } from "@/hooks/use-held-positions";
import { useListings } from "@/hooks/use-listings";
import { usePrice } from "@/hooks/use-price";
import { useSell } from "@/hooks/use-sell";
import { useSellPreview } from "@/hooks/use-sell-preview";
import { useTobHint } from "@/hooks/use-tob-hint";
import { cn, formatCurrency, getCurrencySymbol, MIC_LABEL } from "@/lib/utils";
import { TruncatedTooltip } from "../holdings/TruncatedTooltip";
import { FxRatePreview } from "./FxRatePreview";
import { SellTaxDetailSheet } from "./SellDetails";
import type { SellableHolding } from "./types";
import { computeTradeSummary } from "./lib";

const SellPage = () => {
	const sell = useSell();

	const f = useAppForm({
		...sellFormOpts,
		onSubmit: ({ value }) => {
			const schema = buildSellSchema(maxQty);
			const parsed = schema.safeParse(value);
			if (!parsed.success) {
				toast.error("Sell form submitted with invalid data", {
					description: parsed.error.message,
				});
				return;
			}
			sell.mutate(parsed.data);
		},
		formId: "sell_form",
	});

	const { data: brokers = [] } = useBrokers();
	const { data: listings = [] } = useListings();

	const listingId = useStore(f.store, (state) => state.values.listing_id);
	const brokerId = useStore(f.store, (state) => state.values.broker_id);
	const quantity = useStore(f.store, (state) => state.values.quantity);
	const unitPrice = useStore(f.store, (state) => state.values.unit_price);
	const executedAt = useStore(f.store, (state) => state.values.executed_at);
	const brokerFee = useStore(f.store, (state) => state.values.broker_fee);
	const tobFee = useStore(f.store, (state) => state.values.tob_fee);

	const {
		data: heldPositions,
		isLoading: heldPositionsLoading,
		error: heldPositionsError,
	} = useHeldPositions(executedAt, brokerId || undefined);

	const sellableHoldings = useMemo<SellableHolding[]>(() => {
		const tobByIsin = new Map(listings.map((l) => [l.isin, l.tob_rate_hint]));
		const candidates = new Map<string, SellableHolding>();

		// hasBroker: broker was picked via form and is not the default value/not in database
		function candidateKey(
			hasBroker: boolean,
			listingId: number,
			brokerId?: number,
		) {
			return hasBroker ? `${brokerId}-${listingId}` : `${listingId}`;
		}

		for (const p of heldPositions ?? []) {
			// aggregate by listing only while broker isn't selected
			const key = candidateKey(Boolean(brokerId), p.listing_id, p.broker_id);
			const existing = candidates.get(key);
			if (existing) {
				candidates.set(key, {
					...existing,
					quantity: (
						Number(existing?.quantity || "0") + Number(p?.quantity || "0")
					).toString(),
					tob_rate_hint: tobByIsin.get(p.isin) ?? null,
				});
			} else {
				candidates.set(key, {
					...p,
					tob_rate_hint: tobByIsin.get(p.isin) ?? null,
				});
			}
		}

		if (listingId) {
			const key = candidateKey(Boolean(brokerId), listingId, brokerId);
			if (!candidates.has(key)) {
				const listing = listings.find((l) => l.id === listingId);
				if (listing) {
					candidates.set(key, {
						listing_id: listingId,
						broker_id: brokerId || null,
						isin: listing.isin,
						ticker: listing.ticker,
						exchange_mic: listing.exchange_mic,
						currency_code: listing.currency_code,
						instrument_name: listing.instrument_name,
						instrument_type: listing.instrument_type,
						// 0 is confirmed not held, null is unknown (while loading)
						quantity: heldPositionsLoading ? null : "0",
						tob_rate_hint: listing.tob_rate_hint,
					});
				}
			}
		}

		return [...candidates.values()].sort((a, b) =>
			a.ticker.localeCompare(b.ticker),
		);
	}, [heldPositions, listings, listingId, brokerId, heldPositionsLoading]);

	const holding = sellableHoldings.find((h) => h.listing_id === listingId);
	const listingCurrency = holding?.currency_code;
	const isForeignCurrency = Boolean(
		listingCurrency && listingCurrency !== "EUR",
	);

	const broker = brokers.find((b) => b.id === brokerId);

	const maxQty = holding?.quantity ?? undefined;
	const quantitySchema = useMemo(
		() => buildSellSchema(maxQty).shape.quantity,
		[maxQty],
	);

	const {
		data: listingFx,
		error: listingFxError,
		isLoading: listingFxLoading,
	} = useFx(executedAt, listingCurrency || "EUR");

	const {
		data: brokerFeeFx,
		error: brokerFeeFxError,
		isLoading: brokerFeeFxLoading,
	} = useFx(executedAt, brokerFee?.currency || "EUR");

	const { data: unitPriceHint } = usePrice(executedAt, listingId);
	const unitPriceHintValue: MoneyInput = useMemo(() => {
		return {
			currency: listingCurrency ?? "EUR",
			amount: unitPriceHint != null ? Number(unitPriceHint).toFixed(2) : "",
		};
	}, [listingCurrency, unitPriceHint]);
	useFieldHint(f, "unit_price", unitPriceHintValue);

	const tobHint = useTobHint(
		quantity,
		unitPrice.amount,
		"sell",
		holding?.tob_rate_hint,
		listingFx,
	);
	const tobHintValue: MoneyInput = useMemo(() => {
		return {
			currency: "EUR",
			amount: Number(tobHint || "0").toFixed(2),
		};
	}, [tobHint]);
	useFieldHint(f, "tob_fee", tobHintValue);

	const { data: brokerFeeHint } = useBrokerFee(
		broker?.broker_type || null,
		listingCurrency || "EUR",
		quantity,
		unitPrice.amount,
		holding?.instrument_type || "STOCK",
		holding?.exchange_mic || "XAMS",
		listingFx || "1",
	);
	const brokerFeeHintValue: MoneyInput = useMemo(() => {
		return {
			currency: brokerFeeHint?.currency || "EUR",
			amount: Number(brokerFeeHint?.amount || "0").toFixed(2),
		};
	}, [brokerFeeHint?.amount, brokerFeeHint?.currency]);
	useFieldHint(f, "broker_fee", brokerFeeHintValue);

	const summary = computeTradeSummary({
		quantity,
		unitPrice,
		listingCurrency: listingCurrency ?? "EUR",
		brokerFee,
		tobFee,
		listingFx,
		brokerFeeFx,
	});

	const totalEur =
		summary.baseEur === null || summary.brokerFeeEur === null
			? null
			: summary.baseEur - summary.brokerFeeEur - summary.tobFee.amount;

	const preview = useSellPreview({
		listing_id: listingId,
		broker_id: brokerId,
		quantity,
		unit_price: unitPrice,
		executed_at: executedAt,
		broker_fee: brokerFee,
		tob_fee: tobFee,
	});

	useEffect(() => {
		if (heldPositionsLoading) return;
		if (f.getFieldMeta("quantity")?.isPristine) return;
		f.validateField("quantity", "change");
	}, [heldPositionsLoading, f]);

	return (
		<div className="m-auto mt-6 grid max-w-10/12 grid-cols-1 gap-12 lg:grid-cols-3">
			<div className="space-y-6 lg:col-span-2">
				{heldPositionsError ? (
					<QueryError message={heldPositionsError.message} />
				) : null}
				<form
					onSubmit={(e) => {
						e.preventDefault();
						f.handleSubmit();
					}}
				>
					<FieldGroup>
						<f.AppField name="listing_id">
							{(field) => (
								<field.HoldingPicker
									label="Holding"
									holdings={sellableHoldings}
									isLoading={heldPositionsLoading}
								/>
							)}
						</f.AppField>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="executed_at">
								{(field) => <field.DateTimeField label="Execution time" />}
							</f.AppField>
							<f.AppField name="broker_id">
								{(field) => (
									<field.SelectField
										label="Broker"
										options={brokers.map((b) => ({
											value: b.id,
											label: b.name,
										}))}
										placeholder=""
									/>
								)}
							</f.AppField>
						</div>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField
								name="quantity"
								validators={{
									onChange: quantitySchema,
									onChangeListenTo: ["listing_id", "executed_at"],
								}}
							>
								{(field) => <field.DecimalField label="Quantity" />}
							</f.AppField>
							<f.AppField name="unit_price">
								{(field) => (
									<field.MoneyField
										label="Unit price"
										currencyDisabled={true}
									/>
								)}
							</f.AppField>
						</div>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="broker_fee">
								{(field) => <field.MoneyField label="Broker fee" />}
							</f.AppField>
							<f.AppField name="tob_fee">
								{(field) => (
									<field.MoneyField label="TOB" currencyDisabled={true} />
								)}
							</f.AppField>
						</div>
						<f.AppForm>
							<f.SubmitButton label="Submit" />
						</f.AppForm>
					</FieldGroup>
				</form>
			</div>

			<div className="flex flex-col gap-6">
				<div className="flex flex-col gap-4 rounded-md bg-muted p-6">
					<div className="space-y-4">
						<p className="font-semibold text-lg">Sale Summary</p>
						<div className="flex w-full items-center gap-3">
							{holding && (
								<div className="flex min-w-0 flex-1 flex-col gap-0.5">
									<TruncatedTooltip>{holding.instrument_name}</TruncatedTooltip>
									<div className="flex items-center gap-1.5 whitespace-nowrap font-normal text-muted-foreground text-sm">
										<Badge
											variant="ghost"
											className={cn(
												"bg-primary font-mono text-primary-foreground text-sm tracking-wider",
											)}
										>
											{holding.ticker}
										</Badge>
										<span className="opacity-60">·</span>
										<span>
											{MIC_LABEL[holding.exchange_mic] ?? holding.exchange_mic}
										</span>
										{isForeignCurrency && (
											<>
												<span className="opacity-60">·</span>
												<span>{getCurrencySymbol(holding.currency_code)}</span>
											</>
										)}
										<span className="opacity-60">·</span>
										<span>{holding.quantity ?? "..."} held</span>
									</div>
								</div>
							)}
						</div>

						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">Base</span>
							<span className="font-semibold text-base">
								{formatCurrency(summary.base.amount, summary.base.currency)}
							</span>
						</div>
						{summary.base.currency !== "EUR" && (
							<FxRatePreview
								isLoading={listingFxLoading}
								error={listingFxError}
								rate={listingFx}
								convertedValue={summary.baseEur}
							/>
						)}
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">
								Broker fee
							</span>
							<span className="font-semibold text-base text-rose-700">
								{formatCurrency(
									summary.brokerFee.amount,
									summary.brokerFee.currency,
								)}
							</span>
						</div>
						{summary.brokerFee.currency !== "EUR" && (
							<FxRatePreview
								className="text-rose-700"
								isLoading={brokerFeeFxLoading}
								error={brokerFeeFxError}
								rate={brokerFeeFx}
								convertedValue={summary.brokerFeeEur}
							/>
						)}
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">TOB</span>
							<span className="font-semibold text-base text-rose-700">
								{formatCurrency(summary.tobFee.amount, summary.tobFee.currency)}
							</span>
						</div>
					</div>

					<Separator />

					<div className="flex items-center justify-between gap-3">
						<span className="font-semibold text-xl">Total</span>
						<span className="font-semibold text-xl">
							{totalEur === null ? "-" : formatCurrency(totalEur, "EUR")}
						</span>
					</div>

					{preview.error && <QueryError message={preview.error.message} />}
					{preview.data && (
						<>
							<Separator />
							<div className="space-y-2">
								<div className="flex items-center justify-between gap-3">
									<div className="grid">
										<span className="text-base text-muted-foreground">
											Realized gain
										</span>
										<span className="text-muted-foreground text-sm">
											After fees (TOB + broker)
										</span>
									</div>
									<span className="font-semibold text-base">
										{formatCurrency(
											Number(preview.data.total_economic_gain_eur),
										)}
									</span>
								</div>
								<div className="flex items-center justify-between gap-3">
									<span className="text-base text-muted-foreground">
										Taxable gain
									</span>
									{preview.data.total_taxable_gain_eur !== null ? (
										<span className="font-semibold text-base">
											{formatCurrency(
												Number(preview.data.total_taxable_gain_eur),
											)}
										</span>
									) : (
										<span className="text-muted-foreground text-sm">
											Not subject to Belgian capital gains tax
										</span>
									)}
								</div>
								{preview.data.estimated_gross_cgt_eur !== null && (
									<div className="flex items-center justify-between gap-3">
										<div className="grid">
											<span className="text-base text-muted-foreground">
												Estimated CGT
											</span>
											<span className="text-muted-foreground text-sm">
												Before annual exemption and loss netting
											</span>
										</div>
										<span className="font-semibold text-base">
											{formatCurrency(
												Number(preview.data.estimated_gross_cgt_eur),
											)}
										</span>
									</div>
								)}
								<SellTaxDetailSheet
									computation={preview.data}
									listings={listings}
									summary={summary}
									quantity={quantity}
									unitPrice={unitPrice}
									fxRate={listingFx}
								/>
							</div>
						</>
					)}
				</div>
			</div>
		</div>
	);
};

export default SellPage;
