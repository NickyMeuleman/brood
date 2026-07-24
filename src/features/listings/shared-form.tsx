import { formOptions } from "@tanstack/react-form";
import { z } from "zod";
import type { InstrumentType, Replication } from "@/bindings";

const isinValidator = z
	.string()
	.length(12, { abort: true })
	.regex(/^[A-Z]{2}[A-Z0-9]{9}[0-9]$/, {
		message:
			"ISIN must be 2 letters, 9 letters/digits, then 1 digit. Letters must be capital.",
		abort: true,
	})
	.refine((val) => isValidLuhn(val), {
		message: "Invalid ISIN checksum",
	});

function isValidLuhn(isin: string): boolean {
	let numstr = "";
	for (const c of isin) {
		const code = c.charCodeAt(0);
		if (code >= 65 && code <= 90) {
			numstr += (code - 55).toString();
		} else {
			numstr += c;
		}
	}

	let sum = 0;
	let double = false;
	for (let i = numstr.length - 1; i >= 0; i--) {
		let d = parseInt(numstr.charAt(i), 10);
		if (double) {
			d *= 2;
			if (d > 9) d -= 9;
		}
		sum += d;
		double = !double;
	}

	return sum % 10 === 0;
}

export function isValidIsin(isin: string): boolean {
	return isinValidator.safeParse(isin).success;
}

export const INSTRUMENT_TYPES: Record<InstrumentType, string> = {
	ETF: "Exchange Traded Fund",
	FUND: "Fund",
	STOCK: "Stock",
	BOND: "Bond",
	OTHER: "Other",
};

export const REPLICATIONS: Record<Replication, string> = {
	PHYSICAL: "Physical",
	SYNTHETIC: "Synthetic",
};

const instrumentFieldsSchema = z.object({
	name: z.string().trim().min(1, "Required"),
	issuer: z
		.string()
		.trim()
		.transform((v) => (v === "" ? null : v)),
	instrument_type: z.enum(Object.keys(INSTRUMENT_TYPES) as InstrumentType[]),
	replication: z
		.enum(Object.keys(REPLICATIONS) as Replication[])
		.or(z.literal(""))
		.transform((v) => (v === "" ? null : v)),
	fsma_registered: z.boolean(),
	accumulating: z.boolean().optional(),
	domicile: z.string().trim(),
	subject_to_cgt: z.boolean(),
});

const listingFieldsSchema = z.object({
	ticker: z.string().uppercase().min(1, "Required"),
	mic: z.string().uppercase().min(1, "Choose an exchange"),
	currency: z.string().uppercase().min(1, "Required"),
});

export const listingAddSchema = z.object({
	isin: isinValidator,
	instrument: instrumentFieldsSchema,
	listing: listingFieldsSchema,
});

export const listingAddFormOpts = formOptions({
	defaultValues: {
		isin: "",
		instrument: {
			name: "",
			issuer: "",
			instrument_type: "",
			replication: "",
			fsma_registered: false,
			accumulating: false,
			domicile: "",
			subject_to_cgt: true,
		},
		listing: {
			mic: "",
			ticker: "",
			currency: "",
		},
	},
	validators: {
		// validate on mount or canSubmit starts as true
		onMount: listingAddSchema,
		onChange: listingAddSchema,
	},
	canSubmitWhenInvalid: false,
});
