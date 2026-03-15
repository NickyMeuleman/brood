import { type ColumnDef, createColumnHelper } from "@tanstack/react-table";
import {
	ArrowDownRight,
	ArrowLeftRight,
	ArrowUpRight,
	MoreHorizontal,
} from "lucide-react";
import type { Holding } from "@/bindings";
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
import { cn, formatCurrency, formatPercentage } from "@/lib/utils";
import { SortableHeaderButton } from "./SortableHeaderButton";

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
	h: Holding,
	includeFees: boolean,
	displayInEur: boolean,
) {
	const is_converted = displayInEur && h.currency_code !== "EUR";
	const display_currency = is_converted ? "EUR" : h.currency_code;

	// Fee-selected local-currency values
	const avg_cost_local = includeFees
		? h.avg_cost_basis_with_fees
		: h.avg_cost_basis;
	const total_cost_local = includeFees ? h.total_cost_with_fees : h.total_cost;
	const unrealised_local = includeFees
		? h.unrealised_gain_with_fees
		: h.unrealised_gain;
	const percentage_gain = includeFees
		? h.percentage_gain_with_fees
		: h.percentage_gain;

	// Fee-selected EUR values
	const avg_cost_eur = includeFees
		? h.avg_cost_basis_with_fees_eur
		: h.avg_cost_basis_eur;
	const total_cost_eur = includeFees
		? h.total_cost_with_fees_eur
		: h.total_cost_eur;
	const unrealised_eur = includeFees
		? h.unrealised_gain_with_fees_eur
		: h.unrealised_gain_eur;

	// current_price_eur: exact — inverts market_value_eur = price × qty × rate
	const qty = Number(h.quantity);
	const current_price_eur = qty > 0 ? Number(h.market_value_eur) / qty : 0;

	return {
		...h,
		is_converted,
		display_currency,

		// Resolved display values (what cells render and sort on)
		display_avg_cost_basis: is_converted
			? Number(avg_cost_eur)
			: Number(avg_cost_local),
		display_total_cost: is_converted
			? Number(total_cost_eur)
			: Number(total_cost_local),
		display_unrealised_gain: is_converted
			? Number(unrealised_eur)
			: Number(unrealised_local),
		display_current_price: is_converted
			? current_price_eur
			: Number(h.current_price),
		display_market_value: is_converted
			? Number(h.market_value_eur)
			: Number(h.market_value),
		display_total_fees: is_converted
			? Number(h.total_fees_eur)
			: Number(h.total_fees),
		display_percentage_gain: Number(percentage_gain),

		// Originals (only meaningful when is_converted = true)
		display_avg_cost_basis_original: Number(avg_cost_local),
		display_total_cost_original: Number(total_cost_local),
		display_unrealised_gain_original: Number(unrealised_local),
		display_current_price_original: Number(h.current_price),
		display_market_value_original: Number(h.market_value),
		display_total_fees_original: Number(h.total_fees),

		// EUR values used by footers (always EUR regardless of displayInEur)
		footer_total_cost_eur: Number(total_cost_eur),
		footer_unrealised_gain_eur: Number(unrealised_eur),
		footer_total_fees_eur: Number(h.total_fees_eur),
		footer_market_value_eur: Number(h.market_value_eur),
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
						<ArrowLeftRight className="h-3 w-3 shrink-0 text-muted-foreground/50" />
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
	columnHelper.accessor("ticker", {
		meta: { label: "Ticker" },
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
	columnHelper.accessor((row) => Number(row.market_value), {
		id: "portfolio_weight",
		meta: { label: "Weight" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, table }) => {
			const total = Number(table.options.meta?.totals?.market_value_eur ?? 1);
			const val = Number(row.original.market_value_eur) / total;
			const formatted = formatPercentage(val);
			return <div className="text-right font-medium">{formatted}</div>;
		},
	}),
	columnHelper.accessor("quantity", {
		meta: { label: "Quantity" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ getValue }) => {
			const val = getValue();
			return <div className="text-right font-medium">{val}</div>;
		},
	}),
	columnHelper.accessor((row) => Number(row.display_avg_cost_basis), {
		id: "cost_basis",
		meta: { label: "Cost Basis" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_avg_cost_basis}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_avg_cost_basis_original}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
	}),
	columnHelper.accessor((row) => Number(row.display_total_cost), {
		id: "paid",
		meta: { label: "Paid" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_total_cost}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_total_cost_original}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
		footer: ({ table }) => {
			const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
				const val = curr.original.footer_total_cost_eur;
				return acc + val;
			}, 0);
			const formatted = formatCurrency(total, "EUR");
			return <div className={cn("text-right font-bold")}>{formatted}</div>;
		},
	}),
	columnHelper.accessor("current_price", {
		meta: { label: "Price" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display_current_price}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.display_current_price_original}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
	}),
	columnHelper.accessor((row) => Number(row.total_fees), {
		id: "total_fees",
		meta: { label: "Fees" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row }) => (
			<MoneyCell
				value={row.original.display_total_fees}
				currency={row.original.display_currency}
				isConverted={row.original.is_converted}
				originalValue={row.original.display_total_fees_original}
				originalCurrency={row.original.currency_code}
			/>
		),
		footer: ({ table }) => {
			const total = table
				.getFilteredRowModel()
				.rows.reduce((acc, r) => acc + r.original.footer_total_fees_eur, 0);
			return (
				<div className="text-right font-bold">{formatCurrency(total)}</div>
			);
		},
	}),
	columnHelper.accessor((row) => Number(row.fee_drag), {
		id: "fee_drag",
		meta: { label: "Fee Drag" },
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
	columnHelper.accessor("market_value", {
		meta: { label: "Total" },
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
	columnHelper.accessor((row) => row.display_unrealised_gain, {
		id: "unrealised_gain",
		meta: { label: "Profit/Loss" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, getValue }) => {
			// const isPositive = row.original.display_unrealised_gain >= 0;
			// const val = Number(getValue());
			// const formatted = formatCurrency(val, row.original.currency_code);
			// return (
			// 	<div
			// 		className={cn(
			// 			"flex items-center justify-end font-medium",
			// 			val > 0 ? "text-green-600" : "text-red-600",
			// 		)}
			// 	>
			// 		<span>{val > 0 ? <ArrowUpRight /> : <ArrowDownRight />}</span>
			// 		<span>{formatted}</span>
			// 	</div>
			// );
			return "placehold";
		},
		footer: ({ table }) => {
			const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
				const val = curr.original.footer_unrealised_gain_eur;
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
	columnHelper.accessor((row) => row.display_percentage_gain, {
		id: "percentage_gain",
		meta: { label: "% Gain" },
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
					<span>{formatted}</span>
				</div>
			);
		},
		footer: ({ table }) => {
			const rows = table.getFilteredRowModel().rows;
			const gain = rows.reduce(
				(acc, r) => acc + r.original.footer_unrealised_gain_eur,
				0,
			);
			const cost = rows.reduce(
				(acc, r) => acc + r.original.footer_total_cost_eur,
				0,
			);
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
