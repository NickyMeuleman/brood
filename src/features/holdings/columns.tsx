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

const columnHelper = createColumnHelper<Holding>();

export function createColumns(includeFees: boolean): ColumnDef<Holding>[] {
	const accessorFuncs = includeFees
		? {
				costBasis: (row: Holding) => Number(row.avg_cost_basis_with_fees),
				paid: (row: Holding) => Number(row.total_cost_with_fees),
				unrealisedGain: (row: Holding) => Number(row.unrealised_gain_with_fees),
				unrealisedGainEur: (row: Holding) =>
					Number(row.unrealised_gain_with_fees_eur),
				totalCostEur: (row: Holding) => Number(row.total_cost_with_fees_eur),
				percentageGain: (row: Holding) => Number(row.percentage_gain_with_fees),
			}
		: {
				costBasis: (row: Holding) => Number(row.avg_cost_basis),
				paid: (row: Holding) => Number(row.total_cost),
				unrealisedGain: (row: Holding) => Number(row.unrealised_gain),
				unrealisedGainEur: (row: Holding) => Number(row.unrealised_gain_eur),
				totalCostEur: (row: Holding) => Number(row.total_cost_eur),
				percentageGain: (row: Holding) => Number(row.percentage_gain),
			};
	return [
		columnHelper.accessor("ticker", {
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Ticker" />
			),
		}),
		columnHelper.accessor("name", {
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Name" />
			),
		}),
		columnHelper.accessor((row) => Number(row.market_value), {
			id: "portfolio_weight",
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Weight" align="end" />
			),
			cell: ({ row, table }) => {
				const total = Number(table.options.meta?.totals?.market_value_eur ?? 1);
				const val = Number(row.original.market_value_eur) / total;
				const formatted = formatPercentage(val);
				return <div className="text-right font-medium">{formatted}</div>;
			},
		}),
		columnHelper.accessor("quantity", {
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Quantity" align="end" />
			),
			cell: ({ getValue }) => {
				const quantity = getValue();
				return <div className="text-right font-medium">{quantity}</div>;
			},
		}),
		columnHelper.accessor(accessorFuncs.costBasis, {
			id: "cost_basis",
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Cost basis" align="end" />
			),
			cell: ({ row, getValue }) => {
				const val = getValue();
				const formatted = formatCurrency(val, row.original.currency_code);
				return <div className="text-right font-medium">{formatted}</div>;
			},
		}),
		columnHelper.accessor(accessorFuncs.paid, {
			id: "paid",
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Paid" align="end" />
			),
			cell: ({ row, getValue }) => {
				const val = getValue();
				const formatted = formatCurrency(val, row.original.currency_code);
				return <div className="text-right font-medium">{formatted}</div>;
			},
			footer: ({ table }) => {
				const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
					const val = accessorFuncs.totalCostEur(curr.original);
					return acc + val;
				}, 0);
				const formatted = formatCurrency(total, "EUR");
				return <div className={cn("text-right font-bold")}>{formatted}</div>;
			},
		}),
		columnHelper.accessor("current_price", {
			header: ({ column }) => (
				<SortableHeaderButton
					column={column}
					label="Current Price"
					align="end"
				/>
			),
			cell: ({ row, getValue }) => {
				const val = Number(getValue());
				const formatted = formatCurrency(val, row.original.currency_code);
				return <div className="text-right font-medium">{formatted}</div>;
			},
		}),
		columnHelper.accessor("market_value", {
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Total" align="end" />
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
		columnHelper.accessor(accessorFuncs.unrealisedGain, {
			id: "unrealised_gain",
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="Profit/Loss" align="end" />
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
					const val = includeFees
						? curr.original.unrealised_gain_with_fees_eur
						: curr.original.unrealised_gain_eur;
					return acc + Number(val);
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
		columnHelper.accessor(accessorFuncs.percentageGain, {
			id: "percentage_gain",
			header: ({ column }) => (
				<SortableHeaderButton column={column} label="%" align="end" />
			),
			cell: ({ getValue }) => {
				const val = Number(getValue());
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
					(acc, r) => acc + accessorFuncs.unrealisedGainEur(r.original),
					0,
				);
				const cost = rows.reduce(
					(acc, r) => acc + accessorFuncs.totalCostEur(r.original),
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
}
