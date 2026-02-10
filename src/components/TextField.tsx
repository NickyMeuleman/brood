import { useFieldContext } from "@/hooks/form-context";

export const TextField = ({ label }: { label: string }) => {
	const field = useFieldContext<string>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<div data-invalid={isInvalid}>
			<label htmlFor={field.name}>{label}</label>
			<p>Your name goes in this field</p>
			<input
				id={field.name}
				name={field.name}
				value={field.state.value}
				onChange={(e) => field.handleChange(e.target.value)}
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
