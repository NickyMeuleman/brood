import { useEffect } from "react";
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
	initialCurrencyCode = "EUR",
	currencyDisabled = false,
}: {
	label: string;
	initialCurrencyCode?: string;
	currencyDisabled?: boolean;
}) => {
	const field = useFieldContext<MoneyInput>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;
	const { data: currencies = [] } = useCurrencies();

	useEffect(() => {
		if (
			initialCurrencyCode &&
			field.state.value.currency !== initialCurrencyCode
		) {
			field.handleChange({
				amount: field.state.value.amount,
				currency: initialCurrencyCode,
			});
		}
	}, [
		initialCurrencyCode,
		field.state.value.currency,
		field.state.value.amount,
		field.handleChange,
	]);

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<div className="grid grid-cols-[1fr_auto] gap-3">
				<InputGroup>
					<InputGroupAddon>
						<InputGroupText>
							{getCurrencySymbol(
								field.state.value.currency ?? initialCurrencyCode,
							)}
						</InputGroupText>
					</InputGroupAddon>
					<InputGroupInput
						type="text"
						inputMode="decimal"
						autoCapitalize="off"
						spellCheck="false"
						placeholder="0.00"
						id={field.name}
						name={field.name}
						value={field.state.value.amount}
						onChange={(e) => {
							const val = e.target.value;
							if (/^-?\d*\.?\d*$/.test(val)) {
								field.handleChange({
									currency: field.state.value.currency,
									amount: val,
								});
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
									currency: field.state.value.currency,
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
					id={field.state.value.currency}
					value={field.state.value.currency}
					disabled={currencyDisabled}
					onValueChange={(currency) => {
						if (currency) {
							field.handleChange({
								currency,
								amount: field.state.value.amount,
							});
						}
					}}
					onOpenChange={(open) => {
						if (!open) field.handleBlur();
					}}
				>
					<SelectTrigger className="w-fit min-w-8">
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
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
