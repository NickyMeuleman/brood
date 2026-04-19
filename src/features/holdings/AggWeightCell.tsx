import type { CellContext } from "@tanstack/react-table";
import { formatPercentage } from "@/lib/utils";
import type { HoldingRow } from "./columns";

export function AggWeightCell({
	table,
	row,
	getValue,
}: CellContext<HoldingRow, number>) {
	const total = Number(table.options.meta?.totals?.value ?? 1);
	const weight = (row.original.eur.current.value ?? 0) / total;
	return (
		<div className="flex flex-col place-items-end gap-1">
			<p className="font-medium text-base">
				{getValue()}{" "}
				<span className="font-normal text-muted-foreground text-sm">
					shares
				</span>
			</p>
			<p className="font-normal text-muted-foreground text-sm">
				{formatPercentage(weight)}
			</p>
		</div>
	);
}
