import { formOptions } from "@tanstack/react-form";
import { z } from "zod";

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

export const listingAddSchema = z.object({
	isin: isinValidator,
});

export const listingAddFormOpts = formOptions({
	defaultValues: {
		isin: "",
	},
	validators: {
		// validate on mount or canSubmit starts as true
		onMount: listingAddSchema,
		onChange: listingAddSchema,
	},
	canSubmitWhenInvalid: false,
});
