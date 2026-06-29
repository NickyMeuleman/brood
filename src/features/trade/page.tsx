import { useStore } from "@tanstack/react-form";
import { useCallback, useEffect } from "react";
import { FieldGroup } from "@/components/ui/field";
import { buyFormOpts, buySchema } from "@/features/trade/shared-form.tsx";
import { useAppForm } from "@/hooks/form";
import { useBuy } from "@/hooks/use-buy";
import { useListings } from "@/hooks/use-listings";
import { useTobHint } from "@/hooks/use-tob-hint";

const BuyPage = () => {
	const buy = useBuy();

	const f = useAppForm({
		...buyFormOpts,
		onSubmit: ({ value }) => {
			const parsed = buySchema.parse(value);
			buy.mutate(parsed);
		},
	});

	const { data: listings = [], isLoading: listingsLoading } = useListings();

	const listingId = useStore(f.store, (state) => state.values.listing_id);
	const quantity = useStore(f.store, (state) => state.values.quantity);
	const unitPrice = useStore(f.store, (state) => state.values.unit_price);
	const executedAt = useStore(f.store, (state) => state.values.executed_at);

	const listing = listings.find((l) => l.id === listingId);
	const listingCurrency = listing?.currency_code;
	const { tobHint } = useTobHint(
		executedAt,
		quantity,
		unitPrice,
		listingCurrency ?? "EUR",
		listing?.tob_rate_hint,
	);

	const tobPristine = useStore(f.store, (s) => s.fieldMeta.tob_fee?.isPristine);
	const setTob = useCallback(
		(v: string) => f.setFieldValue("tob_fee", v, { dontUpdateMeta: true }),
		[f],
	);

	useEffect(() => {
		if (!tobPristine) return;
		tobHint === null ? setTob("") : setTob(tobHint);
	}, [tobHint, tobPristine, setTob]);

	return (
		<div className="m-auto w-2/3 max-w-xl p-4 pt-8">
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
					<f.AppField name="broker_fee">
						{(field) => (
							<field.DecimalField label="Broker fee" currencyCode="eur" />
						)}
					</f.AppField>
					<f.AppField name="tob_fee">
						{(field) => <field.DecimalField label="TOB" currencyCode="eur" />}
					</f.AppField>
					<f.AppField name="executed_at">
						{(field) => <field.DateTimeField label="Execution time" />}
					</f.AppField>
					<f.AppForm>
						<f.SubmitButton label="Submit" />
					</f.AppForm>
				</FieldGroup>
			</form>
		</div>
	);
};

export default BuyPage;
