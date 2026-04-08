import { type ColumnDef, createColumnHelper } from "@tanstack/react-table";
import {
	ArrowDownRight,
	ArrowLeftRight,
	ArrowUpRight,
	MoreHorizontal,
} from "lucide-react";
import type { EnvelopeHolding } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import {
	cn,
	formatCurrency,
	formatPercentage,
	getCurrencySymbol,
	MIC_LABEL,
} from "@/lib/utils";
import { SortableHeaderButton } from "./SortableHeaderButton";
import { TruncatedTooltip } from "./TruncatedTooltip";

// ---------------------------------------------------------------------------
// normalizeHolding
//
// Single source of truth for "what does each cell display?"
// Both toggles are resolved here so cells and accessor functions are fully
// dumb — they only read display_* fields and never branch on external state.
//
// This matters for sorting: the accessor reads the same field the cell
// renders, so sorting always operates on the displayed value.
//
// For converted values (displayInEur && currency !== EUR), the original
// local-currency value is preserved in display_*_original so the tooltip
// can show it without knowing anything about toggle state.
//
// Footer totals always aggregate in EUR regardless of displayInEur, because
// cross-currency summation requires a common unit.
// ---------------------------------------------------------------------------
export function normalizeHolding(
	h: EnvelopeHolding,
	includeFees: boolean,
	displayInEur: boolean,
) {
	const is_converted = displayInEur && h.currency_code !== "EUR";
	const display_currency = is_converted ? "EUR" : h.currency_code;

	// Resolves a CurrencyPair to the displayed currency
	const cur = <T,>(pair: { local: T; eur: T }): T =>
		is_converted ? pair.eur : pair.local;

	// Resolves NetGross based on the fees toggle
	const pick = <T,>(ng: { gross: T; net: T }): T =>
		includeFees ? ng.net : ng.gross;

	const atLocal = pick(h.all_time.metrics.local.metrics);
	const atEur = pick(h.all_time.metrics.eur.metrics);
	const basis = pick(h.current.unit_price_basis);

	// Originals for tooltip (only meaningful when is_converted = true)
	return {
		...h,
		is_converted,
		display_currency,

		// Current state
		display_unit_price: Number(cur(h.current.unit_price)),
		display_unit_price_original: Number(h.current.unit_price.local),
		display_market_value: Number(cur(h.current.value)),
		display_market_value_original: Number(h.current.value.local),
		display_unit_price_basis: Number(cur(basis)),
		display_unit_price_basis_original: Number(basis.local),

		// All-time perf
		display_cost: Number(is_converted ? atEur.cost : atLocal.cost),
		display_cost_original: Number(atLocal.cost),
		display_gain: Number(is_converted ? atEur.gain : atLocal.gain),
		display_gain_original: Number(atLocal.gain),
		display_pct_gain: Number(is_converted ? atEur.pct_gain : atLocal.pct_gain),

		// Fees - not affected by pick(), fees are the same on both gross/net
		// includeFees controls whether fees are baked into cost/gain, not whether they're shown here
		display_fees: Number(
			is_converted
				? h.all_time.metrics.eur.fees
				: h.all_time.metrics.local.fees,
		),
		display_fees_original: Number(h.all_time.metrics.local.fees),
		// always EUR, always all-time
		display_fee_drag: Number(h.all_time.metrics.eur.pct_fees),

		// Footer - always EUR
		footer_market_value_eur: Number(h.current.value.eur),
		footer_cost_eur: Number(atEur.cost),
		footer_gain_eur: Number(atEur.gain),
		footer_fees_eur: Number(h.all_time.metrics.eur.fees),
	};
}

type NormalizedHolding = ReturnType<typeof normalizeHolding>;

