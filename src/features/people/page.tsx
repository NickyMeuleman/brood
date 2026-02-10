import { peopleFormOpts } from "@/features/people/shared-form.tsx";
import { useAppForm } from "@/hooks/form";

const PeoplePage = () => {
	const f = useAppForm({
		...peopleFormOpts,
		onSubmit: ({ value }) => {
			console.log({ value });
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
				<div className="fieldgroup">
					<f.AppField name="username">
						{(field) => <field.TextField label="Full Name" />}
					</f.AppField>
					<f.AppField name="age">
						{(field) => <field.NumberField label="Age" />}
					</f.AppField>
					<f.AppForm>
						<f.SubmitButton label="Submit" />
					</f.AppForm>
				</div>
			</form>
		</div>
	);
};

export default PeoplePage;
