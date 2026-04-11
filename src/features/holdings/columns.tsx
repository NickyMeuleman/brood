import { createColumnHelper } from "@tanstack/react-table";
import { cn, formatCurrency, formatPercentage } from "@/lib/utils";
import { ActionsCell } from "./ActionsCell";
import { AggIdentityCell } from "./AggIdentityCell";
import { AggPerformanceCell } from "./AggPerformanceCell";
import { AggPerformanceFooter } from "./AggPerformanceFooter";
import { AggPriceCell } from "./AggPriceCell";
import { AggValueCell } from "./AggValueCell";
import { AggValueFooter } from "./AggValueFooter";
import { AggWeightCell } from "./AggWeightCell";
import type { NormalizedHolding } from "./lib";
import { MoneyCell } from "./MoneyCell";
import { SortableHeaderButton } from "./SortableHeaderButton";

export type HoldingRow = NormalizedHolding;
const columnHelper = createColumnHelper<HoldingRow>();

const aggColumns = [
	columnHelper.accessor((row) => row.ticker, {
		id: "agg_identity",
		meta: { label: "Listing", cellClassName: "w-1/2" },
		header: SortableHeaderButton,
		cell: AggIdentityCell,
	}),
	columnHelper.accessor((row) => row.display.current.quantity, {
		id: "agg_weight",
		meta: { label: "Size", align: "end" },
		header: SortableHeaderButton,
		cell: AggWeightCell,
	}),
	columnHelper.accessor((row) => row.display.current.unit_price, {
		id: "agg_price",
		meta: { label: "Price", align: "end" },
		header: SortableHeaderButton,
		cell: AggPriceCell,
	}),
	columnHelper.accessor((row) => row.display.current.value, {
		id: "agg_value",
		meta: { label: "Value", align: "end" },
		header: SortableHeaderButton,
		cell: AggValueCell,
		footer: AggValueFooter,
	}),
	columnHelper.accessor((row) => row.display.period.gain, {
		id: "agg_performance",
		meta: { label: "Performance", showPeriod: true, align: "end" },
		header: SortableHeaderButton,
		cell: AggPerformanceCell,
		footer: AggPerformanceFooter,
	}),
];

const snapshotColumns = [
	columnHelper.accessor("ticker", {
		meta: { label: "Ticker", hideByDefault: true },
		header: SortableHeaderButton,
		cell: ({ getValue, row }) => (
			<div className="flex items-baseline gap-0.5 font-medium">
				<span>{getValue()}</span>
				{row.original.currency_code !== "EUR" && (
					<span className="text-muted-foreground text-xs">
						{row.original.currency_code}
					</span>
				)}
			</div>
		),
	}),
	columnHelper.accessor("name", {
		meta: { label: "Name", hideByDefault: true },
		header: SortableHeaderButton,
	}),
	columnHelper.accessor((row) => row.display.current.value, {
		id: "weight",
		meta: { label: "Weight", align: "end", hideByDefault: true },
		header: SortableHeaderButton,
		cell: ({ row, table }) => {
			const total = Number(table.options.meta?.totals?.value ?? 1);
			const val = row.original.eur.current.value / total;
			const formatted = formatPercentage(val);
			return <div className="text-right font-medium">{formatted}</div>;
		},
	}),
	columnHelper.accessor((row) => row.display.current.quantity, {
		id: "quantity",
		meta: { label: "Quantity", align: "end", hideByDefault: true },
		header: SortableHeaderButton,
		cell: ({ getValue }) => {
			const val = getValue();
			return <div className="text-right font-medium">{val}</div>;
		},
	}),
	columnHelper.accessor((row) => row.display.current.unit_price_basis, {
		id: "unit_price_basis",
		meta: { label: "Cost Basis", align: "end", hideByDefault: true },
		header: SortableHeaderButton,
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display.current.unit_price_basis}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.local.current.unit_price_basis}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
	}),
	columnHelper.accessor((row) => row.display.current.unit_price, {
		id: "unit_price",
		meta: { label: "Price", align: "end", hideByDefault: true },
		header: SortableHeaderButton,
		cell: ({ row }) => {
			return (
				<MoneyCell
					value={row.original.display.current.unit_price}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.local.current.unit_price}
					originalCurrency={row.original.currency_code}
				/>
			);
		},
	}),
	columnHelper.accessor((row) => row.display.current.value, {
		id: "value",
		meta: { label: "Total", align: "end", hideByDefault: true },
		header: SortableHeaderButton,
		cell: ({ row }) => (
			<MoneyCell
				value={row.original.display.current.value}
				currency={row.original.display_currency}
				isConverted={row.original.is_converted}
				originalValue={row.original.local.current.value}
				originalCurrency={row.original.currency_code}
			/>
		),
		footer: ({ table }) => {
			const val = table
				.getFilteredRowModel()
				.rows.reduce((acc, curr) => acc + curr.original.eur.current.value, 0);
			const formatted = formatCurrency(val, "EUR");
			return <div className="text-right font-bold">{formatted}</div>;
		},
	}),
];

