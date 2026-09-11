import { formOptions } from "@tanstack/react-form";
import { z } from "zod";
import type { MoneyInput } from "@/bindings";

// money is handled as string to preserve accuracy
const DECIMAL_RE = /^\d+(\.\d+)?$/;
const isZero = (v: string) => /^0+(\.0*)?$/.test(v);
const isPositiveDecimal = (v: string) => DECIMAL_RE.test(v) && !isZero(v);

const positiveDecimal = z
	.string()
	.trim()
	.min(1, "Required")
	.refine(isPositiveDecimal, "Must be a positive number");

const currencyCode = z.string().trim().min(1, "Currency required");

export const moneyInput = z.object({
	currency: currencyCode,
	amount: positiveDecimal,
}) satisfies z.ZodType<MoneyInput>;

export const optionalMoneyInput = moneyInput.nullable();

export const buySchema = z.object({
	listing_id: z.int().positive("Choose a listing"),
	broker_id: z.int().positive("Choose a broker"),
	quantity: positiveDecimal,
	executed_at: z.iso.datetime(),
	unit_price: moneyInput,
	broker_fee: optionalMoneyInput,
	tob_fee: optionalMoneyInput,
});

const buyDefaultValues: z.infer<typeof buySchema> = {
	listing_id: 0,
	broker_id: 0,
	quantity: "",
	unit_price: { currency: "EUR", amount: "" },
	broker_fee: null,
	tob_fee: null,
	executed_at: new Date().toISOString(),
};

export const buyFormOpts = formOptions({
	defaultValues: buyDefaultValues,
	validators: {
		// validate on mount or canSubmit starts as true
		onMount: buySchema,
		onChange: buySchema,
	},
	canSubmitWhenInvalid: false,
});

const optionalDecimal = z
	.string()
	.transform((v) => (v === "" ? null : v))
	.refine(
		(v) =>
			v === null || v === "" || (!Number.isNaN(Number(v)) && Number(v) > 0),
		"Must be a positive number",
	);
export const buildSellSchema = (maxQty?: string) =>
	z.object({
		listing_id: z.int().positive("Choose a holding"),
		broker_id: z.int().positive("Choose a broker"),
		quantity: positiveDecimal.refine(
			(v) => maxQty === undefined || Number(v) <= Number(maxQty),
			"Cannot exceed the amount you held at the selected execution time.",
		),
		executed_at: z.iso.datetime(),
		unit_price: positiveDecimal,
		broker_fee: optionalDecimal,
		tob_fee: optionalDecimal,
	});

export const sellFormOpts = formOptions({
	defaultValues: {
		listing_id: 0,
		broker_id: 0,
		quantity: "",
		unit_price: "",
		broker_fee: "",
		tob_fee: "",
		executed_at: new Date().toISOString(),
	},
	validators: {
		// validate on mount or canSubmit starts as true
		onMount: buildSellSchema(undefined),
		onChange: buildSellSchema(undefined),
	},
	canSubmitWhenInvalid: false,
});
