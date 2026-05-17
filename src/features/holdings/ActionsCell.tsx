import type { CellContext } from "@tanstack/react-table";
import {
	AlertCircle,
	CheckCircle2,
	MoreHorizontal,
	RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	useForceUpdateOneCurrencyFx,
	useForceUpdateOneListingPrices,
} from "@/hooks/use-force-sync";
import type { HoldingRow } from "./columns";

export function ActionsCell({ row }: CellContext<HoldingRow, unknown>) {
	const { listing_id, exchange_mic, ticker } = row.original;
	const syncPrices = useForceUpdateOneListingPrices();
	const syncFx = useForceUpdateOneCurrencyFx();
	const isBusy = syncPrices.isPending || syncFx.isPending;

	return (
		<div>
			<DropdownMenu>
				<DropdownMenuTrigger
					render={
						<Button variant="ghost" className="h-8 w-8 p-0" disabled={isBusy} />
					}
				>
					<span className="sr-only">Open menu</span>
					<MoreHorizontal className="h-4 w-4" />
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end">
					<DropdownMenuGroup>
						<DropdownMenuItem
							onClick={() => {
								navigator.clipboard.writeText(row.original.isin);
							}}
						>
							Copy ISIN
						</DropdownMenuItem>
					</DropdownMenuGroup>

					<DropdownMenuSeparator />

					<DropdownMenuGroup>
						<DropdownMenuLabel>Admin</DropdownMenuLabel>
						<DropdownMenuItem
							onClick={async () => {
								syncPrices.mutate({ listing_id, exchange_mic, ticker });
							}}
						>
							Force update price history
						</DropdownMenuItem>
						{row.original.currency_code !== "EUR" && (
							<DropdownMenuItem
								disabled={isBusy}
								onClick={() => {
									syncFx.mutate(row.original.currency_code);
								}}
							>
								Force update {row.original.currency_code} FX rates
							</DropdownMenuItem>
						)}
					</DropdownMenuGroup>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
