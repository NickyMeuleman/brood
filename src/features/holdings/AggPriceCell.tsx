import type { CellContext } from "@tanstack/react-table";
import type { HoldingRow } from "./columns";
import { MoneyCell } from "./MoneyCell";

export function AggPriceCell({ row }: CellContext<HoldingRow, number>) {
	return (
		<div className="flex flex-col place-items-end gap-1">
			<MoneyCell
				value={row.original.display.current.unit_price}
				currency={row.original.display_currency}
				isConverted={row.original.is_converted}
				originalValue={row.original.local.current.unit_price}
				originalCurrency={row.original.currency_code}
				missingLabel="Waiting for first price"
				className="font-medium text-base"
			/>
			<div className="flex gap-0.5 text-muted-foreground text-sm">
				<span>avg.</span>
				<MoneyCell
					value={row.original.display.current.unit_price_basis}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.local.current.unit_price_basis}
					originalCurrency={row.original.currency_code}
					className="font-normal text-sm"
				/>
			</div>
		</div>
	);
}
