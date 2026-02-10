import { useFormContext } from "@/hooks/form-context";

export const SubmitButton = ({ label }: { label: string }) => {
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
					<button type="submit" disabled={!canSubmit} aria-invalid={isInvalid}>
						{isSubmitting ? "..." : label}
					</button>
				);
			}}
		/>
	);
};
