import { createFormHook } from "@tanstack/react-form";
import { DateTimeField } from "@/components/DateTimeField.tsx";
import { DecimalField } from "@/components/DecimalField.tsx";
import { ExchangePicker } from "@/components/ExchangePicker.tsx";
import { ListingPicker } from "@/components/ListingPicker.tsx";
import { NumberField } from "@/components/NumberField.tsx";
import { SubmitButton } from "@/components/SubmitButton.tsx";
import { TextField } from "@/components/TextField.tsx";
import { fieldContext, formContext } from "./form-context.tsx";

export const { useAppForm } = createFormHook({
	fieldComponents: {
		TextField,
		NumberField,
		ListingPicker,
		ExchangePicker,
		DecimalField,
		DateTimeField,
	},
	formComponents: {
		SubmitButton,
	},
	fieldContext,
	formContext,
});
