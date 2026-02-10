import { useFieldContext } from "@/hooks/form-context";

export const NumberField = ({ label }: { label: string }) => {
	const field = useFieldContext<number>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<div data-invalid={isInvalid}>
			<label htmlFor={field.name}>{label}</label>
			<input
				type="number"
				id={field.name}
				name={field.name}
				value={field.state.value}
				onChange={(e) => field.handleChange(Number(e.target.value))}
				onBlur={field.handleBlur}
				aria-invalid={isInvalid}
				autoComplete="off"
			/>
			{isInvalid && (
				<p className="text-red-600">
					{field.state.meta.errors.map((e) => e.message).join(", ")}
				</p>
			)}
		</div>
	);
};
