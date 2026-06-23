import { FieldGroup } from "@/components/ui/field";
import { buyFormOpts } from "@/features/trade/shared-form.tsx";
import { useAppForm } from "@/hooks/form";
import { useBuy } from "@/hooks/use-buy";

const BuyPage = () => {
	const buy = useBuy();
	const f = useAppForm({
		...buyFormOpts,
		onSubmit: ({ value }) => {
			buy.mutate(value);
		},
	});

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
						{(field) => <field.ListingPicker label="Listing" />}
					</f.AppField>
					<f.AppField name="quantity">
						{(field) => <field.DecimalField label="Quantity" />}
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
