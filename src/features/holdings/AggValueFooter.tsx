import type { HeaderContext } from "@tanstack/react-table";
import { formatCurrency } from "@/lib/utils";
import type { HoldingRow } from "./columns";

export function AggValueFooter({ table }: HeaderContext<HoldingRow, number>) {
	const [value, cost] = table
		.getFilteredRowModel()
		.rows.reduce(
			([value, cost], curr) => [
				value + (curr.original.eur.current.value ?? 0),
				cost + (curr.original.eur.all_time.cost ?? 0),
			],
			[0, 0],
		);
	return (
		<div className="flex flex-col place-items-end gap-1">
			<p className="font-medium text-base">{formatCurrency(value)}</p>
			<p className="font-normal text-muted-foreground text-sm">
				paid {formatCurrency(cost)}
			</p>
		</div>
	);
}
