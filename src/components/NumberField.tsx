import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useFieldContext } from "@/hooks/form-context";

export const NumberField = ({ label }: { label: string }) => {
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
};
