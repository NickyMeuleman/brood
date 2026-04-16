import type { CellContext } from "@tanstack/react-table";
import { Badge } from "@/components/ui/badge";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import {
	cn,
	formatPercentage,
	getCurrencySymbol,
	MIC_LABEL,
} from "@/lib/utils";
import type { HoldingRow } from "./columns";
import { TruncatedTooltip } from "./TruncatedTooltip";

export function AggIdentityCell({
	table,
	row,
}: CellContext<HoldingRow, string>) {
	const total = Number(table.options.meta?.totals?.value ?? 1);
	const weight = row.original.eur.current.value / total;
	const angle = weight * 360;
	const image_url = false;
	const exchangeLabel =
		MIC_LABEL[row.original.exchange_mic] ?? row.original.exchange_mic;
	const currencySymbol = getCurrencySymbol(row.original.currency_code);
	const isForeignCurrency = row.original.currency_code !== "EUR";

	return (
		<div className="flex w-full items-center gap-3">
			<Tooltip>
				<TooltipTrigger
					render={
						<div
							className="flex h-14 w-14 shrink-0 items-center justify-center rounded-full"
							style={{
								background: `conic-gradient(var(--ring) ${angle}deg, var(--border) ${angle}deg)`,
							}}
						>
							<div className="flex h-12 w-12 items-center justify-center overflow-hidden rounded-full bg-background">
								{image_url ? (
									<img
										src={image_url}
										alt={row.original.ticker}
										className="h-full w-full object-cover"
									/>
								) : (
									<span className="font-medium font-mono tracking-wider">
										{row.original.ticker}
									</span>
								)}
							</div>
						</div>
					}
				/>
				<TooltipContent>
					<div className="flex flex-col gap-0.5">
						<span className="text-center font-bold text-muted text-xs underline">
							Weight
						</span>
						<span className="font-mono text-base">
							{formatPercentage(weight)}
						</span>
					</div>
				</TooltipContent>
			</Tooltip>
			<div className="flex min-w-0 flex-1 flex-col gap-0.5">
				<TruncatedTooltip>{row.original.name}</TruncatedTooltip>
				<div className="flex items-center gap-1.5 whitespace-nowrap font-normal text-muted-foreground text-sm">
					<Badge
						variant="ghost"
						className={cn(
							"bg-secondary font-mono text-secondary-foreground text-sm tracking-wider",
						)}
					>
						{row.original.ticker}
					</Badge>
					<span className="opacity-60">·</span>
					<span>{exchangeLabel}</span>
					{isForeignCurrency && (
						<>
							<span className="opacity-60">·</span>
							<span>{currencySymbol}</span>
						</>
					)}
				</div>
			</div>
		</div>
	);
}
