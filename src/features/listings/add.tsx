import { FieldGroup } from "@/components/ui/field";
import {
	listingAddFormOpts,
	listingAddSchema,
} from "@/features/listings/shared-form";
import { useAppForm } from "@/hooks/form";
import { useMics } from "@/hooks/use-mics";

const ListingAddPage = () => {
	const f = useAppForm({
		...listingAddFormOpts,
		onSubmit: ({ value }) => {
			const parsed = listingAddSchema.parse(value);
			console.log(parsed);
		},
		formId: "listing_add_form",
	});

	const { data: mics } = useMics();

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
						<f.AppField name="isin">
							{(field) => (
								<field.TextField
									label="ISIN (International Securities Identification Number)"
									description="Unique instrument identification"
								/>
							)}
						</f.AppField>
						<f.AppField name="exchange">
							{(field) => (
								<field.ExchangePicker label="Exchange" exchanges={mics ?? []} />
							)}
						</f.AppField>
						<f.AppForm>
							<f.SubmitButton label="Submit" />
						</f.AppForm>
					</FieldGroup>
				</form>
			</div>
		</div>
	);
};

export default ListingAddPage;
