import { Button } from "@/components/ui/button";
import { useFormContext } from "@/hooks/form-context";

export const SubmitButton = ({ label }: { label: string }) => {
	const form = useFormContext();

	return (
		<form.Subscribe
			selector={(state) => {
				return {
					canSubmit: state.isTouched && state.canSubmit,
					isSubmitting: state.isSubmitting,
					isInvalid: !state.isPristine && !state.isValid,
				};
			}}
			children={({ canSubmit, isSubmitting, isInvalid }) => {
				return (
					<Button type="submit" disabled={!canSubmit} aria-invalid={isInvalid}>
						{isSubmitting ? "..." : label}
					</Button>
				);
			}}
		/>
	);
};
