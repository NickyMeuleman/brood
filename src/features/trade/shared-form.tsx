import { formOptions } from "@tanstack/react-form";
import { z } from "zod";

// money is handled as string to preserve accuracy
// because else 0.1 + 0.2 != 0.3, floating points! amirite?
const DECIMAL_SHAPE_RE = /^-?(?:\d+(?:\.\d*)?|\.\d+)$/;

const decimal = z
	.string()
	.trim()
	.refine(
		(v) => v === "" || DECIMAL_SHAPE_RE.test(v),
		"Must be a valid number",
	);

const nonNegativeDecimal = decimal.refine(
	(v) => Number(v) >= 0,
	"Cannot be negative",
);

const positiveDecimal = nonNegativeDecimal
	.min(1, "Required")
	.refine((v) => Number(v) > 0, "Must be greater than zero");

export const buySchema = z.object({
	listing_id: z.int().positive("Choose a listing"),
	broker_id: z.int().positive("Choose a broker"),
	quantity: positiveDecimal,
	executed_at: z.iso.datetime(),
	unit_price: z.object({
		currency: z.currencyCode(),
		amount: positiveDecimal,
	}),
	broker_fee: z
		.object({
			currency: z.currencyCode(),
			amount: nonNegativeDecimal,
		})
		.nullable(),
	tob_fee: z
		.object({
			currency: z.currencyCode(),
			amount: nonNegativeDecimal,
		})
		.nullable(),
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
		onChange: buySchema,
	},
	canSubmitWhenInvalid: false,
});

export const buildSellSchema = (maxQty?: string) =>
	z.object({
		listing_id: z.int().positive("Choose a listing"),
		broker_id: z.int().positive("Choose a broker"),
		quantity: positiveDecimal.refine(
			(v) => maxQty === undefined || Number(v) <= Number(maxQty),
			"Cannot exceed the amount you held at the selected execution time.",
		),
		executed_at: z.iso.datetime(),
		unit_price: z.object({
			currency: z.currencyCode(),
			amount: positiveDecimal,
		}),
		broker_fee: z
			.object({
				currency: z.currencyCode(),
				amount: nonNegativeDecimal,
			})
			.nullable(),
		tob_fee: z
			.object({
				currency: z.currencyCode(),
				amount: nonNegativeDecimal,
			})
			.nullable(),
	});

const sellDefaultValues: z.infer<ReturnType<typeof buildSellSchema>> = {
	listing_id: 0,
	broker_id: 0,
	quantity: "",
	unit_price: { currency: "EUR", amount: "" },
	broker_fee: null,
	tob_fee: null,
	executed_at: new Date().toISOString(),
};

export const sellFormOpts = formOptions({
	defaultValues: sellDefaultValues,
	validators: {
		onChange: buildSellSchema(undefined),
	},
	canSubmitWhenInvalid: false,
});
