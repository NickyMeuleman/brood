import { format } from "date-fns";
import { ChevronDownIcon } from "lucide-react";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import {
	Field,
	FieldError,
	FieldGroup,
	FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";
import { useFieldContext } from "@/hooks/form-context";

export function DatePickerTime({
	value,
	handleChange,
	handleBlur,
}: {
	value: string | undefined;
	handleChange: (v: string) => void;
	handleBlur: () => void;
}) {
	const [open, setOpen] = React.useState(false);
	const date = value ? new Date(value) : undefined;
	const time = date ? format(date, "HH:mm:ss") : "12:00:00";

	const handleDateSelect = (nextDate: Date | undefined) => {
		if (!nextDate) return;

		const base = date ?? new Date();
		const updated = new Date(nextDate);

		// Retain existing local time, swap the calendar date
		updated.setHours(base.getHours(), base.getMinutes(), base.getSeconds());

		handleChange(updated.toISOString());
		setOpen(false);
	};

	const handleTimeChange = (nextTime: string) => {
		const [hh, mm, ss] = nextTime.split(":").map(Number);
		const updated = date ?? new Date();

		// Retain existing calendar date, swap the local time
		updated.setHours(hh ?? 0, mm ?? 0, ss ?? 0);

		handleChange(updated.toISOString());
	};

	return (
		<FieldGroup className="max-w-xs flex-row">
			<Field>
				<Popover
					open={open}
					onOpenChange={(next) => {
						setOpen(next);
						if (!next) handleBlur();
					}}
				>
					<PopoverTrigger
						render={
							<Button
								variant="outline"
								className="w-32 justify-between font-normal"
							>
								{date ? format(date, "PPP") : "Select date"}
								<ChevronDownIcon data-icon="inline-end" />
							</Button>
						}
					/>
					<PopoverContent className="w-auto overflow-hidden p-0" align="start">
						<Calendar
							mode="single"
							selected={date}
							captionLayout="dropdown"
							defaultMonth={date}
							onSelect={handleDateSelect}
						/>
					</PopoverContent>
				</Popover>
			</Field>
			<Field className="w-auto">
				<Input
					type="time"
					step="1"
					className="min-w-24 appearance-none justify-center bg-background [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-calendar-picker-indicator]:appearance-none"
					value={time}
					onChange={(e) => {
						const time = e.target.value;
						handleTimeChange(time);
					}}
					onBlur={handleBlur}
				/>
			</Field>
		</FieldGroup>
	);
}

export const DateTimeField = ({ label }: { label: string }) => {
	const field = useFieldContext<string>();
	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;

	return (
		<Field data-invalid={isInvalid}>
			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
			<DatePickerTime
				value={field.state.value}
				handleChange={(v) => field.handleChange(v)}
				handleBlur={field.handleBlur}
			/>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