// ---------------------------------------------------------------------------
// MoneyCell — dumb display component.
// Receives pre-resolved values from row.original; knows nothing about toggles.
// ---------------------------------------------------------------------------
function MoneyCell({
	value,
	currency,
	isConverted,
	originalValue,
	originalCurrency,
	className,
}: {
	value: number;
	currency: string;
	isConverted: boolean;
	originalValue?: number;
	originalCurrency?: string;
	className?: string;
}) {
	if (!isConverted) {
		return (
			<div className={cn("text-right font-medium", className)}>
				{formatCurrency(value, currency)}
			</div>
		);
	}

	return (
		<Tooltip>
			<TooltipTrigger
				render={
					<div
						className={cn(
							"flex cursor-default items-center justify-end gap-1 font-medium",
							className,
						)}
					>
						<ArrowLeftRight className="inline h-3 w-3 shrink-0 text-muted-foreground/50" />
						<span className="underline decoration-muted-foreground/40 decoration-dashed underline-offset-2">
							{formatCurrency(value, currency)}
						</span>
					</div>
				}
			/>
			<TooltipContent side="top" className="text-xs">
				<p className="font-medium">
					{formatCurrency(originalValue ?? 0, originalCurrency ?? "EUR")}
					<span className="text-muted-foreground"> in {originalCurrency}</span>
				</p>
			</TooltipContent>
		</Tooltip>
	);
}

const columnHelper = createColumnHelper<NormalizedHolding>();

