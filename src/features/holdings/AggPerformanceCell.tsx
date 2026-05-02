import type { CellContext } from "@tanstack/react-table";
import { Badge } from "@/components/ui/badge";
import { cn, formatPercentage } from "@/lib/utils";
import type { HoldingRow } from "./columns";
import { MoneyCell } from "./MoneyCell";

export function AggPerformanceCell({
	row,
	tfKey,
}: CellContext<HoldingRow, number | null> & { tfKey: "period" | "all_time" }) {
	const gain = row.original.display[tfKey].perf.gain;
	const pct_gain = row.original.display[tfKey].perf.pct_gain;

	if (gain === null || pct_gain === null) {
		return (
			<p className="text-end font-normal text-muted-foreground text-sm">
				Missing data
			</p>
		);
	}
	const isPos = gain > 0;

	return (
		<div
			className={cn(
				"flex place-content-end items-center gap-1",
				isPos ? "text-emerald-700" : "text-rose-700",
			)}
		>
			<div className="flex flex-col place-items-end gap-1">
				<MoneyCell
					value={gain}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={gain}
					originalCurrency={row.original.currency_code}
					numFormatOpts={{ signDisplay: "exceptZero" }}
					className="font-medium text-base"
				/>
				<Badge
					variant="ghost"
					className={cn(
						"font-normal text-sm",
						isPos ? "bg-emerald-50" : "bg-rose-50",
					)}
				>
					{formatPercentage(pct_gain, {
						signDisplay: "exceptZero",
					})}
				</Badge>
			</div>
		</div>
	);
}
