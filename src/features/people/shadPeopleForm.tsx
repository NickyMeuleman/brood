import { createFormHook, createFormHookContexts } from "@tanstack/react-form";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import {
	Field,
	FieldDescription,
	FieldError,
	FieldGroup,
	FieldLabel,
} from "./ui/field";
import { Input } from "./ui/input";

const { fieldContext, formContext, useFieldContext, useFormContext } =
	createFormHookContexts();

// Allow us to bind components to the form to keep type safety but reduce production boilerplate
// Define this once to have a generator of consistent form instances throughout your app
const { useAppForm } = createFormHook({
	fieldComponents: {
		TextField({ label }: { label: string }) {
			// The `Field` infers that it should have a `value` type of `string`
			const field = useFieldContext<string>();
			const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;
			return (
				<Field data-invalid={isInvalid}>
					<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
					<FieldDescription>Your name goes in this field</FieldDescription>
					<Input
						id={field.name}
						name={field.name}
						value={field.state.value}
						onChange={(e) => field.handleChange(e.target.value)}
						onBlur={field.handleBlur}
						aria-invalid={isInvalid}
						autoComplete="off"
					/>
					{isInvalid && <FieldError errors={field.state.meta.errors} />}
				</Field>
			);
		},
		NumberField({ label }: { label: string }) {
			const field = useFieldContext<number>();
			const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

			return (
				<Field data-invalid={isInvalid}>
					<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
					<Input
						type="number"
						id={field.name}
						name={field.name}
						value={field.state.value}
						onChange={(e) => field.handleChange(Number(e.target.value))}
						onBlur={field.handleBlur}
						aria-invalid={isInvalid}
						autoComplete="off"
					/>
					{isInvalid && <FieldError errors={field.state.meta.errors} />}
				</Field>
			);
		},
	},
	formComponents: {
		SubmitButton({ label }: { label: string }) {
			const form = useFormContext();

			return (
				<form.Subscribe
					selector={(state) => ({
						canSubmit: state.isTouched && state.canSubmit,
						isSubmitting: state.isSubmitting,
						isInvalid: state.isTouched && !state.isValid,
					})}
					children={({ canSubmit, isSubmitting, isInvalid }) => {
						return (
							<Button
								type="submit"
								disabled={!canSubmit}
								aria-invalid={isInvalid}
							>
								{isSubmitting ? "..." : label}
							</Button>
						);
					}}
				/>
			);
		},
	},
	fieldContext,
	formContext,
});

const PeoplePage = () => {
	const f = useAppForm({
		defaultValues: {
			username: "",
			age: 0,
		},
		validators: {
			onChange: z.object({
				username: z.string().min(3),
				age: z.number().min(13).multipleOf(2),
			}),
		},
		onSubmit: ({ value }) => {
			console.log({ value });
		},
		canSubmitWhenInvalid: false,
	});
	console.log(f);

	return (
		<div className="pt-8 p-4 w-2/3 max-w-xl m-auto">
			<form
				onSubmit={(e) => {
					e.preventDefault();
					f.handleSubmit();
				}}
			>
				<FieldGroup>
					{/* Components are bound to `form` and `field` to ensure extreme type safety */}
					{/* Use `form.AppField` to render a component bound to a single field */}
					<f.AppField
						name="username"
						children={(field) => <field.TextField label="Full Name" />}
					/>
					{/* The "name" property will throw a TypeScript error if typo'd  */}
					<f.AppField
						name="age"
						children={(field) => <field.NumberField label="Age" />}
					/>
					{/* Components in `form.AppForm` have access to the form context */}
					<f.AppForm>
						<f.SubmitButton label="Submit" />
					</f.AppForm>
				</FieldGroup>
			</form>
		</div>
	);
};

export default PeoplePage;
