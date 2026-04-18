import type { HeaderContext } from "@tanstack/react-table";
import { Badge } from "@/components/ui/badge";
import { cn, formatCurrency, formatPercentage } from "@/lib/utils";
import type { HoldingRow } from "./columns";

export function AggPerformanceFooter({
	table,
	tfKey,
}: HeaderContext<HoldingRow, number> & { tfKey: "period" | "all_time" }) {
	const [gain, currentValue] = table
		.getFilteredRowModel()
		.rows.reduce<[number | null, number | null]>(
			(acc, r) => {
				const g = r.original.eur[tfKey].gain;
				const v = r.original.eur.current.value;

				// If either val is null the whole result becomes null
				if (g === null || v === null) {
					return [null, null];
				}

				// keep acc null
				if (acc[0] === null || acc[1] === null) {
					return [null, null];
				}

				return [acc[0] + g, acc[1] + v];
			},
			[0, 0],
		);

	let percentage = null;
	if (gain !== null && currentValue !== null) {
		// Base is universally (Current Value - Gain)
		// different pct calculation to the one on the backend,
		// but mathematically equivalent and easier to do with (filtered) table data
		const base = currentValue - gain;
		percentage = base !== 0 ? gain / base : 0;
	}

	const isPos = gain !== null && gain >= 0;

	if (gain === null || percentage === null) {
		return (
			<p className="text-end font-normal text-muted-foreground text-sm">
				Missing data
			</p>
		);
	}

	return (
		<div
			className={cn(
				"flex place-content-end items-center gap-1",
				isPos ? "text-emerald-700" : "text-rose-800",
			)}
		>
			<div className="flex flex-col place-items-end gap-1">
				<p className="font-medium text-base">
					{formatCurrency(gain, "EUR", { signDisplay: "exceptZero" })}
				</p>
				<Badge
					variant="ghost"
					className={cn(
						"font-normal text-sm",
						isPos ? "bg-emerald-50" : "bg-rose-50",
					)}
				>
					{formatPercentage(percentage, {
						signDisplay: "exceptZero",
					})}
				</Badge>
			</div>
		</div>
	);
}
