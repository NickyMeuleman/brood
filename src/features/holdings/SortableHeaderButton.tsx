import type { Column } from "@tanstack/react-table";
import { ArrowDown, ArrowUp, ChevronsUpDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export const SortableHeaderButton = <TData, TValue>({
	className,
	label,
	column,
	align = "start",
}: {
	className?: string;
	label: string;
	column: Column<TData, TValue>;
	align?: "start" | "end";
}) => {
	return (
		<div className={cn(align === "end" && "text-right")}>
			<Button
				variant="ghost"
				size="sm"
				// sm button has px-2.5
				className={cn(align === "start" ? "-ml-2.5" : "-mr-2.5", className)}
				onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
			>
				<span>{label}</span>
				{column.getIsSorted() === "desc" ? (
					<ArrowDown />
				) : column.getIsSorted() === "asc" ? (
					<ArrowUp />
				) : (
					<ChevronsUpDown />
				)}
			</Button>
		</div>
	);
};
