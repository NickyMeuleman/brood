import { useStore } from "@tanstack/react-form";
import { FieldGroup } from "@/components/ui/field";
import { buyFormOpts, buySchema } from "@/features/trade/shared-form.tsx";
import { useAppForm } from "@/hooks/form";
import { useBuy } from "@/hooks/use-buy";
import { useListings } from "@/hooks/use-listings";

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
	const selectedListingId = useStore(
		f.store,
		(state) => state.values.listing_id,
	);
	const listingCurrency = listings.find(
		(l) => l.id === selectedListingId,
	)?.currency_code;

	const computeTobHint = (): string | null => {
		const { listing_id, quantity, unit_price } = f.store.state.values;
		const rate_str = listings.find((l) => l.id === listing_id)?.tob_rate_hint;
		if (!rate_str) return null;
		const qty = Number(quantity);
		const price = Number(unit_price);
		const rate = Number(rate_str);

		if (![qty, price, rate].every((n) => Number.isFinite(n) && n > 0))
			return null;
		const hint = (qty * price * Number(rate)).toFixed(2);

		return hint === "0.00" ? null : hint;
	};

	const autoFillTob = () => {
		if (!f.store.state.fieldMeta.tob_fee?.isPristine) return;
		const suggested = computeTobHint();
		if (suggested !== null) {
			f.setFieldValue("tob_fee", suggested, { dontUpdateMeta: true });
		}
	};

	const resetAndFillTob = () => {
		f.setFieldValue("tob_fee", "", { dontUpdateMeta: true });
		const suggested = computeTobHint();
		if (suggested !== null) {
			f.setFieldValue("tob_fee", suggested, { dontUpdateMeta: true });
		}
	};

	return (
		<div className="m-auto w-2/3 max-w-xl p-4 pt-8">
			<form
				onSubmit={(e) => {
					e.preventDefault();
					f.handleSubmit();
				}}
			>
				<FieldGroup>
					<f.AppField
						name="listing_id"
						listeners={{ onChange: resetAndFillTob }}
					>
						{(field) => (
							<field.ListingPicker
								label="Listing"
								listings={listings}
								isLoading={listingsLoading}
							/>
						)}
					</f.AppField>
					<f.AppField name="quantity" listeners={{ onChange: autoFillTob }}>
						{(field) => <field.DecimalField label="Quantity" />}
					</f.AppField>
					<f.AppField name="unit_price" listeners={{ onChange: autoFillTob }}>
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
