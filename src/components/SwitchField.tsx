import type { ReactNode } from "react";
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
} from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import { useFieldContext } from "@/hooks/form-context";

export const SwitchField = ({
	label,
	description,
	disabled,
}: {
	label: string;
	description?: ReactNode;
	disabled?: boolean;
}) => {
	const field = useFieldContext<boolean>();

	return (
		<Field orientation="horizontal">
			<Switch
				id={field.name}
				checked={field.state.value}
				onCheckedChange={(v) => field.handleChange(v)}
				disabled={disabled}
			/>
			<FieldContent>
				<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
				{description ? (
					<FieldDescription>{description}</FieldDescription>
				) : null}
			</FieldContent>
		</Field>
	);
};
