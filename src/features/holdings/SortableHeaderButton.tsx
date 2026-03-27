import type { Column } from "@tanstack/react-table";
import { ArrowDown, ArrowUp, ChevronsUpDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export const SortableHeaderButton = <TData, TValue>({
	className,
	column,
	align = "start",
}: {
	className?: string;
	column: Column<TData, TValue>;
	align?: "start" | "end";
}) => {
	const label = column.columnDef.meta?.label ?? column.id;
	return (
		<div className={cn(align === "end" && "text-right")}>
			<Button
				variant="ghost"
				size="sm"
				// sm button has px-2.5
				className={cn(align === "end" ? "-mr-2.5" : "-ml-2.5", className)}
				onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
			>
				{label}
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
};
