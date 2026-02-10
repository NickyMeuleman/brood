import {
	Field,
	FieldDescription,
	FieldError,
	FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useFieldContext } from "@/hooks/form-context";

export const TextField = ({ label }: { label: string }) => {
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
};
