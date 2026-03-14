import { type ColumnDef, createColumnHelper } from "@tanstack/react-table";
import { ArrowDownRight, ArrowUpRight, MoreHorizontal } from "lucide-react";
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
import { cn, formatCurrency, formatPercentage } from "@/lib/utils";
import { SortableHeaderButton } from "./SortableHeaderButton";

export function normalizeHolding(h: Holding, includeFees: boolean) {
	return {
		...h,
		display_avg_cost_basis: includeFees
			? h.avg_cost_basis_with_fees
			: h.avg_cost_basis,
		display_total_cost: includeFees ? h.total_cost_with_fees : h.total_cost,
		display_unrealised_gain: includeFees
			? h.unrealised_gain_with_fees
			: h.unrealised_gain,
		display_unrealised_gain_eur: includeFees
			? h.unrealised_gain_with_fees_eur
			: h.unrealised_gain_eur,
		display_percentage_gain: includeFees
			? h.percentage_gain_with_fees
			: h.percentage_gain,
		display_total_cost_eur: includeFees
			? h.total_cost_with_fees_eur
			: h.total_cost_eur,
	};
}

type NormalizedHolding = ReturnType<typeof normalizeHolding>;

const columnHelper = createColumnHelper<NormalizedHolding>();

export const columns: ColumnDef<NormalizedHolding>[] = [
	columnHelper.accessor("ticker", {
		meta: { label: "Ticker" },
		header: ({ column }) => <SortableHeaderButton column={column} />,
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
			const quantity = getValue();
			return <div className="text-right font-medium">{quantity}</div>;
		},
	}),
	columnHelper.accessor((row) => Number(row.display_avg_cost_basis), {
		id: "cost_basis",
		meta: { label: "Cost Basis" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, getValue }) => {
			const val = getValue();
			const formatted = formatCurrency(val, row.original.currency_code);
			return <div className="text-right font-medium">{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => Number(row.display_total_cost), {
		id: "paid",
		meta: { label: "Paid" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, getValue }) => {
			const val = getValue();
			const formatted = formatCurrency(val, row.original.currency_code);
			return <div className="text-right font-medium">{formatted}</div>;
		},
		footer: ({ table }) => {
			const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
				const val = Number(curr.original.display_total_cost_eur);
				return acc + val;
			}, 0);
			const formatted = formatCurrency(total, "EUR");
			return <div className={cn("text-right font-bold")}>{formatted}</div>;
		},
	}),
	columnHelper.accessor("current_price", {
		meta: { label: "Current Price" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, getValue }) => {
			const val = Number(getValue());
			const formatted = formatCurrency(val, row.original.currency_code);
			return <div className="text-right font-medium">{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => Number(row.total_fees), {
		id: "total_fees",
		meta: { label: "Fees" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, getValue }) => (
			<div className="text-right font-medium">
				{formatCurrency(getValue(), row.original.currency_code)}
			</div>
		),
		footer: ({ table }) => {
			const total = table
				.getFilteredRowModel()
				.rows.reduce((acc, r) => acc + Number(r.original.total_fees_eur), 0);
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
		cell: ({ row }) => {
			const val = parseFloat(row.getValue("market_value"));
			const formatted = formatCurrency(val, row.original.currency_code);
			return <div className="text-right font-medium">{formatted}</div>;
		},
		footer: ({ table }) => {
			const val = table
				.getFilteredRowModel()
				.rows.reduce(
					(acc, curr) => acc + Number(curr.original.market_value_eur),
					0,
				);
			const formatted = formatCurrency(val, "EUR");
			return <div className="text-right font-bold">{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => Number(row.display_unrealised_gain), {
		id: "unrealised_gain",
		meta: { label: "Profit/Loss" },
		header: ({ column }) => (
			<SortableHeaderButton column={column} align="end" />
		),
		cell: ({ row, getValue }) => {
			const val = Number(getValue());
			const formatted = formatCurrency(val, row.original.currency_code);
			return (
				<div
					className={cn(
						"flex items-center justify-end font-medium",
						val > 0 ? "text-green-600" : "text-red-600",
					)}
				>
					<span>{val > 0 ? <ArrowUpRight /> : <ArrowDownRight />}</span>
					<span>{formatted}</span>
				</div>
			);
		},
		footer: ({ table }) => {
			const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
				const val = Number(curr.original.display_unrealised_gain_eur);
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
	columnHelper.accessor((row) => Number(row.display_percentage_gain), {
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
				(acc, r) => acc + Number(r.original.display_unrealised_gain_eur),
				0,
			);
			const cost = rows.reduce(
				(acc, r) => acc + Number(r.original.display_total_cost_eur),
				0,
			);
			const percentage = gain / cost;
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
] as ColumnDef<Holding>[];
