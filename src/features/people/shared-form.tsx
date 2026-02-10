import { formOptions } from "@tanstack/react-form";
import { z } from "zod";

export const peopleFormOpts = formOptions({
	defaultValues: {
		username: "",
		age: 0,
	},
	validators: {
		onChange: z.object({
			username: z.string().min(3),
			age: z.number().min(13).multipleOf(2),
		}),
	},
	canSubmitWhenInvalid: false,
});
