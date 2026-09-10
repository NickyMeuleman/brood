import { useState } from "react";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { useFieldContext } from "@/hooks/form-context";
import { useCurrencies } from "@/hooks/use-currencies";
import { getCurrencySymbol } from "@/lib/utils";
import {
	InputGroup,
	InputGroupAddon,
	InputGroupInput,
	InputGroupText,
} from "./ui/input-group";
import { Select } from "./ui/select";

export const CurrencyField = ({
	label,
	initialCurrencyCode,
}: {
	label: string;
	initialCurrencyCode?: string;
}) => {
	const field = useFieldContext<string>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;
	const { data: currencies } = useCurrencies();
	const [currencyCode, setCurrencyCode] = useState(
		initialCurrencyCode || "EUR",
	);

	console.log(currencies);

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<InputGroup>
				{/* <Select */}
				{/* 	id={field.name} */}
				{/* 	itemToStringLabel={(val) => */}
				{/* 		options.find((i) => i.value === val)?.label ?? "" */}
				{/* 	} */}
				{/* 	value={field.state.value ?? null} */}
				{/* 	onValueChange={(v) => { */}
				{/* 		if (v !== null && v !== undefined) { */}
				{/* 			field.handleChange(v); */}
				{/* 		} */}
				{/* 	}} */}
				{/* 	onOpenChange={(open) => { */}
				{/* 		if (!open) field.handleBlur(); */}
				{/* 	}} */}
				{/* > */}
				{/* 	<SelectTrigger className="w-full"> */}
				{/* 		<SelectValue placeholder={placeholder ?? "Select…"} /> */}
				{/* 	</SelectTrigger> */}
				{/* 	<SelectContent> */}
				{/* 		<SelectGroup> */}
				{/* 			{options.map((opt) => ( */}
				{/* 				<SelectItem key={String(opt.value)} value={opt.value}> */}
				{/* 					{opt.label} */}
				{/* 				</SelectItem> */}
				{/* 			))} */}
				{/* 		</SelectGroup> */}
				{/* 	</SelectContent> */}
				{/* </Select> */}
				<InputGroupAddon>
					<InputGroupText>{getCurrencySymbol(currencyCode)}</InputGroupText>
				</InputGroupAddon>
				<InputGroupInput
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
			</InputGroup>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
