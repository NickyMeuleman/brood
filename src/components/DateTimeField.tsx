// import { format } from "date-fns";
// import { ChevronDownIcon } from "lucide-react";
// import * as React from "react";
// import { Button } from "@/components/ui/button";
// import { Calendar } from "@/components/ui/calendar";
// import {
// 	Field,
// 	FieldError,
// 	FieldGroup,
// 	FieldLabel,
// } from "@/components/ui/field";
// import { Input } from "@/components/ui/input";
// import {
// 	Popover,
// 	PopoverContent,
// 	PopoverTrigger,
// } from "@/components/ui/popover";
// import { useFieldContext } from "@/hooks/form-context";
// import { cn } from "@/lib/utils";
//
// export function DatePickerTime({
// 	value,
// 	onChange,
// }: {
// 	value: string | undefined;
// 	onChange: (v: string) => void;
// }) {
// 	const [open, setOpen] = React.useState(false);
//
// 	const parsed = value ? new Date(value) : undefined;
//
// 	const [date, setDate] = React.useState<Date | undefined>(parsed);
// 	const [time, setTime] = React.useState(
// 		parsed ? format(parsed, "HH:mm:ss") : "10:30:00",
// 	);
//
// 	// Combine date + time → local ISO string
// 	const updateValue = (d: Date | undefined, t: string) => {
// 		if (!d) return;
//
// 		const [hh, mm, ss] = t.split(":").map(Number);
//
// 		const combined = new Date(d);
// 		combined.setHours(hh ?? 0, mm ?? 0, ss ?? 0);
//
// 		onChange(combined.toISOString());
// 	};
//
// 	return (
// 		<FieldGroup className="max-w-xs flex-row">
// 			<Field>
// 				<FieldLabel className="text-xs" htmlFor="date-picker-optional">
// 					Date
// 				</FieldLabel>
// 				<Popover open={open} onOpenChange={setOpen}>
// 					<PopoverTrigger
// 						render={
// 							<Button
// 								variant="outline"
// 								id="date-picker-optional"
// 								className="w-32 justify-between font-normal"
// 							>
// 								{date ? format(date, "PPP") : "Select date"}
// 								<ChevronDownIcon data-icon="inline-end" />
// 							</Button>
// 						}
// 					/>
// 					<PopoverContent className="w-auto overflow-hidden p-0" align="start">
// 						<Calendar
// 							mode="single"
// 							selected={date}
// 							captionLayout="dropdown"
// 							defaultMonth={date}
// 							onSelect={(date) => {
// 								setDate(date);
// 								updateValue(date, time);
// 								setOpen(false);
// 							}}
// 						/>
// 					</PopoverContent>
// 				</Popover>
// 			</Field>
// 			<Field className="w-32">
// 				<FieldLabel className="text-xs" htmlFor="time-picker-optional">
// 					Time
// 				</FieldLabel>
// 				<Input
// 					type="time"
// 					id="time-picker-optional"
// 					step="1"
// 					className="appearance-none bg-background [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-calendar-picker-indicator]:appearance-none"
// 					value={time}
// 					onChange={(e) => {
// 						const time = e.target.value;
// 						setTime(time);
// 						updateValue(date, time);
// 					}}
// 				/>
// 			</Field>
// 		</FieldGroup>
// 	);
// }
// export const DateTimeField = ({ label }: { label: string }) => {
// 	const field = useFieldContext<string>();
// 	const isInvalid = field.state.meta.isTouched && !field.state.meta.isValid;
//
// 	return (
// 		<Field data-invalid={isInvalid}>
// 			<FieldLabel htmlFor={field.name}>{label}</FieldLabel>
// 			{/* <Input */}
// 			{/* 	type="datetime-local" */}
// 			{/* 	id={field.name} */}
// 			{/* 	name={field.name} */}
// 			{/* 	value={field.state.value} */}
// 			{/* 	onChange={(e) => field.handleChange(e.target.value)} */}
// 			{/* 	onBlur={field.handleBlur} */}
// 			{/* 	aria-invalid={isInvalid} */}
// 			{/* 	autoComplete="off" */}
// 			{/* /> */}
// 			<DatePickerTime
// 				value={field.state.value}
// 				onChange={(v) => field.handleChange(v)}
// 			/>
// 			{isInvalid && <FieldError errors={field.state.meta.errors} />}
// 		</Field>
// 	);
// };

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
import { cn } from "@/lib/utils";

export function DatePickerTime({
	value,
	onChange,
}: {
	value: string | undefined;
	onChange: (v: string) => void;
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

		onChange(updated.toISOString());
		setOpen(false);
	};

	const handleTimeChange = (nextTime: string) => {
		const [hh, mm, ss] = nextTime.split(":").map(Number);
		const updated = date ?? new Date();

		// Retain existing calendar date, swap the local time
		updated.setHours(hh ?? 0, mm ?? 0, ss ?? 0);

		onChange(updated.toISOString());
	};

	return (
		<FieldGroup className="max-w-xs flex-row">
			<Field>
				<FieldLabel className="text-xs" htmlFor="date-picker-optional">
					Date
				</FieldLabel>
				<Popover open={open} onOpenChange={setOpen}>
					<PopoverTrigger
						render={
							<Button
								variant="outline"
								id="date-picker-optional"
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
				<FieldLabel className="text-xs" htmlFor="time-picker-optional">
					Time
				</FieldLabel>
				<Input
					type="time"
					id="time-picker-optional"
					step="1"
					className="min-w-24 appearance-none justify-center bg-background [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-calendar-picker-indicator]:appearance-none"
					value={time}
					onChange={(e) => {
						const time = e.target.value;
						handleTimeChange(time);
					}}
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
				onChange={(v) => field.handleChange(v)}
			/>
			{isInvalid && <FieldError errors={field.state.meta.errors} />}
		</Field>
	);
};