function buildTimeFrameCols(isPeriod: boolean) {
	const tfKey = isPeriod ? "period" : "all_time";
	const idSuffix = isPeriod ? "_period" : "_all_time";

	return [
		columnHelper.accessor((row) => row.display[tfKey].cost, {
			id: `paid${idSuffix}`,
			meta: {
				label: "Paid",
				align: "end",
				hideByDefault: true,
				showPeriod: isPeriod,
			},
			header: SortableHeaderButton,
			cell: ({ row }) => {
				return (
					<MoneyCell
						value={row.original.display[tfKey].cost}
						currency={row.original.display_currency}
						isConverted={row.original.is_converted}
						originalValue={row.original.local[tfKey].cost}
						originalCurrency={row.original.currency_code}
					/>
				);
			},
			footer: ({ table }) => {
				const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
					const val = curr.original.eur[tfKey].cost;
					return acc + val;
				}, 0);
				const formatted = formatCurrency(total, "EUR");
				return <div className={cn("text-right font-bold")}>{formatted}</div>;
			},
		}),
		columnHelper.accessor((row) => row.display[tfKey].fees, {
			id: `fees${idSuffix}`,
			meta: {
				label: "Fees",
				align: "end",
				hideByDefault: true,
				showPeriod: isPeriod,
			},
			header: SortableHeaderButton,
			cell: ({ row }) => (
				<MoneyCell
					value={row.original.display[tfKey].fees}
					currency={row.original.display_currency}
					isConverted={row.original.is_converted}
					originalValue={row.original.local[tfKey].fees}
					originalCurrency={row.original.currency_code}
				/>
			),
			footer: ({ table }) => {
				const total = table
					.getFilteredRowModel()
					.rows.reduce((acc, r) => acc + r.original.eur[tfKey].fees, 0);
				return (
					<div className="text-right font-bold">{formatCurrency(total)}</div>
				);
			},
		}),
		columnHelper.accessor((row) => row.display[tfKey].fee_drag, {
			id: `fee_drag${idSuffix}`,
			meta: {
				label: "Fee Drag",
				align: "end",
				hideByDefault: true,
				showPeriod: isPeriod,
			},
			header: SortableHeaderButton,
			// Fee drag is always relative to total_cost (without fees)
			// using total_cost_with_fees as denominator would be circular.
			// No need for a fees toggle since it's always showing the fee picture.
			cell: ({ getValue }) => (
				<div className="text-right font-medium">
					{formatPercentage(getValue())}
				</div>
			),
		}),
		columnHelper.accessor((row) => row.display[tfKey].gain, {
			id: `unrealised_gain${idSuffix}`,
			meta: {
				label: "Profit/Loss",
				align: "end",
				hideByDefault: true,
				showPeriod: isPeriod,
			},
			header: SortableHeaderButton,
			cell: ({ row }) => {
				return (
					<MoneyCell
						value={row.original.display[tfKey].gain}
						currency={row.original.display_currency}
						isConverted={row.original.is_converted}
						originalValue={row.original.local[tfKey].gain}
						originalCurrency={row.original.currency_code}
						className={cn(
							row.original.display.all_time.gain >= 0
								? "text-emerald-600"
								: "text-rose-600",
						)}
					/>
				);
			},
			footer: ({ table }) => {
				const total = table.getFilteredRowModel().rows.reduce((acc, curr) => {
					const val = curr.original.eur[tfKey].gain;
					return acc + val;
				}, 0);
				const formatted = formatCurrency(total, "EUR");
				return (
					<div
						className={cn(
							"text-right font-bold",
							total > 0 ? "text-emerald-600" : "text-rose-600",
						)}
					>
						{formatted}
					</div>
				);
			},
		}),
		// % of bought value to add to bought value to get market_value
		columnHelper.accessor((row) => row.display[tfKey].pct_gain, {
			id: `percentage_gain${idSuffix}`,
			meta: {
				label: "% Gain",
				align: "end",
				hideByDefault: true,
				showPeriod: isPeriod,
			},
			header: SortableHeaderButton,
			cell: ({ getValue }) => {
				const val = getValue();
				const formatted = formatPercentage(val);
				return (
					<div
						className={cn(
							"flex items-center justify-end font-medium",
							val > 0 ? "text-emerald-600" : "text-rose-600",
						)}
					>
						{formatted}
					</div>
				);
			},
			footer: ({ table }) => {
				const rows = table.getFilteredRowModel().rows;
				const gain = rows.reduce(
					(acc, r) => acc + r.original.eur[tfKey].gain,
					0,
				);
				const cost = rows.reduce(
					(acc, r) => acc + r.original.eur[tfKey].cost,
					0,
				);
				const percentage = cost !== 0 ? gain / cost : 0;
				const formatted = formatPercentage(percentage);
				return (
					<div
						className={cn(
							"text-right font-bold",
							percentage > 0 ? "text-emerald-600" : "text-rose-600",
						)}
					>
						{formatted}
					</div>
				);
			},
		}),
	];
}

const periodColumns = buildTimeFrameCols(true);
const allTimeColumns = buildTimeFrameCols(false);
const timeFrameColumns = periodColumns.map((col, i) => [
	col,
	allTimeColumns[i],
]);

export const columns = [
	...aggColumns,
	...snapshotColumns,
	...timeFrameColumns.flat(),
	columnHelper.display({
		id: "actions",
		meta: { cellClassName: "w-2" },
		cell: ActionsCell,
	}),
	// https://github.com/TanStack/table/issues/4382
];
