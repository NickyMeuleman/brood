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
import { MIC_LABEL } from "@/lib/utils";
import { Item, ItemContent, ItemDescription, ItemTitle } from "./ui/item";

type ExchangeItem = {
	code: string;
	name: string;
};

export const ExchangePicker = ({
	label,
	exchanges,
}: {
	label: string;
	exchanges: string[];
}) => {
	const field = useFieldContext<string>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	const items: ExchangeItem[] = exchanges.map((code) => ({
		code,
		name: MIC_LABEL[code] || "",
	}));

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<Combobox
				id={field.name}
				items={items}
				value={items.find((item) => item.code === field.state.value) ?? null}
				itemToStringLabel={(item) => item.code}
				onValueChange={(item) => field.handleChange(item?.code ?? "")}
				onOpenChange={(open) => {
					if (!open) field.handleBlur();
				}}
				filter={(item, query) => {
					const q = query.toLowerCase();
					return (
						item.code.toLowerCase().includes(q) ||
						item.name.toLowerCase().includes(q)
					);
				}}
			>
				<ComboboxInput placeholder="Select an exchange" />
				<ComboboxContent>
					<ComboboxEmpty>No items found.</ComboboxEmpty>
					<ComboboxList>
						{(item) => (
							<ComboboxItem key={item.code} value={item}>
								<Item size="xs" className="p-0">
									<ItemContent>
										<ItemTitle>{item.code}</ItemTitle>
										{MIC_LABEL[item.code] ? (
											<ItemDescription>{MIC_LABEL[item.code]}</ItemDescription>
										) : null}
									</ItemContent>
								</Item>
							</ComboboxItem>
						)}
					</ComboboxList>
				</ComboboxContent>
			</Combobox>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
