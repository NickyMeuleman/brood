import { useStore } from "@tanstack/react-form";
import type { MoneyInput } from "@/bindings";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
	InputGroup,
	InputGroupAddon,
	InputGroupInput,
	InputGroupText,
} from "@/components/ui/input-group";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { useFieldContext } from "@/hooks/form-context";
import { useCurrencies } from "@/hooks/use-currencies";
import { getCurrencySymbol } from "@/lib/utils";

export const MoneyField = ({
	label,
	currencyDisabled = false,
}: {
	label: string;
	currencyDisabled?: boolean;
}) => {
	const field = useFieldContext<MoneyInput | null>();
	const { data: currencies = [] } = useCurrencies();

	const fieldValue = field.state.value;
	const currency = fieldValue?.currency ?? "EUR";
	const amount = fieldValue?.amount ?? "";

	const { errors, isInvalid } = useStore(field.form.store, (state) => {
		const prefix = field.name;
		const entries = Object.entries(state.fieldMeta).filter(
			([key]) => key === prefix || key.startsWith(`${prefix}.`),
		);

		const errors = entries
			.flatMap(([, meta]) => meta?.errors ?? [])
			.filter(Boolean);

		const isTouched =
			entries.some(([, meta]) => meta?.isTouched) || state.isSubmitted;

		return {
			errors: errors,
			isInvalid: isTouched && errors.length > 0,
		};
	});

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<div className="grid grid-cols-[1fr_auto] gap-3">
				<InputGroup>
					<InputGroupAddon>
						<InputGroupText>{getCurrencySymbol(currency)}</InputGroupText>
					</InputGroupAddon>
					<InputGroupInput
						type="text"
						inputMode="decimal"
						autoCapitalize="off"
						spellCheck="false"
						placeholder="0.00"
						id={`${field.name}-amount`}
						name={`${field.name}-amount`}
						value={amount}
						onChange={(e) => {
							const val = e.target.value;
							if (/^-?\d*\.?\d*$/.test(val)) {
								field.handleChange({ currency, amount: val });
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
								field.handleChange({
									currency,
									amount: cleanVal,
								});
							}

							field.handleBlur();
						}}
						aria-invalid={isInvalid}
						autoComplete="off"
					/>
				</InputGroup>

				<Select
					id={`${field.name}-currency`}
					value={currency}
					disabled={currencyDisabled}
					onValueChange={(currency) => {
						if (currency) {
							field.handleChange({ currency, amount });
						}
					}}
					onOpenChange={(open) => {
						if (!open) field.handleBlur();
					}}
				>
					<SelectTrigger className="w-fit min-w-8" aria-invalid={isInvalid}>
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						<SelectGroup>
							{currencies.map((c) => (
								<SelectItem key={c.code} value={c.code}>
									{c.code}
								</SelectItem>
							))}
						</SelectGroup>
					</SelectContent>
				</Select>
			</div>
			{isInvalid && <FieldError errors={errors} />}
		</Field>
	);
};
