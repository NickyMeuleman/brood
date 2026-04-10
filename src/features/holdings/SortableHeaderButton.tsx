import type { HeaderContext } from "@tanstack/react-table";
import { ArrowDown, ArrowUp, ChevronsUpDown } from "lucide-react";
import type { Period } from "@/bindings";
import { Button } from "@/components/ui/button";
import { cn, PERIOD_LABEL } from "@/lib/utils";
import type { HoldingRow } from "./columns";

function Label({
	label,
	period,
	showPeriod,
}: {
	label?: string;
	period?: Period;
	showPeriod?: boolean;
}) {
	return (
		<p className="flex items-baseline gap-0.5">
			<span>{label}</span>
			{showPeriod && period && period !== "AllTime" ? (
				<span className="text-muted-foreground text-xs uppercase tracking-widest">
					({PERIOD_LABEL[period]})
				</span>
			) : null}
		</p>
	);
}

export function SortableHeaderButton<TValue>({
	column,
	table,
}: HeaderContext<HoldingRow, TValue>) {
	const { label, align = "start", showPeriod } = column.columnDef.meta ?? {};
	const period = table.options.meta?.period;

	return (
		<div className={cn(align === "end" && "text-right")}>
			<Button
				variant="ghost"
				size="sm"
				// sm button has px-2.5
				className={cn(align === "end" ? "-mr-2.5" : "-ml-2.5")}
				onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
			>
				<Label label={label} period={period} showPeriod={showPeriod} />
				{column.getIsSorted() === "desc" ? (
					<ArrowDown data-icon="inline-end" />
				) : column.getIsSorted() === "asc" ? (
					<ArrowUp data-icon="inline-end" />
				) : (
					<ChevronsUpDown data-icon="inline-end" />
				)}
			</Button>
		</div>
	);
}
