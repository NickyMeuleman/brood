import type { CellContext } from "@tanstack/react-table";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { cn, formatPercentage } from "@/lib/utils";
import type { HoldingRow } from "./columns";
import { MoneyCell } from "./MoneyCell";

export function AggPerformanceCell({ row }: CellContext<HoldingRow, number>) {
	const isPos = row.original.display.period.gain > 0;

	return (
		<div
			className={cn(
				"flex place-content-end items-center gap-1",
				row.original.display.period.gain >= 0
					? "text-emerald-600"
					: "text-rose-600",
			)}
		>
			{isPos ? <ArrowUpRight /> : <ArrowDownRight />}
			<div className="flex flex-col place-items-end gap-1">
				<MoneyCell
					value={row.original.display.period.gain}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.local.period.gain}
					originalCurrency={row.original.currency_code}
					className="font-medium text-base"
				/>
				<p className="font-normal text-sm">
					{formatPercentage(row.original.display.period.pct_gain)}
				</p>
			</div>
		</div>
	);
}
