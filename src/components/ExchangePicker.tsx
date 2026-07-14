import {
	Combobox,
	ComboboxContent,
	ComboboxEmpty,
	ComboboxInput,
	ComboboxItem,
	ComboboxList,
} from "@/components/ui/combobox";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { useFieldContext } from "@/hooks/form-context";

export const ExchangePicker = ({
	label,
	exchanges,
}: {
	label: string;
	exchanges: string[];
}) => {
	const field = useFieldContext<string>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<Combobox
				id={field.name}
				items={exchanges}
				value={field.state.value}
				onValueChange={(s) => field.handleChange(s ?? "")}
				onOpenChange={(open) => {
					if (!open) field.handleBlur();
				}}
			>
				<ComboboxInput placeholder="Select an exchange" />
				<ComboboxContent>
					<ComboboxEmpty>No items found.</ComboboxEmpty>
					<ComboboxList>
						{(item) => (
							<ComboboxItem key={item} value={item}>
								{item}
							</ComboboxItem>
						)}
					</ComboboxList>
				</ComboboxContent>
			</Combobox>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
