import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { useFieldContext } from "@/hooks/form-context";

export function SelectField<T>({
	label,
	options,
	placeholder,
}: {
	label: string;
	options: { value: T; label: string }[];
	placeholder?: string;
}) {
	const field = useFieldContext<T>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<Select
				id={field.name}
				itemToStringLabel={(val) =>
					options.find((i) => i.value === val)?.label ?? ""
				}
				value={field.state.value ?? null}
				onValueChange={(v) => {
					if (v !== null && v !== undefined) {
						field.handleChange(v);
					}
				}}
				onOpenChange={(open) => {
					if (!open) field.handleBlur();
				}}
			>
				<SelectTrigger className="w-full">
					<SelectValue placeholder={placeholder ?? "Select…"} />
				</SelectTrigger>
				<SelectContent>
					<SelectGroup>
						{options.map((opt) => (
							<SelectItem key={String(opt.value)} value={opt.value}>
								{opt.label}
							</SelectItem>
						))}
					</SelectGroup>
				</SelectContent>
			</Select>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
}
