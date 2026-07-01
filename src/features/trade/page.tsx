import { useStore } from "@tanstack/react-form";
import { useCallback, useEffect } from "react";
import { FieldGroup } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { buyFormOpts, buySchema } from "@/features/trade/shared-form.tsx";
import { useAppForm } from "@/hooks/form";
import { useBuy } from "@/hooks/use-buy";
import { useFx } from "@/hooks/use-fx";
import { useListings } from "@/hooks/use-listings";
import { usePrice } from "@/hooks/use-price";
import { useTobHint } from "@/hooks/use-tob-hint";
import { formatCurrency } from "@/lib/utils";

const BuyPage = () => {
	const buy = useBuy();

	const f = useAppForm({
		...buyFormOpts,
		onSubmit: ({ value }) => {
			const parsed = buySchema.parse(value);
			buy.mutate(parsed);
		},
		formId: "buyform",
	});

	const { data: listings = [], isLoading: listingsLoading } = useListings();

	const listingId = useStore(f.store, (state) => state.values.listing_id);
	const quantity = useStore(f.store, (state) => state.values.quantity);
	const unitPrice = useStore(f.store, (state) => state.values.unit_price);
	const executedAt = useStore(f.store, (state) => state.values.executed_at);
	const brokerFee = useStore(f.store, (state) => state.values.broker_fee);
	const tobFee = useStore(f.store, (state) => state.values.tob_fee);

	const listing = listings.find((l) => l.id === listingId);
	const listingCurrency = listing?.currency_code;
	const { data: fxRate } = useFx(executedAt, listingCurrency || "EUR");
	const { data: priceHint } = usePrice(executedAt, listingId);
	const tobHint = useTobHint(
		quantity,
		unitPrice,
		"buy",
		listing?.tob_rate_hint,
		fxRate,
	);

	const tobPristine = useStore(f.store, (s) => s.fieldMeta.tob_fee?.isPristine);
	const setTob = useCallback(
		(v: string) => {
			f.setFieldValue("tob_fee", v);
			f.setFieldMeta("tob_fee", (prev) => ({
				...prev,
				isTouched: false,
				isDirty: false,
				isPristine: true,
			}));
		},
		[f],
	);
	useEffect(() => {
		if (!tobPristine || !tobHint) return;
		setTob(tobHint);
	}, [tobHint, tobPristine, setTob]);

	const unitPricePristine = useStore(
		f.store,
		(s) => s.fieldMeta.unit_price?.isPristine,
	);
	const setUnitPrice = useCallback(
		(v: string) => {
			f.setFieldValue("unit_price", v);
			f.setFieldMeta("unit_price", (prev) => ({
				...prev,
				isTouched: false,
				isDirty: false,
				isPristine: true,
			}));
		},
		[f],
	);
	useEffect(() => {
		if (!unitPricePristine || !priceHint) return;
		const v = Number(priceHint).toFixed(2);
		setUnitPrice(v);
	}, [priceHint, unitPricePristine, setUnitPrice]);

	const base = Number(quantity) * Number(unitPrice);
	const convertedBase = base * Number(fxRate || 1);
	const total = convertedBase + Number(brokerFee) + Number(tobFee);

	return (
		<div className="m-auto mt-6 grid max-w-10/12 grid-cols-1 gap-12 lg:grid-cols-3">
			<div className="space-y-6 lg:col-span-2">
				<form
					onSubmit={(e) => {
						e.preventDefault();
						f.handleSubmit();
					}}
				>
					<FieldGroup>
						<f.AppField name="listing_id">
							{(field) => (
								<field.ListingPicker
									label="Listing"
									listings={listings}
									isLoading={listingsLoading}
								/>
							)}
						</f.AppField>
						<f.AppField name="executed_at">
							{(field) => <field.DateTimeField label="Execution time" />}
						</f.AppField>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="quantity">
								{(field) => <field.DecimalField label="Quantity" />}
							</f.AppField>
							<f.AppField name="unit_price">
								{(field) => (
									<field.DecimalField
										label="Unit price"
										currencyCode={listingCurrency}
									/>
								)}
							</f.AppField>
						</div>
						<div className="grid grid-cols-2 gap-6">
							<f.AppField name="broker_fee">
								{(field) => (
									<field.DecimalField label="Broker fee" currencyCode="eur" />
								)}
							</f.AppField>
							<f.AppField name="tob_fee">
								{(field) => (
									<field.DecimalField label="TOB" currencyCode="eur" />
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
						<p className="font-semibold text-lg">Price Summary</p>
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">Base</span>
							<span className="font-semibold text-base">
								{formatCurrency(base, listingCurrency)}
							</span>
						</div>

						{listingCurrency && listingCurrency !== "EUR" && fxRate ? (
							<div className="flex items-center justify-between gap-3 pl-4">
								<span className="text-muted-foreground text-sm">
									Converted to EUR
								</span>
								<span className="font-medium text-sm">
									{formatCurrency(convertedBase)}
								</span>
							</div>
						) : null}
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">
								Broker fee
							</span>
							<span className="font-semibold text-base">
								{formatCurrency(Number(brokerFee))}
							</span>
						</div>
						<div className="flex items-center justify-between gap-3">
							<span className="text-base text-muted-foreground">TOB</span>
							<span className="font-semibold text-base">
								{formatCurrency(Number(tobFee))}
							</span>
						</div>
					</div>

					<Separator />

					<div className="flex items-center justify-between gap-3">
						<span className="font-semibold text-xl">Total</span>
						<span className="font-semibold text-xl">
							{formatCurrency(total)}
						</span>
					</div>
				</div>
			</div>
		</div>
	);
};

export default BuyPage;
