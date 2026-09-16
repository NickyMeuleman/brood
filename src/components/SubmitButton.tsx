import { Button } from "@/components/ui/button";
import { useFormContext } from "@/hooks/form-context";

export const SubmitButton = ({ label }: { label: string }) => {
	const form = useFormContext();

	return (
		<form.Subscribe
			selector={(state) => {
				return {
					canSubmit: state.canSubmit,
					isSubmitting: state.isSubmitting,
					isInvalid: !state.isValid && !state.isPristine,
					isPristine: state.isPristine,
				};
			}}
			children={({ canSubmit, isSubmitting, isInvalid, isPristine }) => {
				return (
					<Button
						type="submit"
						disabled={!canSubmit || isSubmitting || isPristine}
						aria-invalid={isInvalid}
					>
						{isSubmitting ? "..." : label}
					</Button>
				);
			}}
		/>
	);
};
