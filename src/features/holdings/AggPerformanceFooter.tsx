import type { HeaderContext } from "@tanstack/react-table";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { cn, formatCurrency, formatPercentage } from "@/lib/utils";
import type { HoldingRow } from "./columns";

export function AggPerformanceFooter({
	table,
}: HeaderContext<HoldingRow, number>) {
	const [gain, cost] = table
		.getFilteredRowModel()
		.rows.reduce(
			([gain, cost], r) => [
				gain + r.original.eur.period.gain,
				cost + r.original.eur.period.cost,
			],
			[0, 0],
		);
	const percentage = cost !== 0 ? gain / cost : 0;

	return (
		<div
			className={cn(
				"flex place-content-end items-center gap-1",
				gain >= 0 ? "text-emerald-600" : "text-rose-600",
			)}
		>
			{gain >= 0 ? <ArrowUpRight /> : <ArrowDownRight />}
			<div className="flex flex-col place-items-end gap-1">
				<p className="font-medium text-base">{formatCurrency(gain)}</p>
				<p className="font-normal text-sm">{formatPercentage(percentage)}</p>
			</div>
		</div>
	);
}
