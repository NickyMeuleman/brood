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
