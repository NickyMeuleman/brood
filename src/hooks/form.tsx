import { createFormHook } from "@tanstack/react-form";
import { NumberField } from "@/components/NumberField.tsx";
import { SubmitButton } from "@/components/SubmitButton.tsx";
import { TextField } from "@/components/TextField.tsx";
import { fieldContext, formContext } from "./form-context.tsx";

export const { useAppForm } = createFormHook({
	fieldComponents: {
		TextField,
		NumberField,
	},
	formComponents: {
		SubmitButton,
	},
	fieldContext,
	formContext,
});
