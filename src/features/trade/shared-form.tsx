import { formOptions } from "@tanstack/react-form";
import { z } from "zod";

const positiveDecimal = z
	.string()
	.min(1, "Required")
	.refine(
		(v) => !Number.isNaN(Number(v)) && Number(v) > 0,
		"Must be a positive number",
	);

const optionalDecimal = z
	.string()
	.transform((v) => (v === "" ? null : v))
	.refine(
		(v) =>
			v === null || v === "" || (!Number.isNaN(Number(v)) && Number(v) > 0),
		"Must be a positive number",
	);

export const buySchema = z.object({
	listing_id: z.int().positive(),
	quantity: positiveDecimal,
	executed_at: z.iso.datetime(),
	unit_price: positiveDecimal,
	broker_fee: optionalDecimal,
	tob_fee: optionalDecimal,
});

export const buyFormOpts = formOptions({
	defaultValues: {
		listing_id: 0,
		quantity: "",
		unit_price: "",
		broker_fee: "",
		tob_fee: "",
		executed_at: new Date().toISOString(),
	},
	validators: {
    // validate on mount or canSubmit starts as true
		onMount: buySchema,
		onChange: buySchema,
	},
	canSubmitWhenInvalid: false,
});
