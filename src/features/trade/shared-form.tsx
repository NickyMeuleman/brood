import { formOptions } from "@tanstack/react-form";
import { z } from "zod";

export const buyFormOpts = formOptions({
	defaultValues: {
		listing_id: 1,
		quantity: "",
		executed_at: "",
	},
	validators: {
		onBlur: z.object({
			listing_id: z.int().positive(),
			quantity: z
				.string()
				.min(1, "Required")
				.regex(/^-?\d+(\.\d+)?$/, "Must be a valid decimal (e.g., '123.45')")
				.refine(
					(val) => {
						const isZero = /^-?0+(\.0+)?$/.test(val);
						return !isZero;
					},
					{
						message: "Amount cannot be zero",
					},
				),
			executed_at: z.string(),
		}),
	},
	canSubmitWhenInvalid: false,
});