export const columns: ColumnDef<NormalizedHolding>[] = [
	columnHelper.accessor((row) => row.ticker, {
		id: "agg_identity",
		meta: { label: "Listing", cellClassName: "w-1/2" },
		header: ({ column }) => <SortableHeaderButton column={column} />,
		cell: ({ row, table }) => {
			const total = Number(table.options.meta?.totals?.value ?? 1);
			const weight = row.original.footer_market_value_eur / total;
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
					<div className="flex min-w-0 flex-1 flex-col gap-px">
						<TruncatedTooltip>{row.original.name}</TruncatedTooltip>
						<div className="flex items-center gap-1.5 whitespace-nowrap">
							<span className="font-mono text-muted-foreground text-sm">
								{row.original.ticker}
							</span>
							<span className="text-muted-foreground/40 text-xs">·</span>
							<Badge
								variant="outline"
								className="flex h-5 gap-0.5 rounded px-1 font-normal text-muted-foreground text-xs"
							>
								<span>{exchangeLabel}</span>
								{isForeignCurrency && (
									<>
										<span className="text-muted-foreground/40">·</span>
										<span>{currencySymbol}</span>
									</>
								)}
							</Badge>
						</div>
					</div>
				</div>
			);
		},
	}),
	columnHelper.accessor((row) => Number(row.current.quantity), {
		id: "agg_weight",
		meta: { label: "Size" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ getValue, row, table }) => {
			const total = Number(table.options.meta?.totals?.value ?? 1);
			const weight = row.original.footer_market_value_eur / total;
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
		},
	}),
	columnHelper.accessor((row) => row.display_unit_price, {
		id: "agg_price",
		meta: { label: "Price" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<div className="flex flex-col place-items-end gap-1">
					<MoneyCell
						value={row.original.display_unit_price}
						currency={row.original.display_currency}
						isConverted={row.original.is_converted}
						originalValue={row.original.display_unit_price_original}
						originalCurrency={row.original.currency_code}
						className="font-medium text-base"
					/>
					<div className="flex gap-0.5 text-muted-foreground text-sm">
						<span>avg.</span>
						<MoneyCell
							value={row.original.display_unit_price_basis}
							currency={row.original.display_currency}
							isConverted={row.original.is_converted}
							originalValue={row.original.display_unit_price_basis_original}
							originalCurrency={row.original.currency_code}
							className="font-normal text-sm"
						/>
					</div>
				</div>
			);
		},
	}),
	columnHelper.accessor((row) => row.display_market_value, {
		id: "agg_value",
		meta: { label: "Value" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<div className="flex flex-col place-items-end gap-1">
					<MoneyCell
						value={row.original.display_market_value}
						currency={row.original.display_currency}
						isConverted={row.original.is_converted}
						originalValue={row.original.display_market_value_original}
						originalCurrency={row.original.currency_code}
						className="font-medium text-base"
					/>
					<div className="flex gap-0.5 text-muted-foreground text-sm">
						<span>paid</span>
						<MoneyCell
							value={row.original.display_cost}
							currency={row.original.display_currency}
							isConverted={row.original.is_converted}
							originalValue={row.original.display_cost_original}
							originalCurrency={row.original.currency_code}
							className="font-normal text-sm"
						/>
					</div>
				</div>
			);
		},
		footer: ({ table }) => {
			const [market_val, cost] = table
				.getFilteredRowModel()
				.rows.reduce(
					([market_val, cost], curr) => [
						market_val + curr.original.footer_market_value_eur,
						cost + curr.original.footer_cost_eur,
					],
					[0, 0],
				);
			return (
				<div className="flex flex-col place-items-end gap-1">
					<p className="font-medium text-base">{formatCurrency(market_val)}</p>
					<p className="font-normal text-muted-foreground text-sm">
						paid {formatCurrency(cost)}
					</p>
				</div>
			);
		},
	}),
	columnHelper.accessor((row) => row.display_gain, {
		id: "agg_performance",
		meta: { label: "Performance" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			const isPos = row.original.display_gain > 0;

			return (
				<div
					className={cn(
						"flex place-content-end items-center gap-1",
						row.original.display_gain >= 0 ? "text-green-600" : "text-red-600",
					)}
				>
					{isPos ? <ArrowUpRight /> : <ArrowDownRight />}
					<div className="flex flex-col place-items-end gap-1">
						<MoneyCell
							value={row.original.display_gain}
							currency={row.original.display_currency}
							isConverted={row.original.is_converted}
							originalValue={row.original.display_gain_original}
							originalCurrency={row.original.currency_code}
							className="font-medium text-base"
						/>
						<p className="font-normal text-sm">
							{formatPercentage(row.original.display_pct_gain)}
						</p>
					</div>
				</div>
			);
		},
		footer: ({ table }) => {
			const [gain, cost] = table
				.getFilteredRowModel()
				.rows.reduce(
					([gain, cost], r) => [
						gain + r.original.footer_gain_eur,
						cost + r.original.footer_cost_eur,
					],
					[0, 0],
				);
			const percentage = cost !== 0 ? gain / cost : 0;

			return (
				<div
					className={cn(
						"flex place-content-end items-center gap-1",
						gain >= 0 ? "text-green-600" : "text-red-600",
					)}
				>
					{gain >= 0 ? <ArrowUpRight /> : <ArrowDownRight />}
					<div className="flex flex-col place-items-end gap-1">
						<p className="font-medium text-base">{formatCurrency(gain)}</p>
						<p className="font-normal text-sm">
							{formatPercentage(percentage)}
						</p>
					</div>
				</div>
			);
		},
	}),
	columnHelper.accessor("ticker", {
		meta: { label: "Ticker", hideByDefault: true },
		header: ({ column }) => <SortableHeaderButton column={column} />,
		cell: ({ getValue, row }) => (
			<div className="font-medium">
				{getValue()}
				{row.original.currency_code !== "EUR" && (
					<span className="ml-1.5 text-muted-foreground text-xs">
						{row.original.currency_code}
					</span>
				)}
			</div>
		),
	}),
	columnHelper.accessor("name", {
		meta: { label: "Name", hideByDefault: true },
		header: ({ column }) => <SortableHeaderButton column={column} />,
	}),
	columnHelper.accessor((row) => row.display_market_value, {
		id: "weight",
		meta: { label: "Weight", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, table }) => {
			const total = Number(table.options.meta?.totals?.market_value_eur ?? 1);
			const val = row.original.footer_market_value_eur / total;
			const formatted = formatPercentage(val);
			return <div className="text-right font-medium">{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => Number(row.current.quantity), {
		id: "quantity",
		meta: { label: "Quantity", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ getValue }) => {
			const val = getValue();
			return <div className="text-right font-medium">{val}</div>;
		},
	}),
	columnHelper.accessor((row) => row.display_unit_price_basis, {
		id: "unit_price_basis",
		meta: { label: "Cost Basis", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_unit_price_basis}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_unit_price_basis_original}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
	}),
	columnHelper.accessor((row) => row.display_cost, {
		id: "paid",
		meta: { label: "Paid", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_cost}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_cost_original}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
		footer: ({ table }) => {
			const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
				const val = curr.original.footer_cost_eur;
				return acc + val;
			}, 0);
			const formatted = formatCurrency(total, "EUR");
			return <div className={cn("text-right font-bold")}>{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => row.display_unit_price, {
		id: "unit_price",
		meta: { label: "Price", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_unit_price}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_unit_price_original}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
	}),
	columnHelper.accessor((row) => row.display_fees, {
		id: "total_fees",
		meta: { label: "Fees", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => (
			<MoneyCell
				value={row.original.display_fees}
				currency={row.original.display_currency}
				isConverted={row.original.is_converted}
				originalValue={row.original.display_fees_original}
				originalCurrency={row.original.currency_code}
			/>
		),
		footer: ({ table }) => {
			const total = table
				.getFilteredRowModel()
				.rows.reduce((acc, r) => acc + r.original.footer_fees_eur, 0);
			return (
				<div className="text-right font-bold">{formatCurrency(total)}</div>
			);
		},
	}),
	columnHelper.accessor((row) => row.display_fee_drag, {
		id: "fee_drag",
		meta: { label: "Fee Drag", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		// Fee drag is always relative to total_cost (without fees)
		// using total_cost_with_fees as denominator would be circular.
		// No need for a fees toggle since it's always showing the fee picture.
		cell: ({ getValue }) => (
			<div className="text-right font-medium">
				{formatPercentage(getValue())}
			</div>
		),
	}),
	columnHelper.accessor((row) => row.display_market_value, {
		id: "market_value",
		meta: { label: "Total", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => (
			<MoneyCell
				value={row.original.display_market_value}
				currency={row.original.display_currency}
				isConverted={row.original.is_converted}
				originalValue={row.original.display_market_value_original}
				originalCurrency={row.original.currency_code}
			/>
		),
		footer: ({ table }) => {
			const val = table
				.getFilteredRowModel()
				.rows.reduce(
					(acc, curr) => acc + curr.original.footer_market_value_eur,
					0,
				);
			const formatted = formatCurrency(val, "EUR");
			return <div className="text-right font-bold">{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => row.display_gain, {
		id: "unrealised_gain",
		meta: { label: "Profit/Loss", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_gain}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_gain_original}
					originalCurrency={row.original.currency_code}
					className={cn(
						row.original.display_gain >= 0 ? "text-green-600" : "text-red-600",
					)}
				/>
			);
		},
		footer: ({ table }) => {
			const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
				const val = curr.original.footer_gain_eur;
				return acc + val;
			}, 0);
			const formatted = formatCurrency(total, "EUR");
			return (
				<div
					className={cn(
						"text-right font-bold",
						total > 0 ? "text-green-600" : "text-red-600",
					)}
				>
					{formatted}
				</div>
			);
		},
	}),
	// % of bought value to add to bought value to get market_value
	columnHelper.accessor((row) => row.display_pct_gain, {
		id: "percentage_gain",
		meta: { label: "% Gain", hideByDefault: true },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ getValue }) => {
			const val = getValue();
			const formatted = formatPercentage(val);
			return (
				<div
					className={cn(
						"flex items-center justify-end font-medium",
						val > 0 ? "text-green-600" : "text-red-600",
					)}
				>
					{formatted}
				</div>
			);
		},
		footer: ({ table }) => {
			const rows = table.getFilteredRowModel().rows;
			const gain = rows.reduce((acc, r) => acc + r.original.footer_gain_eur, 0);
			const cost = rows.reduce((acc, r) => acc + r.original.footer_cost_eur, 0);
			const percentage = cost !== 0 ? gain / cost : 0;
			const formatted = formatPercentage(percentage);
			return (
				<div
					className={cn(
						"text-right font-bold",
						percentage > 0 ? "text-green-600" : "text-red-600",
					)}
				>
					{formatted}
				</div>
			);
		},
	}),
	columnHelper.display({
		id: "actions",
		meta: { cellClassName: "w-2" },
		cell: ({ row }) => {
			return (
				<DropdownMenu>
					<DropdownMenuTrigger
						render={<Button variant="ghost" className="h-8 w-8 p-0" />}
					>
						<span className="sr-only">Open menu</span>
						<MoreHorizontal className="h-4 w-4" />
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end">
						<DropdownMenuGroup>
							<DropdownMenuLabel>Actions</DropdownMenuLabel>
							<DropdownMenuItem
								onClick={() => {
									navigator.clipboard.writeText(row.original.isin);
								}}
							>
								Copy ISIN
							</DropdownMenuItem>
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>
			);
		},
	}),
	// https://github.com/TanStack/table/issues/4382
] as ColumnDef<NormalizedHolding>[];
