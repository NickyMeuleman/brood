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
					isInvalid: !state.isValid,
				};
			}}
			children={({ canSubmit, isSubmitting, isInvalid }) => {
				return (
					<Button
						type="submit"
						disabled={!canSubmit || isSubmitting}
						aria-invalid={isInvalid}
					>
						{isSubmitting ? "..." : label}
					</Button>
				);
			}}
		/>
	);
};
