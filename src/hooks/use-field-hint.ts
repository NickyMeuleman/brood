/** biome-ignore-all lint/suspicious/noExplicitAny: tanstack be using a lot of generics */
import {
	type DeepKeys,
	type DeepValue,
	type FormApi,
	useStore,
} from "@tanstack/react-form";
import { useEffect } from "react";

export type AnyFormApi<TFormData> = FormApi<
	TFormData,
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export interface FieldHintOptions {
	enabled?: boolean;
}

export function applyHint<TFormData, TName extends DeepKeys<TFormData>>(
	form: AnyFormApi<TFormData>,
	name: TName,
	value: DeepValue<TFormData, TName>,
) {
	form.setFieldValue(name, value);
	form.setFieldMeta(name, (prev) => ({
		...prev,
		isTouched: false,
		isDirty: false,
		isPristine: true,
	}));
}

export function setFieldHint<TFormData, TName extends DeepKeys<TFormData>>(
	form: AnyFormApi<TFormData>,
	name: TName,
	value: DeepValue<TFormData, TName> | undefined | null,
	options: FieldHintOptions = {},
) {
	const { enabled = true } = options;
	if (!enabled || value === undefined || value === null) return;

	const meta = form.getFieldMeta?.(name) ?? form.store.state.fieldMeta[name];
	const isPristine = meta?.isPristine ?? true;

	if (!isPristine) return;
	applyHint(form, name, value);
}

// fill a field with a hint only if the user hasn't touched that field
export function useFieldHint<TFormData, TName extends DeepKeys<TFormData>>(
	form: AnyFormApi<TFormData>,
	name: TName,
	value: DeepValue<TFormData, TName> | undefined | null,
	options: FieldHintOptions = {},
) {
	const { enabled = true } = options;

	const isPristine = useStore(
		form.store,
		(s) => s.fieldMeta[name]?.isPristine ?? true,
	);

	useEffect(() => {
		if (!enabled || !isPristine || value === undefined || value === null) {
			return;
		}
		applyHint(form, name, value);
	}, [form, name, value, isPristine, enabled]);
}
