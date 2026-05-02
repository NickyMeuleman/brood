import type { HeaderContext } from "@tanstack/react-table";
import { Badge } from "@/components/ui/badge";
import { cn, formatCurrency, formatPercentage } from "@/lib/utils";
import type { HoldingRow } from "./columns";

export function AggPerformanceFooter({
	table,
	tfKey,
}: HeaderContext<HoldingRow, number | null> & {
	tfKey: "period" | "all_time";
}) {
	const [gain, base] = table
		.getFilteredRowModel()
		.rows.reduce<[number | null, number | null]>(
			(acc, r) => {
				const gain = r.original.eur[tfKey].perf.gain;
				const cost = r.original.eur[tfKey].perf.cost;

				// If either val is null the whole result becomes null
				if (gain === null || cost === null) {
					return [null, null];
				}

				// keep acc null
				if (acc[0] === null || acc[1] === null) {
					return [null, null];
				}

				const startValue = r.original.eur[tfKey].start.value ?? 0;
				return [acc[0] + gain, acc[1] + cost + startValue];
			},
			[0, 0],
		);

	let percentage = null;
	if (gain !== null && base !== null) {
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
