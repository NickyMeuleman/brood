import type { CellContext } from "@tanstack/react-table";
import type { HoldingRow } from "./columns";
import { MoneyCell } from "./MoneyCell";

export function AggValueCell({ row }: CellContext<HoldingRow, number>) {
	return (
		<div className="flex flex-col place-items-end gap-1">
			<MoneyCell
				value={row.original.display.current.value}
				currency={row.original.display_currency}
				isConverted={row.original.is_converted}
				originalValue={row.original.local.current.value}
				originalCurrency={row.original.currency_code}
				className="font-medium text-base"
			/>
			<div className="flex gap-0.5 text-muted-foreground text-sm">
				<span>paid</span>
				<MoneyCell
					value={row.original.display.all_time.cost}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.local.all_time.cost}
					originalCurrency={row.original.currency_code}
					className="font-normal text-sm"
				/>
			</div>
		</div>
	);
}
