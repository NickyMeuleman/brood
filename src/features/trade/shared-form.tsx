import { formOptions } from "@tanstack/react-form";
import { z } from "zod";
import type { MoneyInput } from "@/bindings";

const positiveDecimal = z
	.string()
	.min(1, "Required")
	.refine(
		(v) => !Number.isNaN(Number(v)) && Number(v) > 0,
		"Must be a positive number",
	);

export const positiveMoneyInput = z
	.object({
		currency: z.string().min(1, "Currency required"),
		amount: z.string(),
	})
	.transform((val, ctx): MoneyInput => {
		const trimmed = val.amount.trim();
		if (!trimmed) {
			ctx.addIssue({
				code: "custom",
				message: "Required",
				path: ["amount"],
			});
			return z.NEVER;
		}

		const num = Number(trimmed);
		if (Number.isNaN(num) || num <= 0) {
			ctx.addIssue({
				code: "custom",
				message: "Must be a positive number",
				path: ["amount"],
			});
			return z.NEVER;
		}

		return { currency: val.currency, amount: trimmed };
	});

export const optionalMoneyInput = z
	.object({
		currency: z.string().min(1, "Currency required"),
		amount: z.string(),
	})
	.transform((val, ctx): MoneyInput | null => {
		const trimmed = val.amount.trim();
		if (!trimmed) return null;

		const num = Number(trimmed);
		if (Number.isNaN(num) || num <= 0) {
			ctx.addIssue({
				code: "custom",
				message: "Must be a positive number",
				path: ["amount"],
			});
			return z.NEVER;
		}

		return { currency: val.currency, amount: trimmed };
	});

export const buySchema = z.object({
	listing_id: z.int().positive("Choose a listing"),
	broker_id: z.int().positive("Choose a broker"),
	quantity: positiveDecimal,
	executed_at: z.iso.datetime(),
	unit_price: positiveMoneyInput,
	broker_fee: optionalMoneyInput,
	tob_fee: optionalMoneyInput,
});

export const buyFormOpts = formOptions({
	defaultValues: {
		listing_id: 0,
		broker_id: 0,
		quantity: "",
		unit_price: { currency: "EUR", amount: "" },
		broker_fee: { currency: "EUR", amount: "" },
		tob_fee: { currency: "EUR", amount: "" },
		executed_at: new Date().toISOString(),
	},
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
