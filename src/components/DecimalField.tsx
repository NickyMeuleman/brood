import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useFieldContext } from "@/hooks/form-context";

export const DecimalField = ({ label }: { label: string }) => {
	const field = useFieldContext<string>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<Input
				type="text"
				inputMode="decimal"
				autoCapitalize="off"
				spellCheck="false"
				placeholder="0.00"
				id={field.name}
				name={field.name}
				value={field.state.value}
				onChange={(e) => {
					const val = e.target.value;
					if (/^-?\d*\.?\d*$/.test(val)) {
						field.handleChange(val);
					}
				}}
				onBlur={(e) => {
					let cleanVal = e.target.value;
					if (cleanVal === "-" || cleanVal === "-.") cleanVal = "";
					else if (cleanVal.endsWith(".")) cleanVal = cleanVal.slice(0, -1);
					else if (cleanVal.startsWith(".")) cleanVal = `0${cleanVal}`;
					else if (cleanVal.startsWith("-."))
						cleanVal = `-0.${cleanVal.slice(2)}`;

					if (cleanVal !== e.target.value) {
						field.handleChange(cleanVal);
					}

					field.handleBlur();
				}}
				aria-invalid={isInvalid}
				autoComplete="off"
			/>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
