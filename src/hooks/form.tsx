import { createFormHook } from "@tanstack/react-form";
import { DateTimeField } from "@/components/DateTimeField.tsx";
import { DecimalField } from "@/components/DecimalField.tsx";
import { ExchangePicker } from "@/components/ExchangePicker.tsx";
import { ListingPicker } from "@/components/ListingPicker.tsx";
import { NumberField } from "@/components/NumberField.tsx";
import { SelectField } from "@/components/SelectField.tsx";
import { SubmitButton } from "@/components/SubmitButton.tsx";
import { SwitchField } from "@/components/SwitchField.tsx";
import { TextField } from "@/components/TextField.tsx";
import { fieldContext, formContext } from "./form-context.tsx";
import { HoldingPicker } from "@/components/HoldingPicker.tsx";

export const { useAppForm } = createFormHook({
	fieldComponents: {
		TextField,
		NumberField,
		ListingPicker,
    HoldingPicker,
		ExchangePicker,
		DecimalField,
		DateTimeField,
		SwitchField,
		SelectField,
	},
	formComponents: {
		SubmitButton,
	},
	fieldContext,
	formContext,
});
